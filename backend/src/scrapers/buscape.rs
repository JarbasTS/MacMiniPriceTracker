use async_trait::async_trait;
use scraper::Html;
use serde_json::Value;
use tracing::debug;

use super::{abs_url, fetch_html, json_decimal, search_url, sel, StoreScraper};
use crate::models::{RawScrapedItem, SearchTarget, Store};

/// Comparador: cada resultado traz a melhor oferta entre várias lojas (Magalu, Amazon, ...).
pub struct BuscapeScraper;

#[async_trait]
impl StoreScraper for BuscapeScraper {
    fn scraper_type(&self) -> &'static str {
        "buscape"
    }

    async fn scrape(&self, store: &Store, target: &SearchTarget, client: &wreq::Client) -> anyhow::Result<Vec<RawScrapedItem>> {
        let html = fetch_html(client, &search_url(store, &target.search_query)).await?;
        Ok(Self::parse_html(&html, &store.base_url))
    }
}

impl BuscapeScraper {
    pub fn parse_html(html: &str, base_url: &str) -> Vec<RawScrapedItem> {
        let doc = Html::parse_document(html);
        let Some(script) = doc.select(&sel("script#__NEXT_DATA__")).next() else {
            debug!("Buscapé: página sem __NEXT_DATA__");
            return Vec::new();
        };
        let data: Value = match serde_json::from_str(&script.text().collect::<String>()) {
            Ok(v) => v,
            Err(e) => {
                debug!("Buscapé: __NEXT_DATA__ inválido ({e})");
                return Vec::new();
            }
        };
        data["props"]["initialReduxState"]["hits"]["hits"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|hit| {
                let item = parse_hit(hit, base_url);
                if item.is_none() {
                    debug!("Buscapé: resultado descartado (campo faltando)");
                }
                item
            })
            .collect()
    }
}

fn parse_hit(hit: &Value, base_url: &str) -> Option<RawScrapedItem> {
    Some(RawScrapedItem {
        external_id: hit["objectId"].as_str()?.to_string(),
        title: hit["name"].as_str()?.to_string(),
        url: abs_url(base_url, hit["url"].as_str()?)?,
        image_url: hit["image"].as_str().map(String::from),
        // Loja da melhor oferta; o dashboard mostra "Magazine Luiza via Buscapé".
        seller_name: hit["bestOffer"]["merchantName"].as_str().map(String::from),
        price_cash: json_decimal(&hit["price"])?,
        price_installments: json_decimal(&hit["installments"]["total_value"]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn parse_html_resultado_valido() {
        let html = r#"<html><body><script id="__NEXT_DATA__" type="application/json">
          {"props":{"initialReduxState":{"hits":{"hits":[
            {"objectId":"product13994856","name":"Apple Mac Mini M5 16 GB RAM","price":7444.44,
             "installments":{"amount_months":6,"price":1240.74,"total_value":7444.44},
             "bestOffer":{"merchantName":"Magazine Luiza"},"image":"https://image.zst.com.br/x.jpg",
             "url":"/pc-computador/mac-mini-apple-m5"},
            {"objectId":"product2","name":"Mac mini sem preço","url":"/x"}
          ]}}}}
        </script></body></html>"#;
        let items = BuscapeScraper::parse_html(html, "https://www.buscape.com.br");
        assert_eq!(items.len(), 1);
        let it = &items[0];
        assert_eq!(it.external_id, "product13994856");
        assert_eq!(it.url, "https://www.buscape.com.br/pc-computador/mac-mini-apple-m5");
        assert_eq!(it.price_cash, Decimal::from_str("7444.44").unwrap());
        assert_eq!(it.seller_name.as_deref(), Some("Magazine Luiza"));
    }

    #[test]
    fn sem_next_data_retorna_vazio() {
        assert!(BuscapeScraper::parse_html("<html></html>", "https://www.buscape.com.br").is_empty());
    }
}
