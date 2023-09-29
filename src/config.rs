use clap::Parser;
use lettre::message::Mailbox;
use url::Url;

#[derive(Parser, Clone, Debug)]
pub struct Config {
    #[clap(long, env)]
    pub database_url: String,
    #[clap(long, env)]
    pub database_max_connections: u32,
    #[clap(long, env)]
    pub redis_url: String,
    #[clap(long, env)]
    pub host: String,
    #[clap(long, env)]
    pub port: u16,
    #[clap(long, env)]
    pub public_url: Url,
    #[clap(long, env)]
    pub title: String,
    #[clap(long, env)]
    pub max_mem_log_size: usize,
    #[clap(long, env)]
    pub content_dir: String,
    #[clap(long, env)]
    pub smtp_server: String,
    #[clap(long, env)]
    pub smtp_user: String,
    #[clap(long, env)]
    pub smtp_password: String,
    #[clap(long, env)]
    pub email_from: Mailbox,
}
