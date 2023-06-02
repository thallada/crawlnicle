use std::path::Path;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use notify::Watcher;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceBuilder;
use tower_livereload::LiveReloadLayer;
use tower_http::trace::TraceLayer;
use tracing::debug;

use lib::config;
use lib::handlers;
use lib::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = config::Config::new()?;

    tracing_subscriber::fmt::init();

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    let addr = format!("{}:{}", &config.host, &config.port).parse()?;
    let mut app = Router::new()
        .route("/api/v1/feeds", get(handlers::api::feeds::get))
        .route("/api/v1/feed", post(handlers::api::feed::post))
        .route("/api/v1/feed/:id", get(handlers::api::feed::get))
        .route("/api/v1/entries", get(handlers::api::entries::get))
        .route("/api/v1/entry", post(handlers::api::entry::post))
        .route("/api/v1/entry/:id", get(handlers::api::entry::get))
        .route("/", get(handlers::home::get))
        .route("/feeds", get(handlers::feeds::get))
        .route("/entry/:id", get(handlers::entry::get))
        .with_state(AppState {
            pool,
            config,
        })
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    #[cfg(debug_assertions)]
    {
        let livereload = LiveReloadLayer::new();
        let reloader = livereload.reloader();
        let mut watcher = notify::recommended_watcher(move |_| reloader.reload())?;
        watcher.watch(Path::new("target/debug/crawlnicle"), notify::RecursiveMode::Recursive)?;
        app = app.layer(livereload);
    }

    debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
