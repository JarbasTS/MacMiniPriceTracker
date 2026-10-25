use async_trait::async_trait;
use serde_json::Value;
use tracing::debug;

use super::{fetch_html, json_decimal, StoreScraper};
use crate::models::{RawScrapedItem, SearchTarget, Store};

/// Preço oficial de cada configuração base do Mac mini na Apple Store BR (referência de "preço cheio").
pub struct AppleBrScraper;

#[async_trait]
impl StoreScraper for AppleBrScraper {
    fn scraper_type(&self) -> &'static str {
        "apple_br"
    }

    // shortcut: a página é a mesma para todo alvo, então é baixada uma vez por alvo (3 req/varredura); o sanitizador separa M5/M5 Pro/M6. Cachear por varredura se o número de alvos crescer.
    async fn scrape(&self, store: &Store, _target: &SearchTarget, client: &wreq::Client) -> anyhow::Result<Vec<RawScrapedItem>> {
        let html = fetch_html(client, &store.search_url_template).await?;
        Ok(Self::parse_html(&html, &store.search_url_template))
    }
}

impl AppleBrScraper {
    pub fn parse_html(html: &str, page_url: &str) -> Vec<RawScrapedItem> {
        let Some(data) = product_selection_data(html) else {
            debug!("Apple: productSelectionData não encontrado");
            return Vec::new();
        };
        data["mainDisplayValues"]["prices"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(key, p)| {
                Some(RawScrapedItem {
                    external_id: format!("apple-{key}"),
                    title: title_for(key)?,
                    url: page_url.to_string(),
                    image_url: None,
                    seller_name: Some("Apple".to_string()),
                    price_cash: json_decimal(&p["currentPrice"]["raw_amount"])?,
                    price_installments: None,
                })
            })
            .collect()
    }
}

/// O objeto vem dentro de um script JS (`productSelectionData: {...}`), não de uma tag JSON.
/// O StreamDeserializer lê só o primeiro valor e ignora o JS que vem depois.
fn product_selection_data(html: &str) -> Option<Value> {
    let start = html.find("productSelectionData")?;
    let brace = start + html[start..].find('{')?;
    serde_json::Deserializer::from_str(&html[brace..]).into_iter::<Value>().next()?.ok()
}

/// "m5pro-15-16" → "Apple Mac mini M5 Pro CPU 15 núcleos GPU 16 núcleos".
fn title_for(price_key: &str) -> Option<String> {
    let mut parts = price_key.split('-');
    let chip = parts.next()?.to_uppercase().replace("PRO", " Pro").replace("MAX", " Max");
    let (cpu, gpu) = (parts.next()?, parts.next()?);
    Some(format!("Apple Mac mini {chip} CPU {cpu} núcleos GPU {gpu} núcleos"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn parse_html_precos_por_configuracao() {
        let html = r#"<script>window.PRODUCT_SELECTION_BOOTSTRAP = { productSelectionData: {"products":[],
          "mainDisplayValues":{"prices":{
            "m6-12-12":{"amount":10799,"currentPrice":{"amount":"R$ 10.799,00","raw_amount":"10799.00"}},
            "m5pro-15-16":{"amount":18999,"currentPrice":{"amount":"R$ 18.999,00","raw_amount":"18999.00"}},
            "quebrado":{"currentPrice":{}}
          }}}, other: 1 };</script>"#;
        let mut items = AppleBrScraper::parse_html(html, "https://www.apple.com/br/shop/buy-mac/mac-mini");
        items.sort_by(|a, b| a.external_id.cmp(&b.external_id));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].external_id, "apple-m5pro-15-16");
        assert_eq!(items[0].title, "Apple Mac mini M5 Pro CPU 15 núcleos GPU 16 núcleos");
        assert_eq!(items[1].price_cash, Decimal::from_str("10799").unwrap());
    }

    #[test]
    fn titulos_passam_no_sanitizador_certo() {
        use crate::scrapers::sanitizer::normalize;
        let m6 = normalize(&title_for("m6-12-12").unwrap());
        let pro = normalize(&title_for("m5pro-18-20").unwrap());
        assert!(m6.contains(" m6 ") && !m6.contains(" pro "));
        assert!(pro.contains(" m5 ") && pro.contains(" pro "));
    }
}
