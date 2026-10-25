use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(sqlx::FromRow, Serialize, Deserialize, Clone, Debug)]
pub struct SearchTarget {
    pub id: Uuid,
    pub name: String,
    pub chip_family: String,
    pub search_query: String,
    pub required_keywords: Vec<String>,
    pub excluded_keywords: Vec<String>,
    pub min_price_brl: Decimal,
    pub max_price_brl: Option<Decimal>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
