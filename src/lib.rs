pub mod actors;
pub mod api_response;
pub mod auth;
pub mod config;
pub mod domain_locks;
pub mod error;
pub mod handlers;
pub mod headers;
pub mod htmx;
pub mod log;
pub mod models;
pub mod partials;
pub mod state;
pub mod utils;
pub mod uuid;

pub const USER_AGENT: &str = "crawlnicle/0.1.0";
pub const JS_MANIFEST: &str = include_str!("../static/js/manifest.txt");
pub const CSS_MANIFEST: &str = include_str!("../static/css/manifest.txt");
