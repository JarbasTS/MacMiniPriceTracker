use axum::{
    extract::{rejection::JsonRejection, rejection::PathRejection, Path, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use super::AppState;
use crate::{
    error::AppError,
    models::Store,
    scrapers::{dynamic_css::DynamicConfig, scraper_for},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/stores", get(list).post(create))
        .route("/stores/:id", patch(update))
}

#[derive(Deserialize)]
struct NewStore {
    slug: String,
    scraper_type: String,
    name: String,
    base_url: String,
    search_url_template: String,
    is_active: Option<bool>,
    scrape_interval_minutes: Option<i32>,
    scraper_config: Option<Value>,
}

#[derive(Deserialize)]
struct StorePatch {
    is_active: Option<bool>,
    name: Option<String>,
    search_url_template: Option<String>,
    scrape_interval_minutes: Option<i32>,
    scraper_config: Option<Value>,
}

/// Borda de confiança: tudo aqui vem da UI e vai direto para o scraper.
fn validate(s: &Store) -> Result<(), AppError> {
    let bad = |msg: String| Err(AppError::BadRequest(msg));
    if s.slug.trim().is_empty() || s.name.trim().is_empty() {
        return bad("slug e name são obrigatórios".into());
    }
    if scraper_for(&s.scraper_type).is_none() {
        return bad(format!("scraper_type desconhecido: {}", s.scraper_type));
    }
    if !s.base_url.starts_with("http") {
        return bad("base_url deve começar com http".into());
    }
    // apple_br não busca por termo: a URL é a página fixa do produto.
    let needs_query = s.scraper_type != "apple_br";
    if !s.search_url_template.starts_with("http") || (needs_query && !s.search_url_template.contains("{query}")) {
        return bad("search_url_template deve começar com http e conter {query}".into());
    }
    // Teto de 24 h: a API só considera ativo o anúncio visto nos últimos 2 dias.
    if !(1..=1440).contains(&s.scrape_interval_minutes) {
        return bad("scrape_interval_minutes deve estar entre 1 e 1440".into());
    }
    if s.scraper_type == "dynamic_css" {
        let cfg: DynamicConfig = serde_json::from_value(s.scraper_config.clone())
            .map_err(|e| AppError::BadRequest(format!("scraper_config inválido: {e}")))?;
        cfg.compile().map_err(|e| AppError::BadRequest(e.to_string()))?;
    }
    Ok(())
}

async fn list(State(st): State<AppState>) -> Result<Json<Vec<Store>>, AppError> {
    Ok(Json(sqlx::query_as("SELECT * FROM stores ORDER BY name").fetch_all(&st.pool).await?))
}

async fn create(State(st): State<AppState>, body: Result<Json<NewStore>, JsonRejection>) -> Result<(StatusCode, Json<Store>), AppError> {
    let Json(b) = body?;
    let draft = Store {
        id: Uuid::nil(),
        slug: b.slug,
        scraper_type: b.scraper_type,
        name: b.name,
        base_url: b.base_url,
        search_url_template: b.search_url_template,
        is_active: b.is_active.unwrap_or(true),
        scrape_interval_minutes: b.scrape_interval_minutes.unwrap_or(30),
        scraper_config: b.scraper_config.unwrap_or_else(|| Value::Object(Default::default())),
        created_at: Utc::now(),
    };
    validate(&draft)?;
    let store = sqlx::query_as(
        "INSERT INTO stores (slug, scraper_type, name, base_url, search_url_template, is_active, scrape_interval_minutes, scraper_config) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
    )
    .bind(&draft.slug)
    .bind(&draft.scraper_type)
    .bind(&draft.name)
    .bind(&draft.base_url)
    .bind(&draft.search_url_template)
    .bind(draft.is_active)
    .bind(draft.scrape_interval_minutes)
    .bind(&draft.scraper_config)
    .fetch_one(&st.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(store)))
}

async fn update(
    State(st): State<AppState>,
    id: Result<Path<Uuid>, PathRejection>,
    body: Result<Json<StorePatch>, JsonRejection>,
) -> Result<Json<Store>, AppError> {
    let (Path(id), Json(p)) = (id?, body?);
    // Mescla com o registro atual e valida o resultado inteiro (ex.: scraper_config contra o scraper_type).
    let mut s: Store = sqlx::query_as("SELECT * FROM stores WHERE id = $1").bind(id).fetch_one(&st.pool).await?;
    if let Some(v) = p.is_active {
        s.is_active = v;
    }
    if let Some(v) = p.name {
        s.name = v;
    }
    if let Some(v) = p.search_url_template {
        s.search_url_template = v;
    }
    if let Some(v) = p.scrape_interval_minutes {
        s.scrape_interval_minutes = v;
    }
    if let Some(v) = p.scraper_config {
        s.scraper_config = v;
    }
    validate(&s)?;
    let store = sqlx::query_as(
        "UPDATE stores SET is_active = $2, name = $3, search_url_template = $4, scrape_interval_minutes = $5, scraper_config = $6 \
         WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(s.is_active)
    .bind(&s.name)
    .bind(&s.search_url_template)
    .bind(s.scrape_interval_minutes)
    .bind(&s.scraper_config)
    .fetch_one(&st.pool)
    .await?;
    Ok(Json(store))
}
