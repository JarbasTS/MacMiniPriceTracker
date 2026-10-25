use axum::{
    extract::{rejection::PathRejection, rejection::QueryRejection, Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppState;
use crate::{
    error::AppError,
    models::{ListingView, PriceHistory},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/listings", get(list))
        .route("/listings/:id", get(detail))
        .route("/stats/summary", get(summary))
}

const VIEW_SELECT: &str = "SELECT l.*, s.name AS store_name, s.slug AS store_slug, t.name AS target_name, t.chip_family \
     FROM listings l JOIN stores s ON s.id = l.store_id JOIN search_targets t ON t.id = l.target_id";

// "Ativo" = visto nos últimos 2 dias (in_stock nunca vira false; ver shortcut do runner).
const ACTIVE: &str = "l.last_seen_at > NOW() - interval '2 days'";

#[derive(Deserialize)]
struct ListingsQuery {
    chip_family: Option<String>,
    store_id: Option<Uuid>,
    sort: Option<String>,
}

async fn list(State(st): State<AppState>, q: Result<Query<ListingsQuery>, QueryRejection>) -> Result<Json<Vec<ListingView>>, AppError> {
    let Query(q) = q?;
    // `sort` nunca é interpolado: só cláusulas fixas.
    let order = match q.sort.as_deref().unwrap_or("price_asc") {
        "price_asc" => "l.current_price ASC",
        "price_desc" => "l.current_price DESC",
        "recent" => "l.last_seen_at DESC",
        other => return Err(AppError::BadRequest(format!("sort inválido: {other} (use price_asc, price_desc ou recent)"))),
    };
    let sql = format!(
        "{VIEW_SELECT} WHERE {ACTIVE} AND ($1::text IS NULL OR t.chip_family = $1) \
         AND ($2::uuid IS NULL OR l.store_id = $2) ORDER BY {order}"
    );
    let rows = sqlx::query_as(&sql).bind(q.chip_family).bind(q.store_id).fetch_all(&st.pool).await?;
    Ok(Json(rows))
}

#[derive(Serialize)]
struct ListingDetail {
    #[serde(flatten)]
    view: ListingView,
    history: Vec<PriceHistory>,
}

async fn detail(State(st): State<AppState>, id: Result<Path<Uuid>, PathRejection>) -> Result<Json<ListingDetail>, AppError> {
    let Path(id) = id?;
    let view: ListingView = sqlx::query_as(&format!("{VIEW_SELECT} WHERE l.id = $1")).bind(id).fetch_one(&st.pool).await?;
    let history = sqlx::query_as("SELECT * FROM price_history WHERE listing_id = $1 ORDER BY captured_at ASC")
        .bind(id)
        .fetch_all(&st.pool)
        .await?;
    Ok(Json(ListingDetail { view, history }))
}

#[derive(Serialize, sqlx::FromRow)]
struct ChipMin {
    chip_family: String,
    min_price: Decimal,
    store_name: String,
    listing_id: Uuid,
}

#[derive(Serialize)]
struct Summary {
    by_chip: Vec<ChipMin>,
    total_listings: i64,
    last_scrape_at: Option<DateTime<Utc>>,
}

async fn summary(State(st): State<AppState>) -> Result<Json<Summary>, AppError> {
    let by_chip = sqlx::query_as(&format!(
        "SELECT DISTINCT ON (t.chip_family) t.chip_family, l.current_price AS min_price, s.name AS store_name, l.id AS listing_id \
         FROM listings l JOIN search_targets t ON t.id = l.target_id JOIN stores s ON s.id = l.store_id \
         WHERE {ACTIVE} ORDER BY t.chip_family, l.current_price ASC"
    ))
    .fetch_all(&st.pool)
    .await?;
    let (total_listings, last_scrape_at): (i64, Option<DateTime<Utc>>) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FILTER (WHERE {ACTIVE}), MAX(l.last_seen_at) FROM listings l"
    ))
    .fetch_one(&st.pool)
    .await?;
    Ok(Json(Summary { by_chip, total_listings, last_scrape_at }))
}
