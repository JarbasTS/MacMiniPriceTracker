//! Identidade vinda do Cloudflare Access: valida o JWT `Cf-Access-Jwt-Assertion` e só deixa admin escrever.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{config::AccessConfig, error::AppError, routes::AppState};

/// Quem está fazendo a requisição; vai nas extensions para o `/api/me`.
#[derive(Clone, Debug, Serialize)]
pub struct Viewer {
    pub email: Option<String>,
    pub admin: bool,
}

#[derive(Debug, Deserialize)]
pub struct Claims {
    pub email: String,
}

pub struct Auth {
    /// `None` = auth desligada (dev local): todo mundo é admin.
    cfg: Option<AccessConfig>,
    client: wreq::Client,
    keys: RwLock<Keys>,
}

#[derive(Default)]
struct Keys {
    by_kid: HashMap<String, DecodingKey>,
    fetched_at: Option<Instant>,
}

/// As chaves do Access giram; um `kid` novo dispara download, no máximo um por minuto.
const REFETCH_MIN: Duration = Duration::from_secs(60);

impl Auth {
    pub fn new(cfg: Option<AccessConfig>, client: wreq::Client) -> Self {
        Self { cfg, client, keys: RwLock::default() }
    }

    pub fn enabled(&self) -> bool {
        self.cfg.is_some()
    }

    pub async fn viewer(&self, headers: &HeaderMap) -> Result<Viewer, AppError> {
        let Some(cfg) = &self.cfg else {
            return Ok(Viewer { email: None, admin: true });
        };
        let token = headers.get("cf-access-jwt-assertion").and_then(|v| v.to_str().ok()).ok_or(AppError::Unauthorized)?;
        let kid = decode_header(token).ok().and_then(|h| h.kid).ok_or(AppError::Unauthorized)?;
        let key = self.key(cfg, &kid).await?.ok_or(AppError::Unauthorized)?;
        let claims = verify(token, &key, &cfg.aud, &cfg.team_domain).map_err(|e| {
            tracing::debug!("JWT do Access recusado: {e}");
            AppError::Unauthorized
        })?;
        let admin = is_admin(&cfg.admins, &claims.email);
        Ok(Viewer { email: Some(claims.email), admin })
    }

    async fn key(&self, cfg: &AccessConfig, kid: &str) -> Result<Option<DecodingKey>, AppError> {
        {
            let keys = self.keys.read().await;
            if let Some(k) = keys.by_kid.get(kid) {
                return Ok(Some(k.clone()));
            }
            if keys.fetched_at.is_some_and(|t| t.elapsed() < REFETCH_MIN) {
                return Ok(None);
            }
        }
        // Download sem lock: requisições com kid conhecido não esperam. Dois downloads simultâneos são inofensivos.
        // Se falhar, fetched_at não muda e a próxima requisição tenta de novo.
        let url = format!("{}/cdn-cgi/access/certs", cfg.team_domain);
        let set: JwkSet = async { anyhow::Ok(self.client.get(&url).send().await?.error_for_status()?.json().await?) }.await?;
        let by_kid: HashMap<String, DecodingKey> = set
            .keys
            .iter()
            .filter_map(|k| Some((k.common.key_id.clone()?, DecodingKey::from_jwk(k).ok()?)))
            .collect();
        let key = by_kid.get(kid).cloned();
        *self.keys.write().await = Keys { by_kid, fetched_at: Some(Instant::now()) };
        Ok(key)
    }
}

/// Assinatura RS256, audiência (o app no Access), issuer (o team domain) e validade.
pub fn verify(token: &str, key: &DecodingKey, aud: &str, iss: &str) -> jsonwebtoken::errors::Result<Claims> {
    let mut v = Validation::new(Algorithm::RS256);
    v.set_audience(&[aud]);
    v.set_issuer(&[iss]);
    Ok(decode::<Claims>(token, key, &v)?.claims)
}

fn is_admin(admins: &[String], email: &str) -> bool {
    admins.iter().any(|a| a.eq_ignore_ascii_case(email))
}

fn allowed(method: &Method, viewer: &Viewer) -> bool {
    viewer.admin || matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// 401 sem JWT válido; 403 para escrita de quem não é admin.
pub async fn guard(State(st): State<AppState>, mut req: Request, next: Next) -> Result<Response, AppError> {
    let viewer = st.auth.viewer(req.headers()).await?;
    if !allowed(req.method(), &viewer) {
        return Err(AppError::Forbidden);
    }
    req.extensions_mut().insert(viewer);
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;

    // Par gerado só para estes testes; não vale em lugar nenhum.
    const PRIV: &[u8] = include_bytes!("../testdata/jwt_test_only.key");
    const PUB: &[u8] = include_bytes!("../testdata/jwt_test_only.pub");
    const AUD: &str = "aud-do-app";
    const ISS: &str = "https://time.cloudflareaccess.com";

    fn token(aud: &str, iss: &str, exp_offset: i64) -> String {
        let exp = chrono::Utc::now().timestamp() + exp_offset;
        let claims = json!({ "email": "amigo@x.com", "aud": [aud], "iss": iss, "exp": exp });
        encode(&Header::new(Algorithm::RS256), &claims, &EncodingKey::from_rsa_pem(PRIV).unwrap()).unwrap()
    }

    fn key() -> DecodingKey {
        DecodingKey::from_rsa_pem(PUB).unwrap()
    }

    #[test]
    fn jwt_valido_passa() {
        let c = verify(&token(AUD, ISS, 3600), &key(), AUD, ISS).unwrap();
        assert_eq!(c.email, "amigo@x.com");
    }

    #[test]
    fn jwt_invalido_falha() {
        assert!(verify(&token("outro-app", ISS, 3600), &key(), AUD, ISS).is_err());
        assert!(verify(&token(AUD, "https://outro.cloudflareaccess.com", 3600), &key(), AUD, ISS).is_err());
        // Além da folga padrão de 60 s.
        assert!(verify(&token(AUD, ISS, -3600), &key(), AUD, ISS).is_err());
        assert!(verify("lixo", &key(), AUD, ISS).is_err());
    }

    #[test]
    fn so_admin_escreve() {
        let amigo = Viewer { email: Some("amigo@x.com".into()), admin: false };
        let dono = Viewer { email: Some("dono@x.com".into()), admin: true };
        assert!(allowed(&Method::GET, &amigo));
        assert!(!allowed(&Method::POST, &amigo));
        assert!(!allowed(&Method::DELETE, &amigo));
        assert!(allowed(&Method::POST, &dono));
        assert!(is_admin(&["Dono@X.com".into()], "dono@x.com"));
        assert!(!is_admin(&["dono@x.com".into()], "amigo@x.com"));
    }
}
