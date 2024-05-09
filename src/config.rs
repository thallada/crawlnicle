use std::str::FromStr;

use axum_client_ip::SecureClientIpSource;
use clap::Parser;
use lettre::message::Mailbox;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize, Clone)]
pub struct IpSource(pub SecureClientIpSource);

impl FromStr for IpSource {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // SourceClientIpSource doesn't implement FromStr itself, so I have to implement it on this
        // wrapping newtype. See https://github.com/imbolc/axum-client-ip/issues/11
        let inner = match s {
            "RightmostForwarded" => SecureClientIpSource::RightmostForwarded,
            "RightmostXForwardedFor" => SecureClientIpSource::RightmostXForwardedFor,
            "XRealIp" => SecureClientIpSource::XRealIp,
            "FlyClientIp" => SecureClientIpSource::FlyClientIp,
            "TrueClientIp" => SecureClientIpSource::TrueClientIp,
            "CfConnectingIp" => SecureClientIpSource::CfConnectingIp,
            "ConnectInfo" => SecureClientIpSource::ConnectInfo,
            _ => return Err("Unknown variant"),
        };
        Ok(Self(inner))
    }
}

#[derive(Parser, Clone, Debug)]
pub struct Config {
    #[clap(long, env)]
    pub database_url: String,
    #[clap(long, env, default_value = "5")]
    pub database_max_connections: u32,
    #[clap(long, env, default_value = "redis://localhost")]
    pub redis_url: String,
    #[clap(long, env, default_value = "5")]
    pub redis_pool_size: usize,
    #[clap(long, env, default_value = "127.0.0.1")]
    pub host: String,
    #[clap(long, env, default_value = "3000")]
    pub port: u16,
    #[clap(long, env, default_value = "http://localhost:3000")]
    pub public_url: Url,
    #[clap(long, env, default_value = "crawlnicle")]
    pub title: String,
    #[clap(long, env, default_value = "1000000")]
    pub max_mem_log_size: usize,
    #[clap(long, env, default_value = "./content")]
    pub content_dir: String,
    #[clap(long, env)]
    pub smtp_server: String,
    #[clap(long, env)]
    pub smtp_user: String,
    #[clap(long, env)]
    pub smtp_password: String,
    #[clap(long, env, default_value = "crawlnicle <no-reply@mail.crawlnicle.com>")]
    pub email_from: Mailbox,
    #[clap(long, env)]
    pub session_secret: String, // base64 encoded
    #[clap(long, env, default_value = "ConnectInfo")]
    pub ip_source: IpSource,
    #[clap(long, env, default_value = "100")]
    pub session_duration_days: i64,
}
