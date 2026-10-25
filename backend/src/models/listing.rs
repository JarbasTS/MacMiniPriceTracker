use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(sqlx::FromRow, Serialize, Deserialize, Clone, Debug)]
pub struct Listing {
    pub id: Uuid,
    pub store_id: Uuid,
    pub target_id: Uuid,
    pub external_id: String,
    pub title: String,
    pub url: String,
    pub image_url: Option<String>,
    pub seller_name: Option<String>,
    pub current_price: Decimal,
    pub installment_price: Option<Decimal>,
    pub lowest_price: Decimal,
    pub highest_price: Decimal,
    pub in_stock: bool,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

/// Listing + nomes da loja e do alvo (JOIN), formato devolvido pela API.
#[derive(sqlx::FromRow, Serialize, Clone, Debug)]
pub struct ListingView {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub listing: Listing,
    pub store_name: String,
    pub store_slug: String,
    pub target_name: String,
    pub chip_family: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Clone, Debug)]
pub struct PriceHistory {
    pub id: i64,
    pub listing_id: Uuid,
    pub price_cash: Decimal,
    pub price_installments: Option<Decimal>,
    pub captured_at: DateTime<Utc>,
}

/// Item bruto devolvido por um scraper, antes do sanitizador.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RawScrapedItem {
    pub external_id: String,
    pub title: String,
    pub url: String,
    pub image_url: Option<String>,
    pub seller_name: Option<String>,
    pub price_cash: Decimal,
    pub price_installments: Option<Decimal>,
}
