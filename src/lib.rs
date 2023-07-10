pub mod actors;
pub mod config;
pub mod error;
pub mod handlers;
pub mod jobs;
pub mod log;
pub mod models;
pub mod partials;
pub mod state;
pub mod turbo_stream;
pub mod utils;
pub mod uuid;

pub const JS_BUNDLES: &str = include_str!("../static/js/manifest.txt");
pub const CSS_BUNDLES: &str = include_str!("../static/css/manifest.txt");
