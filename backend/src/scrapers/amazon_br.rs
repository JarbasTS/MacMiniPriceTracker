use async_trait::async_trait;
use scraper::{ElementRef, Html};
use sqlx::PgPool;
use tracing::debug;

use super::{abs_url, fetch_html, img_src, sanitizer::parse_brl, search_url, sel, text, StoreScraper};
use crate::models::{RawScrapedItem, SearchTarget, Store};

pub struct AmazonBrScraper;

#[async_trait]
impl StoreScraper for AmazonBrScraper {
    async fn scrape(&self, store: &Store, target: &SearchTarget, client: &wreq::Client, _pool: &PgPool) -> anyhow::Result<Vec<RawScrapedItem>> {
        let html = fetch_html(client, &search_url(store, &target.search_query)).await?;
        if html.contains("validateCaptcha") {
            anyhow::bail!("captcha");
        }
        Ok(Self::parse_html(&html, &store.base_url))
    }
}

impl AmazonBrScraper {
    pub fn parse_html(html: &str, base_url: &str) -> Vec<RawScrapedItem> {
        let doc = Html::parse_document(html);
        doc.select(&sel(r#"div[data-component-type="s-search-result"]"#))
            .filter_map(|card| {
                let item = parse_card(card, base_url);
                if item.is_none() {
                    debug!("Amazon: card descartado (patrocinado ou campo faltando)");
                }
                item
            })
            .collect()
    }
}

fn parse_card(card: ElementRef, base_url: &str) -> Option<RawScrapedItem> {
    // data-asin vazio = bloco patrocinado.
    let asin = card.value().attr("data-asin").filter(|a| !a.is_empty())?;
    let title = card.select(&sel("h2 span")).next().map(text).filter(|t| !t.is_empty())?;
    let price_cash = card
        .select(&sel(".a-price:not(.a-text-price) .a-offscreen"))
        .next()
        .and_then(|p| parse_brl(&text(p)))
        .or_else(|| {
            // .a-price-whole traz "5.499," — só os dígitos interessam.
            let whole: String = text(card.select(&sel(".a-price-whole")).next()?).chars().filter(char::is_ascii_digit).collect();
            let fraction = card.select(&sel(".a-price-fraction")).next().map(text).unwrap_or_default();
            parse_brl(&format!("{whole},{fraction}"))
        })?;
    let href = card.select(&sel("h2 a[href], a.a-link-normal[href]")).next()?.value().attr("href")?;
    Some(RawScrapedItem {
        external_id: asin.to_string(),
        title,
        url: abs_url(base_url, href)?,
        image_url: card.select(&sel("img.s-image")).next().and_then(img_src),
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

    const FIXTURE: &str = r#"
      <div data-component-type="s-search-result" data-asin="B0TESTE123">
        <img class="s-image" src="https://m.media-amazon.com/mac.jpg">
        <a class="a-link-normal" href="/Apple-Mac-mini/dp/B0TESTE123/ref=sr_1_1"><h2><span>Apple Mac mini M5 16GB</span></h2></a>
        <span class="a-price a-text-price"><span class="a-offscreen">R$ 6.999,00</span></span>
        <span class="a-price"><span class="a-offscreen">R$ 5.499,90</span></span>
      </div>
      <div data-component-type="s-search-result" data-asin="B0SEMPRECO">
        <a class="a-link-normal" href="/x"><h2><span>Mac mini sem preço</span></h2></a>
      </div>"#;

    #[test]
    fn parse_html_card_valido() {
        let items = AmazonBrScraper::parse_html(FIXTURE, "https://www.amazon.com.br");
        assert_eq!(items.len(), 1);
        let it = &items[0];
        assert_eq!(it.external_id, "B0TESTE123");
        assert_eq!(it.title, "Apple Mac mini M5 16GB");
        assert_eq!(it.url, "https://www.amazon.com.br/Apple-Mac-mini/dp/B0TESTE123/ref=sr_1_1");
        assert_eq!(it.price_cash, Decimal::from_str("5499.90").unwrap());
        assert_eq!(it.image_url.as_deref(), Some("https://m.media-amazon.com/mac.jpg"));
    }

    #[test]
    fn preco_por_whole_fraction_e_patrocinado() {
        let html = r#"
          <div data-component-type="s-search-result" data-asin="B0WHOLE">
            <h2><a href="/dp/B0WHOLE"><span>Mac mini M6</span></a></h2>
            <span class="a-price"><span class="a-price-whole">6.200,</span><span class="a-price-fraction">50</span></span>
          </div>
          <div data-component-type="s-search-result" data-asin="">
            <h2><a href="/dp/ADS"><span>Patrocinado</span></a></h2>
            <span class="a-price"><span class="a-offscreen">R$ 1,00</span></span>
          </div>"#;
        let items = AmazonBrScraper::parse_html(html, "https://www.amazon.com.br");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].price_cash, Decimal::from_str("6200.50").unwrap());
    }
}
