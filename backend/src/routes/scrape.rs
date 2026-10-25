use axum::{
    extract::{rejection::QueryRejection, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use tracing::warn;

use super::AppState;
use crate::{error::AppError, scrapers::runner::run_scrape};

pub fn routes() -> Router<AppState> {
    Router::new().route("/scrape/trigger", post(trigger))
}

#[derive(Deserialize)]
struct TriggerQuery {
    #[serde(default)]
    wait: bool,
}

async fn trigger(State(st): State<AppState>, q: Result<Query<TriggerQuery>, QueryRejection>) -> Result<Response, AppError> {
    let Query(q) = q?;
    // Trava pega antes do spawn: a resposta já sabe se vai rodar (202) ou se há outra varredura (409).
    let Ok(guard) = st.scrape_lock.clone().try_lock_owned() else {
        return Err(AppError::Conflict("scrape already running".into()));
    };
    if q.wait {
        let report = run_scrape(&st.pool, &st.client, None).await?;
        drop(guard);
        return Ok(Json(report).into_response());
    }
    tokio::spawn(async move {
        let _guard = guard;
        if let Err(e) = run_scrape(&st.pool, &st.client, None).await {
            warn!("varredura manual falhou: {e:#}");
        }
    });
    Ok((StatusCode::ACCEPTED, Json(json!({ "status": "accepted" }))).into_response())
}
