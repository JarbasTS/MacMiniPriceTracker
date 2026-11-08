mod auth;
mod config;
mod db;
mod error;
mod models;
mod routes;
mod scheduler;
mod scrapers;

use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use routes::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let cfg = config::Config::from_env()?;
    let pool = db::connect(&cfg).await?;
    info!("migrations ok");

    if cfg.access.is_none() {
        warn!("Cloudflare Access não configurado: qualquer um pode alterar dados (ok só em dev)");
    }
    let client = scrapers::http_client()?;
    let auth = Arc::new(auth::Auth::new(cfg.access.clone(), client.clone()));
    let state = AppState { pool, client, scrape_lock: Arc::default(), auth };
    // O handle precisa viver até o fim do processo.
    let _scheduler = scheduler::start(state.pool.clone(), state.client.clone(), state.scrape_lock.clone()).await?;

    let listener = TcpListener::bind(("0.0.0.0", cfg.port)).await?;
    info!("API ouvindo em {}", listener.local_addr()?);
    axum::serve(listener, routes::router(state)).with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}

/// Ctrl+C local ou SIGTERM do `docker stop`.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
    info!("encerrando");
}
