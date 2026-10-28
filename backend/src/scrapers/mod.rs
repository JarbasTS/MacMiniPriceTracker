pub mod amazon_br;
pub mod apple_br;
pub mod buscape;
pub mod dynamic_css;
pub mod kabum_api;
pub mod mercado_livre;
pub mod runner;
pub mod sanitizer;

use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use sqlx::PgPool;
use rust_decimal::Decimal;
use scraper::{ElementRef, Selector};
use tracing::debug;
use wreq::header::{HeaderMap, HeaderValue, ACCEPT_LANGUAGE, RETRY_AFTER};
use wreq::StatusCode;
use wreq_util::Emulation;

use crate::models::{RawScrapedItem, SearchTarget, Store};
pub use amazon_br::AmazonBrScraper;
pub use apple_br::AppleBrScraper;
pub use buscape::BuscapeScraper;
pub use dynamic_css::DynamicCssScraper;
pub use kabum_api::KabumApiScraper;
pub use mercado_livre::MercadoLivreScraper;

#[async_trait]
pub trait StoreScraper: Send + Sync {
    async fn scrape(&self, store: &Store, target: &SearchTarget, client: &wreq::Client, pool: &PgPool) -> anyhow::Result<Vec<RawScrapedItem>>;
}

pub fn scraper_for(scraper_type: &str) -> Option<&'static dyn StoreScraper> {
    match scraper_type {
        "mercado_livre" => Some(&MercadoLivreScraper),
        "amazon_br" => Some(&AmazonBrScraper),
        "dynamic_css" => Some(&DynamicCssScraper),
        "kabum_api" => Some(&KabumApiScraper),
        "buscape" => Some(&BuscapeScraper),
        "apple_br" => Some(&AppleBrScraper),
        _ => None,
    }
}

/// Cliente com fingerprint TLS/HTTP2 do Chrome. O UA e os headers vêm do perfil; um UA próprio
/// destoaria do fingerprint. Só o idioma é sobrescrito, para receber a página em pt-BR.
pub fn http_client() -> anyhow::Result<wreq::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("pt-BR,pt;q=0.9"));
    Ok(wreq::Client::builder()
        .emulation(Emulation::Chrome149)
        .cookie_store(true)
        .gzip(true)
        .brotli(true)
        .timeout(Duration::from_secs(20))
        .default_headers(headers)
        .build()?)
}

pub(crate) fn search_url(store: &Store, query: &str) -> String {
    store.search_url_template.replace("{query}", &urlencoding::encode(query))
}

pub(crate) async fn fetch_html(client: &wreq::Client, url: &str) -> anyhow::Result<String> {
    Ok(send_with_retry(|| client.get(url)).await?.error_for_status()?.text().await?)
}

const MAX_RETRIES: u32 = 2;

/// Envia e repete em 429/503 (a Amazon usa 503 como soft-block). `build` recria a requisição a cada tentativa.
/// Devolve a última resposta, qualquer que seja o status: quem chama decide o que é erro.
// shortcut: retry só para 429/503 e no máximo 2 vezes; se a loja continuar bloqueando, aumentar scrape_interval_minutes em vez de insistir.
pub(crate) async fn send_with_retry(build: impl Fn() -> wreq::RequestBuilder) -> anyhow::Result<wreq::Response> {
    let mut attempt = 0;
    loop {
        let resp = build().send().await?;
        let retry_after = resp.headers().get(RETRY_AFTER).and_then(|v| v.to_str().ok());
        let Some(wait) = retry_delay(resp.status(), retry_after, attempt) else {
            return Ok(resp);
        };
        debug!(status = %resp.status(), ?wait, attempt = attempt + 1, "nova tentativa");
        tokio::time::sleep(wait).await;
        attempt += 1;
    }
}

/// Espera antes da próxima tentativa: `Retry-After` em segundos (até 30 s) ou 5 s × tentativa; `None` = não repetir.
fn retry_delay(status: StatusCode, retry_after: Option<&str>, attempt: u32) -> Option<Duration> {
    let retryable = status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::SERVICE_UNAVAILABLE;
    if !retryable || attempt >= MAX_RETRIES {
        return None;
    }
    let secs = retry_after.and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(5 * u64::from(attempt + 1));
    Some(Duration::from_secs(secs.min(30)))
}

/// Seletores fixos do código; um erro aqui é bug coberto pelos testes de cada scraper.
pub(crate) fn sel(css: &str) -> Selector {
    Selector::parse(css).expect("seletor estático inválido")
}

/// Texto do elemento com espaços colapsados.
pub(crate) fn text(el: ElementRef) -> String {
    el.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Só http(s): o href vem de HTML de terceiros e vira link clicável (`javascript:` rodaria no dashboard).
pub(crate) fn abs_url(base: &str, href: &str) -> Option<String> {
    let u = url::Url::parse(base).ok()?.join(href).ok()?;
    matches!(u.scheme(), "http" | "https").then(|| u.to_string())
}

/// Número JSON (`5499.9`) ou string numérica (`"5499.90"`) → Decimal exato, sem passar por f64.
pub(crate) fn json_decimal(v: &serde_json::Value) -> Option<Decimal> {
    let s = v.as_str().map(String::from).or_else(|| v.as_number().map(|n| n.to_string()))?;
    Decimal::from_str(&s).ok()
}

/// `data-src` primeiro: lojas fazem lazy-load com placeholder em `src`.
pub(crate) fn img_src(el: ElementRef) -> Option<String> {
    let v = el.value();
    v.attr("data-src").or(v.attr("src")).filter(|s| s.starts_with("http")).map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abs_url_so_http() {
        let base = "https://loja.com.br";
        assert_eq!(abs_url(base, "/p/1").as_deref(), Some("https://loja.com.br/p/1"));
        assert_eq!(abs_url(base, "javascript:alert(1)"), None);
        assert_eq!(abs_url(base, "data:text/html,x"), None);
    }

    #[test]
    fn retry_delay_regras() {
        let s503 = StatusCode::SERVICE_UNAVAILABLE;
        assert_eq!(retry_delay(s503, None, 0), Some(Duration::from_secs(5)));
        assert_eq!(retry_delay(s503, None, 1), Some(Duration::from_secs(10)));
        assert_eq!(retry_delay(s503, None, 2), None, "no máximo 2 novas tentativas");
        assert_eq!(retry_delay(StatusCode::TOO_MANY_REQUESTS, Some("7"), 0), Some(Duration::from_secs(7)));
        assert_eq!(retry_delay(s503, Some("120"), 0), Some(Duration::from_secs(30)), "Retry-After limitado a 30 s");
        assert_eq!(retry_delay(s503, Some("Wed, 21 Oct 2026 07:28:00 GMT"), 0), Some(Duration::from_secs(5)), "data HTTP cai no padrão");
        assert_eq!(retry_delay(StatusCode::NOT_FOUND, None, 0), None);
        assert_eq!(retry_delay(StatusCode::FORBIDDEN, None, 0), None);
    }
}
