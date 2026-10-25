use axum::{
    extract::{rejection::JsonRejection, rejection::PathRejection, Path, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::{error::AppError, models::SearchTarget};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/targets", get(list).post(create))
        .route("/targets/:id", patch(update).delete(remove))
}

#[derive(Deserialize)]
struct NewTarget {
    name: String,
    chip_family: String,
    search_query: String,
    required_keywords: Vec<String>,
    /// Ausente = default do schema (lista anti-acessórios).
    excluded_keywords: Option<Vec<String>>,
    min_price_brl: Decimal,
    max_price_brl: Option<Decimal>,
    is_active: Option<bool>,
}

// shortcut: Option simples não distingue "ausente" de null, então PATCH não consegue limpar max_price_brl. Usar Option<Option<_>> com serde_with se a UI precisar.
#[derive(Deserialize)]
struct TargetPatch {
    name: Option<String>,
    chip_family: Option<String>,
    search_query: Option<String>,
    required_keywords: Option<Vec<String>>,
    excluded_keywords: Option<Vec<String>>,
    min_price_brl: Option<Decimal>,
    max_price_brl: Option<Decimal>,
    is_active: Option<bool>,
}

fn validate(
    name: &str,
    chip_family: &str,
    search_query: &str,
    required: &[String],
    min: Decimal,
    max: Option<Decimal>,
) -> Result<(), AppError> {
    let bad = |msg: &str| Err(AppError::BadRequest(msg.into()));
    if name.trim().is_empty() || chip_family.trim().is_empty() || search_query.trim().is_empty() {
        return bad("name, chip_family e search_query são obrigatórios");
    }
    if required.is_empty() || required.iter().any(|k| k.trim().is_empty()) {
        return bad("required_keywords não pode ser vazia nem ter itens em branco");
    }
    if min <= Decimal::ZERO {
        return bad("min_price_brl deve ser > 0");
    }
    if max.is_some_and(|m| m < min) {
        return bad("max_price_brl deve ser >= min_price_brl");
    }
    Ok(())
}

async fn list(State(st): State<AppState>) -> Result<Json<Vec<SearchTarget>>, AppError> {
    Ok(Json(sqlx::query_as("SELECT * FROM search_targets ORDER BY name").fetch_all(&st.pool).await?))
}

async fn create(
    State(st): State<AppState>,
    body: Result<Json<NewTarget>, JsonRejection>,
) -> Result<(StatusCode, Json<SearchTarget>), AppError> {
    let Json(b) = body?;
    validate(&b.name, &b.chip_family, &b.search_query, &b.required_keywords, b.min_price_brl, b.max_price_brl)?;
    let mut tx = st.pool.begin().await?;
    let mut target: SearchTarget = sqlx::query_as(
        "INSERT INTO search_targets (name, chip_family, search_query, required_keywords, min_price_brl, max_price_brl, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
    )
    .bind(&b.name)
    .bind(&b.chip_family)
    .bind(&b.search_query)
    .bind(&b.required_keywords)
    .bind(b.min_price_brl)
    .bind(b.max_price_brl)
    .bind(b.is_active.unwrap_or(true))
    .fetch_one(&mut *tx)
    .await?;
    // Separado do INSERT para não duplicar em Rust o default de excluded_keywords do schema.
    if let Some(excluded) = &b.excluded_keywords {
        target = sqlx::query_as("UPDATE search_targets SET excluded_keywords = $2 WHERE id = $1 RETURNING *")
            .bind(target.id)
            .bind(excluded)
            .fetch_one(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(target)))
}

async fn update(
    State(st): State<AppState>,
    id: Result<Path<Uuid>, PathRejection>,
    body: Result<Json<TargetPatch>, JsonRejection>,
) -> Result<Json<SearchTarget>, AppError> {
    let (Path(id), Json(p)) = (id?, body?);
    let mut t: SearchTarget = sqlx::query_as("SELECT * FROM search_targets WHERE id = $1").bind(id).fetch_one(&st.pool).await?;
    if let Some(v) = p.name {
        t.name = v;
    }
    if let Some(v) = p.chip_family {
        t.chip_family = v;
    }
    if let Some(v) = p.search_query {
        t.search_query = v;
    }
    if let Some(v) = p.required_keywords {
        t.required_keywords = v;
    }
    if let Some(v) = p.excluded_keywords {
        t.excluded_keywords = v;
    }
    if let Some(v) = p.min_price_brl {
        t.min_price_brl = v;
    }
    if p.max_price_brl.is_some() {
        t.max_price_brl = p.max_price_brl;
    }
    if let Some(v) = p.is_active {
        t.is_active = v;
    }
    validate(&t.name, &t.chip_family, &t.search_query, &t.required_keywords, t.min_price_brl, t.max_price_brl)?;
    let target = sqlx::query_as(
        "UPDATE search_targets SET name = $2, chip_family = $3, search_query = $4, required_keywords = $5, \
         excluded_keywords = $6, min_price_brl = $7, max_price_brl = $8, is_active = $9 WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(&t.name)
    .bind(&t.chip_family)
    .bind(&t.search_query)
    .bind(&t.required_keywords)
    .bind(&t.excluded_keywords)
    .bind(t.min_price_brl)
    .bind(t.max_price_brl)
    .bind(t.is_active)
    .fetch_one(&st.pool)
    .await?;
    Ok(Json(target))
}

async fn remove(State(st): State<AppState>, id: Result<Path<Uuid>, PathRejection>) -> Result<StatusCode, AppError> {
    let Path(id) = id?;
    let done = sqlx::query("DELETE FROM search_targets WHERE id = $1").bind(id).execute(&st.pool).await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }

    #[test]
    fn validacao_de_alvo() {
        let req = vec!["mac".to_string()];
        assert!(validate("M6", "M6", "mac mini m6", &req, d(4000), None).is_ok());
        assert!(validate("M6", "M6", "mac mini m6", &req, d(0), None).is_err());
        assert!(validate("M6", "M6", "mac mini m6", &req, d(4000), Some(d(3999))).is_err());
        assert!(validate("M6", "M6", "mac mini m6", &[], d(4000), None).is_err());
        assert!(validate("M6", "M6", "  ", &req, d(4000), None).is_err());
    }
}
