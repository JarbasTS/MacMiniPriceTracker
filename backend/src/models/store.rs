use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(sqlx::FromRow, Serialize, Deserialize, Clone, Debug)]
pub struct Store {
    pub id: Uuid,
    pub slug: String,
    pub scraper_type: String,
    pub name: String,
    pub base_url: String,
    pub search_url_template: String,
    pub is_active: bool,
    pub scrape_interval_minutes: i32,
    pub scraper_config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
