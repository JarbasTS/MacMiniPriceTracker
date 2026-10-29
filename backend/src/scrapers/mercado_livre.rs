use std::sync::LazyLock;

use anyhow::Context;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use scraper::{ElementRef, Html};
use serde_json::Value;
use sqlx::PgPool;
use tracing::{debug, info, warn};
use wreq::header::{AUTHORIZATION, CONTENT_TYPE};
use wreq::StatusCode;

use super::{abs_url, fetch_html, img_src, json_decimal, sanitizer, sanitizer::parse_brl, search_url, send_with_retry, sel, text, StoreScraper};
use crate::models::{RawScrapedItem, SearchTarget, Store};

const API: &str = "https://api.mercadolibre.com";

pub struct MercadoLivreScraper;

#[async_trait]
impl StoreScraper for MercadoLivreScraper {
    async fn scrape(&self, store: &Store, target: &SearchTarget, client: &wreq::Client, pool: &PgPool) -> anyhow::Result<Vec<RawScrapedItem>> {
        // Só o catálogo (/products) responde para app não certificado; a busca comum dá 403.
        match fetch_api(pool, client, target).await {
            Ok(items) => return Ok(items),
            Err(e) => debug!("ML API indisponível ({e:#}); usando HTML"),
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

/// Busca no catálogo e pega a oferta mais barata de cada produto cujo nome bate com o alvo.
async fn fetch_api(pool: &PgPool, client: &wreq::Client, target: &SearchTarget) -> anyhow::Result<Vec<RawScrapedItem>> {
    let bearer = format!("Bearer {}", access_token(pool, client).await?);
    let search: Value = send_with_retry(|| {
        client
            .get(format!("{API}/products/search"))
            .query(&[("status", "active"), ("site_id", "MLB"), ("q", target.search_query.as_str()), ("limit", "50")])
            .header(AUTHORIZATION, &bearer)
    })
    .await?
    .error_for_status()?
    .json()
    .await?;
    let products = search["results"].as_array().map(Vec::as_slice).unwrap_or_default();
    // O catálogo mistura acessórios e chips antigos; filtrar antes poupa uma requisição por produto.
    let relevant: Vec<&Value> =
        products.iter().filter(|p| p["name"].as_str().is_some_and(|n| sanitizer::matches_keywords(n, target))).collect();
    let mut items = Vec::new();
    for product in &relevant {
        let Some(id) = product["id"].as_str() else { continue };
        let offers = match product_offers(client, &bearer, id).await {
            Ok(Some(offers)) => offers,
            Ok(None) => continue, // "No winners found": produto sem oferta ativa
            Err(e) => {
                warn!(product = id, "ML: falha ao ler ofertas: {e:#}");
                continue;
            }
        };
        items.extend(parse_product(product, &offers));
    }
    info!(target = %target.name, catalogo = products.len(), relevantes = relevant.len(), com_oferta = items.len(), "ML: busca no catálogo");
    Ok(items)
}

async fn product_offers(client: &wreq::Client, bearer: &str, id: &str) -> anyhow::Result<Option<Value>> {
    let resp = send_with_retry(|| client.get(format!("{API}/products/{id}/items")).header(AUTHORIZATION, bearer)).await?;
    if resp.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    Ok(Some(resp.error_for_status()?.json().await?))
}

fn parse_product(product: &Value, offers: &Value) -> Option<RawScrapedItem> {
    let id = product["id"].as_str()?;
    let (price_cash, offer) = offers["results"]
        .as_array()?
        .iter()
        .filter(|o| o["condition"].as_str() != Some("used"))
        .filter_map(|o| Some((json_decimal(&o["price"])?, o)))
        .min_by_key(|(price, _)| *price)?;
    // Sem o item_id, /p/{id} abre a oferta vencedora (buy box), que nem sempre é a mais barata.
    let url = match offer["item_id"].as_str() {
        Some(item) => format!("https://www.mercadolivre.com.br/p/{id}?pdp_filters=item_id:{item}"),
        None => format!("https://www.mercadolivre.com.br/p/{id}"),
    };
    Some(RawScrapedItem {
        external_id: id.to_string(),
        title: product["name"].as_str()?.to_string(),
        url,
        image_url: product["pictures"][0]["url"].as_str().map(String::from),
        seller_name: None,
        price_cash,
        price_installments: None,
    })
}

/// Access token válido; renova com o refresh token do banco (semeado por `ML_REFRESH_TOKEN` na 1ª vez).
/// O ML invalida o refresh token a cada uso, então o novo é gravado na mesma transação.
async fn access_token(pool: &PgPool, client: &wreq::Client) -> anyhow::Result<String> {
    let mut tx = pool.begin().await?;
    // FOR UPDATE serializa renovações concorrentes: a segunda espera e reaproveita o token novo.
    let row: Option<(String, Option<String>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT refresh_token, access_token, expires_at FROM oauth_tokens WHERE provider = 'mercado_livre' FOR UPDATE",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let refresh = match row {
        Some((_, Some(access), Some(exp))) if exp > Utc::now() + Duration::minutes(5) => return Ok(access),
        Some((refresh, ..)) => refresh,
        None => std::env::var("ML_REFRESH_TOKEN").context("ML_REFRESH_TOKEN não definida")?,
    };
    let client_id = std::env::var("ML_CLIENT_ID").context("ML_CLIENT_ID não definida")?;
    let secret = std::env::var("ML_CLIENT_SECRET").context("ML_CLIENT_SECRET não definida")?;
    let body = format!(
        "grant_type=refresh_token&client_id={}&client_secret={}&refresh_token={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(&secret),
        urlencoding::encode(&refresh)
    );
    let resp = client.post(format!("{API}/oauth/token")).header(CONTENT_TYPE, "application/x-www-form-urlencoded").body(body).send().await?;
    // Sem o corpo no erro: ele pode ecoar credenciais.
    anyhow::ensure!(resp.status().is_success(), "renovação do token ML falhou: HTTP {}", resp.status());
    let json: Value = resp.json().await?;
    let access = json["access_token"].as_str().context("resposta sem access_token")?;
    let new_refresh = json["refresh_token"].as_str().context("resposta sem refresh_token")?;
    let expires_in = json["expires_in"].as_i64().unwrap_or(21600);
    sqlx::query(
        "INSERT INTO oauth_tokens (provider, refresh_token, access_token, expires_at)
         VALUES ('mercado_livre', $1, $2, NOW() + make_interval(secs => $3))
         ON CONFLICT (provider) DO UPDATE SET refresh_token = EXCLUDED.refresh_token,
           access_token = EXCLUDED.access_token, expires_at = EXCLUDED.expires_at, updated_at = NOW()",
    )
    .bind(new_refresh)
    .bind(access)
    .bind(expires_in as f64)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    info!("token do Mercado Livre renovado");
    Ok(access.to_string())
}

/// `MLB-1234567` e `MLB1234567` viram o mesmo id; sem match, a URL sem query/fragmento.
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
    fn parse_product_pega_oferta_mais_barata() {
        let product = serde_json::json!({"id": "MLB28117648", "name": "Apple Mac mini M4 16GB", "pictures": [{"url": "https://http2.mlstatic.com/a.webp"}]});
        let offers = serde_json::json!({"results": [
            {"item_id": "MLB1", "price": 4700, "condition": "new"},
            {"item_id": "MLB2", "price": 4479.6, "condition": "new"},
            {"item_id": "MLB3", "price": 3900, "condition": "used"},
            {"price": null}
        ]});
        let it = parse_product(&product, &offers).unwrap();
        assert_eq!(it.external_id, "MLB28117648");
        assert_eq!(it.url, "https://www.mercadolivre.com.br/p/MLB28117648?pdp_filters=item_id:MLB2");
        assert_eq!(it.price_cash, Decimal::from_str("4479.6").unwrap(), "usado não entra");
        assert_eq!(it.image_url.as_deref(), Some("https://http2.mlstatic.com/a.webp"));
        assert!(parse_product(&product, &serde_json::json!({"results": []})).is_none(), "sem oferta não vira item");
    }

    #[test]
    fn external_id_formatos() {
        assert_eq!(external_id("https://x.com/MLB-1234567-mac-mini"), "MLB1234567");
        assert_eq!(external_id("https://x.com/p/MLB1234567?a=1"), "MLB1234567");
        assert_eq!(external_id("https://x.com/produto?a=1"), "https://x.com/produto");
    }
}
