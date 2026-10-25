use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::warn;
use uuid::Uuid;

use crate::scrapers::runner::{run_exclusive, ScrapeLock};

type LastRun = Arc<Mutex<HashMap<Uuid, Instant>>>;

/// Cron a cada 5 min; cada loja só é varrida quando seu `scrape_interval_minutes` vence.
/// Primeira varredura ~10 s após o boot.
// shortcut: estado em memória, reinício re-varre todas as lojas. Persistir last_scraped_at em stores se isso virar problema.
pub async fn start(pool: PgPool, client: wreq::Client, lock: ScrapeLock) -> anyhow::Result<JobScheduler> {
    let last: LastRun = Arc::default();
    let tick = move || {
        let (pool, client, lock, last) = (pool.clone(), client.clone(), lock.clone(), last.clone());
        async move {
            if let Err(e) = run_due(&pool, &client, &lock, &last).await {
                warn!("scheduler: {e:#}");
            }
        }
    };

    let sched = JobScheduler::new().await?;
    let cron_tick = tick.clone();
    sched.add(Job::new_async("0 */5 * * * *", move |_id, _sched| Box::pin(cron_tick()))?).await?;
    sched.start().await?;

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        tick().await;
    });
    Ok(sched)
}

async fn run_due(pool: &PgPool, client: &wreq::Client, lock: &Mutex<()>, last: &LastRun) -> anyhow::Result<()> {
    let stores: Vec<(Uuid, i32)> = sqlx::query_as("SELECT id, scrape_interval_minutes FROM stores WHERE is_active")
        .fetch_all(pool)
        .await?;
    let now = Instant::now();
    let due: Vec<Uuid> = {
        let last = last.lock().await;
        stores
            .into_iter()
            .filter(|(id, mins)| {
                let interval = Duration::from_secs((*mins).max(1) as u64 * 60);
                last.get(id).map_or(true, |t| now.duration_since(*t) >= interval)
            })
            .map(|(id, _)| id)
            .collect()
    };
    if due.is_empty() {
        return Ok(());
    }
    if run_exclusive(lock, pool, client, Some(due.clone())).await?.is_some() {
        let mut last = last.lock().await;
        for id in due {
            last.insert(id, now);
        }
    }
    Ok(())
}
