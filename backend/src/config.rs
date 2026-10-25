use anyhow::Context;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL não definida")?;
        let port = match std::env::var("PORT") {
            Ok(p) => p.parse().context("PORT inválida")?,
            Err(_) => 8080,
        };
        Ok(Self { database_url, port })
    }
}
