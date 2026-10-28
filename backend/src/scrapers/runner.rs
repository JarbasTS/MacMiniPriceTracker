use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{sanitizer, scraper_for};
use crate::models::{RawScrapedItem, SearchTarget, Store};

/// Trava compartilhada entre cron e trigger manual: uma varredura por vez.
pub type ScrapeLock = Arc<Mutex<()>>;

#[derive(Debug, Default, Clone)]
pub struct ScrapeReport {
    pub stores: usize,
    pub found: usize,
    pub accepted: usize,
    pub errors: usize,
}

const UPSERT_LISTING: &str = r#"
INSERT INTO listings (store_id, target_id, external_id, title, url, image_url, seller_name,
                      current_price, installment_price, lowest_price, highest_price)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$8,$8)
ON CONFLICT (store_id, external_id) DO UPDATE SET
  title = EXCLUDED.title, url = EXCLUDED.url, image_url = EXCLUDED.image_url,
  seller_name = EXCLUDED.seller_name,
  current_price = EXCLUDED.current_price, installment_price = EXCLUDED.installment_price,
  lowest_price  = LEAST(listings.lowest_price, EXCLUDED.current_price),
  highest_price = GREATEST(listings.highest_price, EXCLUDED.current_price),
  in_stock = true, last_seen_at = NOW()
RETURNING id"#;

// NOT EXISTS cobre anúncio novo e preço inalterado num só statement.
const INSERT_HISTORY: &str = r#"
INSERT INTO price_history (listing_id, price_cash, price_installments)
SELECT $1, $2, $3
WHERE NOT EXISTS (
  SELECT 1 FROM (
    SELECT price_cash FROM price_history
    WHERE listing_id = $1 ORDER BY captured_at DESC LIMIT 1
  ) last
  WHERE last.price_cash = $2
)"#;

// shortcut: in_stock nunca vira false; anúncio sumido só para de atualizar last_seen_at. A API trata "ativo" como last_seen_at > NOW() - 2 days. Trocar por marcação explícita se precisar de histórico de disponibilidade.
pub async fn run_scrape(pool: &PgPool, client: &wreq::Client, store_filter: Option<Vec<Uuid>>) -> anyhow::Result<ScrapeReport> {
    let stores: Vec<Store> =
        sqlx::query_as("SELECT * FROM stores WHERE is_active AND ($1::uuid[] IS NULL OR id = ANY($1)) ORDER BY name")
            .bind(store_filter)
            .fetch_all(pool)
            .await?;
    let targets: Vec<SearchTarget> = sqlx::query_as("SELECT * FROM search_targets WHERE is_active ORDER BY name")
        .fetch_all(pool)
        .await?;

    let mut report = ScrapeReport { stores: stores.len(), ..Default::default() };
    for store in &stores {
        let Some(scraper) = scraper_for(&store.scraper_type) else {
            warn!(store = %store.slug, "scraper_type desconhecido: {}", store.scraper_type);
            report.errors += 1;
            continue;
        };
        debug!(store = %store.slug, scraper = %store.scraper_type, "varrendo loja");
        let antes = report.found;
        let store_targets: Vec<&SearchTarget> = targets.iter().filter(|t| targets_store(store, t)).collect();
        for (i, target) in store_targets.into_iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(jitter()).await;
            }
            let items = match scraper.scrape(store, target, client, pool).await {
                Ok(items) => items,
                Err(e) => {
                    warn!(store = %store.slug, target = %target.name, "falha no scraping: {e:#}");
                    report.errors += 1;
                    continue;
                }
            };
            report.found += items.len();
            for item in items.iter().filter(|it| sanitizer::is_valid(&it.title, it.price_cash, target)) {
                match persist(pool, store, target, item).await {
                    Ok(()) => report.accepted += 1,
                    Err(e) => {
                        warn!(store = %store.slug, external_id = %item.external_id, "falha ao gravar: {e:#}");
                        report.errors += 1;
                    }
                }
            }
        }
        if report.found == antes {
            // Layout novo ou bloqueio silencioso chegam aqui como lista vazia; alvo isolado sem resultado é normal.
            warn!(store = %store.slug, "nenhum alvo retornou itens");
        }
    }
    info!(?report, "varredura concluída");
    Ok(report)
}

/// Roda a varredura se nenhuma outra estiver em andamento; `None` = já havia uma rodando.
pub async fn run_exclusive(
    lock: &Mutex<()>,
    pool: &PgPool,
    client: &wreq::Client,
    store_filter: Option<Vec<Uuid>>,
) -> anyhow::Result<Option<ScrapeReport>> {
    let Ok(_guard) = lock.try_lock() else {
        info!("varredura já em andamento");
        return Ok(None);
    };
    run_scrape(pool, client, store_filter).await.map(Some)
}

async fn persist(pool: &PgPool, store: &Store, target: &SearchTarget, item: &RawScrapedItem) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    let (listing_id,): (Uuid,) = sqlx::query_as(UPSERT_LISTING)
        .bind(store.id)
        .bind(target.id)
        .bind(&item.external_id)
        .bind(&item.title)
        .bind(&item.url)
        .bind(&item.image_url)
        .bind(&item.seller_name)
        .bind(item.price_cash)
        .bind(item.price_installments)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query(INSERT_HISTORY)
        .bind(listing_id)
        .bind(item.price_cash)
        .bind(item.price_installments)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// `scraper_config.chip_families` restringe a loja a esses chips; sem a chave, a loja busca todos os alvos.
fn targets_store(store: &Store, target: &SearchTarget) -> bool {
    match store.scraper_config["chip_families"].as_array() {
        Some(chips) => chips.iter().any(|c| c.as_str().is_some_and(|c| c.eq_ignore_ascii_case(&target.chip_family))),
        None => true,
    }
}

/// Pausa de 2–4 s entre requisições à mesma loja (UUID v4 como fonte aleatória, sem crate nova).
fn jitter() -> Duration {
    Duration::from_millis(2000 + (Uuid::new_v4().as_u128() % 2001) as u64)
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;

    fn item(price: i64) -> RawScrapedItem {
        RawScrapedItem {
            external_id: "T1".into(),
            title: "Mac mini M6".into(),
            url: "https://x.com/T1".into(),
            image_url: None,
            seller_name: None,
            price_cash: Decimal::from(price),
            price_installments: None,
        }
    }

    #[test]
    fn chip_families_filtra_alvos() {
        let store = |cfg: serde_json::Value| Store {
            id: Uuid::new_v4(),
            slug: "ml".into(),
            scraper_type: "mercado_livre".into(),
            name: "ML".into(),
            base_url: String::new(),
            search_url_template: String::new(),
            is_active: true,
            scrape_interval_minutes: 30,
            scraper_config: cfg,
            created_at: chrono::Utc::now(),
        };
        let target = |chip: &str| SearchTarget {
            id: Uuid::new_v4(),
            name: format!("Mac mini {chip}"),
            chip_family: chip.into(),
            search_query: String::new(),
            required_keywords: vec![],
            excluded_keywords: vec![],
            min_price_brl: Decimal::ZERO,
            max_price_brl: None,
            is_active: true,
            created_at: chrono::Utc::now(),
        };
        let so_m6 = store(serde_json::json!({"chip_families": ["m6"]}));
        assert!(targets_store(&so_m6, &target("M6")));
        assert!(!targets_store(&so_m6, &target("M5")));
        assert!(!targets_store(&so_m6, &target("M5 Pro")));
        assert!(targets_store(&store(serde_json::json!({})), &target("M5")), "sem a chave busca todos");
    }

    // Usa as lojas e alvos semeados pelas migrations.
    #[ignore = "precisa de DATABASE_URL (Postgres ligado)"]
    #[sqlx::test]
    async fn persist_guarda_extremos_e_historico_sem_duplicar(pool: PgPool) -> sqlx::Result<()> {
        let store: Store = sqlx::query_as("SELECT * FROM stores LIMIT 1").fetch_one(&pool).await?;
        let target: SearchTarget = sqlx::query_as("SELECT * FROM search_targets LIMIT 1").fetch_one(&pool).await?;
        for price in [5000, 5000, 4500] {
            persist(&pool, &store, &target, &item(price)).await.unwrap();
        }
        let (lowest, highest, current): (Decimal, Decimal, Decimal) =
            sqlx::query_as("SELECT lowest_price, highest_price, current_price FROM listings").fetch_one(&pool).await?;
        assert_eq!((lowest, highest, current), (Decimal::from(4500), Decimal::from(5000), Decimal::from(4500)));
        let (rows,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM price_history").fetch_one(&pool).await?;
        assert_eq!(rows, 2, "preço repetido não cria linha nova");
        Ok(())
    }
}
