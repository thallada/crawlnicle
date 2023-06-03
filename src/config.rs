use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub database_max_connections: u32,
    pub host: String,
    pub port: u16,
    pub title: String,
    pub max_mem_log_size: usize,
}

impl Config {
    pub fn new() -> Result<Config> {
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
        let database_max_connections = std::env::var("DATABASE_MAX_CONNECTIONS").context("DATABASE_MAX_CONNECTIONS not set")?.parse()?;
        let host = std::env::var("HOST").context("HOST not set")?;
        let port = std::env::var("PORT").context("PORT not set")?.parse()?;
        let title = std::env::var("TITLE").context("TITLE not set")?;
        let max_mem_log_size = std::env::var("MAX_MEM_LOG_SIZE").context("MAX_MEM_LOG_SIZE not set")?.parse()?;

        Ok(Config {
            database_url,
            database_max_connections,
            host,
            port,
            title,
            max_mem_log_size,
        })
    }
}
