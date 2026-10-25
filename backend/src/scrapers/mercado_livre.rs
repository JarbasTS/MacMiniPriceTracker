use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;
use scraper::{ElementRef, Html};
use serde_json::Value;
use tracing::debug;

use super::{abs_url, fetch_html, img_src, json_decimal, sanitizer::parse_brl, search_url, sel, text, StoreScraper};
use crate::models::{RawScrapedItem, SearchTarget, Store};

pub struct MercadoLivreScraper;

#[async_trait]
impl StoreScraper for MercadoLivreScraper {
    fn scraper_type(&self) -> &'static str {
        "mercado_livre"
    }

    async fn scrape(&self, store: &Store, target: &SearchTarget, client: &wreq::Client) -> anyhow::Result<Vec<RawScrapedItem>> {
        if store.scraper_config["use_api_fallback"].as_bool() == Some(true) {
            // A API pública passou a exigir token (401/403/429); qualquer falha cai no HTML.
            match fetch_api(client, &target.search_query).await {
                Ok(items) => return Ok(items),
                Err(e) => debug!("ML API indisponível ({e}); usando HTML"),
            }
        }
        let url = search_url(store, &target.search_query.replace(' ', "-"));
        let html = fetch_html(client, &url).await?;
        // Anti-bot responde 200 com redirect para /gz/account-verification.
        if html.contains("account-verification") {
            anyhow::bail!("bloqueio anti-bot (account-verification)");
        }
        Ok(Self::parse_html(&html, &store.base_url))
    }
}

async fn fetch_api(client: &wreq::Client, query: &str) -> anyhow::Result<Vec<RawScrapedItem>> {
    let json: Value = client
        .get("https://api.mercadolibre.com/sites/MLB/search")
        .query(&[("q", query)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(parse_api(&json))
}

fn parse_api(json: &Value) -> Vec<RawScrapedItem> {
    json["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            Some(RawScrapedItem {
                external_id: r["id"].as_str()?.to_string(),
                title: r["title"].as_str()?.to_string(),
                url: r["permalink"].as_str()?.to_string(),
                image_url: r["thumbnail"].as_str().map(String::from),
                seller_name: r["seller"]["nickname"].as_str().map(String::from),
                price_cash: json_decimal(&r["price"])?,
                price_installments: None,
            })
        })
        .collect()
}

/// `MLB-1234567` e `MLB1234567` viram o mesmo id (formato da API); sem match, a URL sem query/fragmento.
pub fn external_id(url: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"MLB-?\d+").expect("regex estática"));
    match RE.find(url) {
        Some(m) => m.as_str().replace('-', ""),
        None => url.split(['?', '#']).next().unwrap_or(url).to_string(),
    }
}

impl MercadoLivreScraper {
    pub fn parse_html(html: &str, base_url: &str) -> Vec<RawScrapedItem> {
        let doc = Html::parse_document(html);
        doc.select(&sel(".ui-search-layout__item"))
            .filter_map(|card| {
                let item = parse_card(card, base_url);
                if item.is_none() {
                    debug!("ML: card descartado (campo faltando)");
                }
                item
            })
            .collect()
    }
}

fn parse_card(card: ElementRef, base_url: &str) -> Option<RawScrapedItem> {
    let title = card.select(&sel(".poly-component__title, .ui-search-item__title")).next().map(text)?;
    // Pula o preço riscado ("de R$ X") e pega o atual.
    let amount = card
        .select(&sel(".andes-money-amount"))
        .find(|m| !m.value().classes().any(|c| c == "andes-money-amount--previous"))?;
    let fraction = amount.select(&sel(".andes-money-amount__fraction")).next().map(text)?;
    let price_cash = match amount.select(&sel(".andes-money-amount__cents")).next() {
        Some(cents) => parse_brl(&format!("{fraction},{}", text(cents))),
        None => parse_brl(&fraction),
    }?;
    let href = card.select(&sel("a[href]")).next()?.value().attr("href")?;
    let url = abs_url(base_url, href)?;
    Some(RawScrapedItem {
        external_id: external_id(&url),
        title,
        image_url: card.select(&sel("img")).next().and_then(img_src),
        url,
        seller_name: None,
        price_cash,
        price_installments: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    const FIXTURE: &str = r#"<ol>
      <li class="ui-search-layout__item"><div class="poly-card">
        <img src="data:image/gif;base64,R0lG" data-src="https://http2.mlstatic.com/mac.webp">
        <h3><a class="poly-component__title" href="https://www.mercadolivre.com.br/apple-mac-mini-m5/p/MLB-1234567?tracking=1#poly">Apple Mac mini M5 16GB</a></h3>
        <s class="andes-money-amount andes-money-amount--previous"><span class="andes-money-amount__fraction">6.999</span></s>
        <div class="poly-price__current"><span class="andes-money-amount">
          <span class="andes-money-amount__fraction">5.499</span><span class="andes-money-amount__cents">90</span>
        </span></div>
      </div></li>
      <li class="ui-search-layout__item"><a class="poly-component__title" href="https://www.mercadolivre.com.br/MLB999">Mac mini sem preço</a></li>
    </ol>"#;

    #[test]
    fn parse_html_card_valido() {
        let items = MercadoLivreScraper::parse_html(FIXTURE, "https://www.mercadolivre.com.br");
        assert_eq!(items.len(), 1);
        let it = &items[0];
        assert_eq!(it.external_id, "MLB1234567");
        assert_eq!(it.title, "Apple Mac mini M5 16GB");
        assert_eq!(it.price_cash, Decimal::from_str("5499.90").unwrap());
        assert_eq!(it.image_url.as_deref(), Some("https://http2.mlstatic.com/mac.webp"));
    }

    #[test]
    fn external_id_formatos() {
        assert_eq!(external_id("https://x.com/MLB-1234567-mac-mini"), "MLB1234567");
        assert_eq!(external_id("https://x.com/p/MLB1234567?a=1"), "MLB1234567");
        assert_eq!(external_id("https://x.com/produto?a=1"), "https://x.com/produto");
    }

    #[test]
    fn parse_api_resultados() {
        let json: Value = serde_json::from_str(
            r#"{"results":[{"id":"MLB1","title":"Mac mini M5","permalink":"https://x/MLB1","thumbnail":"https://x/i.jpg","price":5499.9,"seller":{"nickname":"LOJA"}},{"id":"MLB2","title":"sem preço"}]}"#,
        )
        .unwrap();
        let items = parse_api(&json);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].price_cash, Decimal::from_str("5499.9").unwrap());
        assert_eq!(items[0].seller_name.as_deref(), Some("LOJA"));
    }
}
