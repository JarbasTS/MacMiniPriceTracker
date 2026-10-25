use anyhow::anyhow;
use async_trait::async_trait;
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use tracing::debug;

use super::{abs_url, fetch_html, img_src, sanitizer::parse_brl, search_url, text, StoreScraper};
use crate::models::{RawScrapedItem, SearchTarget, Store};

pub struct DynamicCssScraper;

#[derive(Deserialize, Debug, Clone)]
pub struct DynamicConfig {
    item_selector: String,
    title_selector: String,
    price_selector: String,
    url_selector: String,
    image_selector: Option<String>,
    external_id_attr: Option<String>,
}

pub struct DynamicSelectors {
    item: Selector,
    title: Selector,
    price: Selector,
    url: Selector,
    image: Option<Selector>,
    external_id_attr: Option<String>,
}

fn css(s: &str) -> anyhow::Result<Selector> {
    // O erro do `scraper` não é Send e a mensagem dele ("please report this to the developer") confunde na UI.
    Selector::parse(s).map_err(|_| anyhow!("seletor CSS inválido: {s:?}"))
}

impl DynamicConfig {
    pub fn compile(&self) -> anyhow::Result<DynamicSelectors> {
        Ok(DynamicSelectors {
            item: css(&self.item_selector)?,
            title: css(&self.title_selector)?,
            price: css(&self.price_selector)?,
            url: css(&self.url_selector)?,
            image: self.image_selector.as_deref().map(css).transpose()?,
            external_id_attr: self.external_id_attr.clone(),
        })
    }
}

#[async_trait]
impl StoreScraper for DynamicCssScraper {
    fn scraper_type(&self) -> &'static str {
        "dynamic_css"
    }

    async fn scrape(&self, store: &Store, target: &SearchTarget, client: &wreq::Client) -> anyhow::Result<Vec<RawScrapedItem>> {
        let cfg: DynamicConfig = serde_json::from_value(store.scraper_config.clone())?;
        let selectors = cfg.compile()?;
        let html = fetch_html(client, &search_url(store, &target.search_query)).await?;
        Ok(Self::parse_html(&html, &store.base_url, &selectors))
    }
}

impl DynamicCssScraper {
    pub fn parse_html(html: &str, base_url: &str, s: &DynamicSelectors) -> Vec<RawScrapedItem> {
        let doc = Html::parse_document(html);
        doc.select(&s.item)
            .filter_map(|card| {
                let item = parse_card(card, base_url, s);
                if item.is_none() {
                    debug!("dynamic_css: card descartado (campo faltando)");
                }
                item
            })
            .collect()
    }
}

fn parse_card(card: ElementRef, base_url: &str, s: &DynamicSelectors) -> Option<RawScrapedItem> {
    let title = card.select(&s.title).next().map(text).filter(|t| !t.is_empty())?;
    let price_cash = parse_brl(&text(card.select(&s.price).next()?))?;
    let href = card.select(&s.url).next()?.value().attr("href")?;
    let url = abs_url(base_url, href)?;
    let external_id = s
        .external_id_attr
        .as_deref()
        .and_then(|attr| card.value().attr(attr))
        .map(String::from)
        .unwrap_or_else(|| url.clone());
    Some(RawScrapedItem {
        external_id,
        title,
        url,
        image_url: s.image.as_ref().and_then(|img| card.select(img).next()).and_then(img_src),
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

    fn kabum() -> DynamicConfig {
        serde_json::from_value(serde_json::json!({
            "item_selector": "article.productCard",
            "title_selector": "span.nameCard",
            "price_selector": "span.priceCard",
            "url_selector": "a.productLink",
            "image_selector": "img.imageCard",
            "external_id_attr": "data-smarthintproductid"
        }))
        .unwrap()
    }

    #[test]
    fn parse_html_card_valido() {
        let html = r#"
          <article class="productCard" data-smarthintproductid="987">
            <a class="productLink" href="/produto/987/mac-mini-m5"><img class="imageCard" src="https://img.kabum.com.br/987.jpg"></a>
            <span class="nameCard">Apple Mac mini M5 16GB</span><span class="priceCard">R$ 5.499,90</span>
          </article>
          <article class="productCard" data-smarthintproductid="988">
            <a class="productLink" href="/produto/988"></a><span class="nameCard">Mac mini sem preço</span>
          </article>"#;
        let items = DynamicCssScraper::parse_html(html, "https://www.kabum.com.br", &kabum().compile().unwrap());
        assert_eq!(items.len(), 1);
        let it = &items[0];
        assert_eq!(it.external_id, "987");
        assert_eq!(it.url, "https://www.kabum.com.br/produto/987/mac-mini-m5");
        assert_eq!(it.price_cash, Decimal::from_str("5499.90").unwrap());
        assert_eq!(it.image_url.as_deref(), Some("https://img.kabum.com.br/987.jpg"));
    }

    #[test]
    fn seletor_invalido_da_erro() {
        let mut cfg = kabum();
        cfg.price_selector = "span[[".into();
        assert!(cfg.compile().is_err());
    }
}
