use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;
use tracing::debug;
use wreq::header::{ACCEPT, REFERER};

use super::{json_decimal, search_url, send_with_retry, StoreScraper};
use crate::models::{RawScrapedItem, SearchTarget, Store};

/// API JSON interna que a própria busca do site chama (não documentada).
pub struct KabumApiScraper;

#[async_trait]
impl StoreScraper for KabumApiScraper {
    async fn scrape(&self, store: &Store, target: &SearchTarget, client: &wreq::Client, _pool: &PgPool) -> anyhow::Result<Vec<RawScrapedItem>> {
        // A API só casa o termo com hífens ("mac-mini-m5"); com espaço devolve 404.
        let url = search_url(store, &target.search_query.replace(' ', "-"));
        let resp = send_with_retry(|| {
            client
                .get(&url)
                .header(ACCEPT, "application/json")
                .header(REFERER, "https://www.kabum.com.br/")
        })
        .await?;
        // 404 CATALOG_NOT_MATCH = busca sem resultados, não é falha.
        if resp.status() == wreq::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        let json: Value = resp.error_for_status()?.json().await?;
        Ok(Self::parse_json(&json, &store.base_url))
    }
}

impl KabumApiScraper {
    pub fn parse_json(json: &Value, base_url: &str) -> Vec<RawScrapedItem> {
        json["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| {
                let item = parse_product(p, base_url);
                if item.is_none() {
                    debug!("KaBuM: produto descartado (indisponível ou campo faltando)");
                }
                item
            })
            .collect()
    }
}

fn parse_product(p: &Value, base_url: &str) -> Option<RawScrapedItem> {
    let a = &p["attributes"];
    if a["available"].as_bool() != Some(true) {
        return None;
    }
    let id = p["id"].as_u64()?;
    // Oferta relâmpago, quando existe, tem preço próprio.
    let price_cash = json_decimal(&a["offer"]["price_with_discount"]).or_else(|| json_decimal(&a["price_with_discount"]))?;
    Some(RawScrapedItem {
        external_id: id.to_string(),
        title: a["title"].as_str()?.to_string(),
        url: format!("{}/produto/{id}", base_url.trim_end_matches('/')),
        image_url: a["images"][0].as_str().map(String::from),
        // Vazio quando o vendedor é a própria KaBuM.
        seller_name: a["marketplace"]["seller_name"].as_str().map(String::from),
        price_cash,
        price_installments: json_decimal(&a["price"]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn parse_json_produto_valido() {
        let json: Value = serde_json::from_str(
            r#"{"data":[
              {"type":"product","id":1081041,"attributes":{"title":"Mac Mini 2026 M6 16gb RAM 512SSD","price":11200.0,
                "price_with_discount":10700.0,"offer":null,"available":true,
                "images":["https://images.kabum.com.br/1081041.jpg"],"marketplace":{"seller_name":"TRACKER ST"}}},
              {"type":"product","id":2,"attributes":{"title":"Mac mini esgotado","price":9000.0,"price_with_discount":8500.0,"available":false}},
              {"type":"product","id":3,"attributes":{"title":"Mac mini sem preço","available":true}}
            ]}"#,
        )
        .unwrap();
        let items = KabumApiScraper::parse_json(&json, "https://www.kabum.com.br");
        assert_eq!(items.len(), 1);
        let it = &items[0];
        assert_eq!(it.external_id, "1081041");
        assert_eq!(it.url, "https://www.kabum.com.br/produto/1081041");
        assert_eq!(it.price_cash, Decimal::from_str("10700").unwrap());
        assert_eq!(it.price_installments, Some(Decimal::from_str("11200").unwrap()));
        assert_eq!(it.seller_name.as_deref(), Some("TRACKER ST"));
    }

    #[test]
    fn oferta_relampago_tem_prioridade() {
        let json: Value = serde_json::from_str(
            r#"{"data":[{"id":7,"attributes":{"title":"Mac mini M6","price":11000.0,"price_with_discount":10500.0,
              "offer":{"price_with_discount":9999.9},"available":true}}]}"#,
        )
        .unwrap();
        let items = KabumApiScraper::parse_json(&json, "https://www.kabum.com.br");
        assert_eq!(items[0].price_cash, Decimal::from_str("9999.9").unwrap());
    }
}
