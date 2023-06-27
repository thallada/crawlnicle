pub mod config;
pub mod error;
pub mod handlers;
pub mod jobs;
pub mod log;
pub mod models;
pub mod partials;
pub mod state;
pub mod utils;
pub mod uuid;

pub const JS_BUNDLES: &'static str = include_str!("../static/js/js_bundles.txt");
