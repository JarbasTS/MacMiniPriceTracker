mod listings;
mod scrape;
mod stores;
mod targets;

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{error::AppError, scrapers::runner::ScrapeLock};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub client: wreq::Client,
    pub scrape_lock: ScrapeLock,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api", api_routes())
        .fallback(|| async { AppError::NotFound })
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .merge(listings::routes())
        .merge(stores::routes())
        .merge(targets::routes())
        .merge(scrape::routes())
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
