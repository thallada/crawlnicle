use clap::Parser;

#[derive(Parser, Clone, Debug)]
pub struct Config {
    #[clap(long, env)]
    pub database_url: String,
    #[clap(long, env)]
    pub database_max_connections: u32,
    #[clap(long, env)]
    pub host: String,
    #[clap(long, env)]
    pub port: u16,
    #[clap(long, env)]
    pub title: String,
    #[clap(long, env)]
    pub max_mem_log_size: usize,
}
