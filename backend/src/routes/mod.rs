mod listings;
mod scrape;
mod stores;
mod targets;

use std::sync::Arc;

use axum::{middleware, routing::get, Extension, Json, Router};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    auth::{self, Auth, Viewer},
    error::AppError,
    scrapers::runner::ScrapeLock,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub client: wreq::Client,
    pub scrape_lock: ScrapeLock,
    pub auth: Arc<Auth>,
}

pub fn router(state: AppState) -> Router {
    // O guard cobre as rotas montadas antes do .layer; /health fica aberto.
    let api = api_routes()
        .layer(middleware::from_fn_with_state(state.clone(), auth::guard))
        .route("/health", get(health));
    let app = Router::new().nest("/api", api).fallback(|| async { AppError::NotFound });
    // Em produção o front chama /api na mesma origem; CORS aberto só serve ao dev (front :3000, API :8080).
    let app = if state.auth.enabled() { app } else { app.layer(CorsLayer::permissive()) };
    app.layer(TraceLayer::new_for_http()).with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .merge(listings::routes())
        .merge(stores::routes())
        .merge(targets::routes())
        .merge(scrape::routes())
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn me(Extension(viewer): Extension<Viewer>) -> Json<Viewer> {
    Json(viewer)
}
