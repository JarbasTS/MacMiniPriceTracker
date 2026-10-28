use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use serde_json::{json, Value};
use tracing::warn;

use super::AppState;
use crate::{error::AppError, scrapers::runner::run_scrape};

pub fn routes() -> Router<AppState> {
    Router::new().route("/scrape/trigger", post(trigger)).route("/scrape/status", get(status))
}

async fn trigger(State(st): State<AppState>) -> Result<(StatusCode, Json<Value>), AppError> {
    // Trava pega antes do spawn: a resposta já sabe se vai rodar (202) ou se há outra varredura (409).
    let Ok(guard) = st.scrape_lock.clone().try_lock_owned() else {
        return Err(AppError::Conflict("scrape already running".into()));
    };
    tokio::spawn(async move {
        let _guard = guard;
        if let Err(e) = run_scrape(&st.pool, &st.client, None).await {
            warn!("varredura manual falhou: {e:#}");
        }
    });
    Ok((StatusCode::ACCEPTED, Json(json!({ "status": "accepted" }))))
}

/// O front consulta até `running` virar false para recarregar os dados.
async fn status(State(st): State<AppState>) -> Json<Value> {
    Json(json!({ "running": st.scrape_lock.try_lock().is_err() }))
}
