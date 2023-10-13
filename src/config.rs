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
    #[clap(long, env)]
    pub session_secret: String,
    #[clap(long, env)]
    pub ip_source: IpSource,
}
