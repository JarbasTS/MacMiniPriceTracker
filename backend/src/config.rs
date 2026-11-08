use anyhow::Context;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    /// `None` = sem Cloudflare Access na frente (dev local).
    pub access: Option<AccessConfig>,
}

#[derive(Clone, Debug)]
pub struct AccessConfig {
    /// Ex.: `https://<time>.cloudflareaccess.com`; também é o `iss` do JWT.
    pub team_domain: String,
    /// AUD tag do app no Access.
    pub aud: String,
    pub admins: Vec<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL não definida")?;
        let port = match std::env::var("PORT") {
            Ok(p) => p.parse().context("PORT inválida")?,
            Err(_) => 8080,
        };
        Ok(Self { database_url, port, access: access_from_env()? })
    }
}

fn access_from_env() -> anyhow::Result<Option<AccessConfig>> {
    // Vazia conta como ausente: o .env.example traz as chaves sem valor.
    let var = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    match (var("CF_ACCESS_TEAM_DOMAIN"), var("CF_ACCESS_AUD"), var("ADMIN_EMAILS")) {
        (None, None, None) => Ok(None),
        (Some(team), Some(aud), Some(admins)) => {
            let admins: Vec<String> = admins.split(',').map(str::trim).filter(|a| !a.is_empty()).map(String::from).collect();
            anyhow::ensure!(!admins.is_empty(), "ADMIN_EMAILS sem nenhum e-mail");
            // Sem o esquema o iss do JWT nunca bate e todo mundo toma 401.
            anyhow::ensure!(team.starts_with("https://"), "CF_ACCESS_TEAM_DOMAIN deve começar com https://");
            Ok(Some(AccessConfig { team_domain: team.trim_end_matches('/').to_string(), aud, admins }))
        }
        // Meia configuração abriria a escrita para todo mundo sem ninguém perceber.
        _ => anyhow::bail!("defina CF_ACCESS_TEAM_DOMAIN, CF_ACCESS_AUD e ADMIN_EMAILS juntas, ou nenhuma"),
    }
}
