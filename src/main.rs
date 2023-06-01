use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use notify::Watcher;
use sqlx::postgres::PgPoolOptions;
use std::{env, path::Path};
use tower::ServiceBuilder;
use tower_livereload::LiveReloadLayer;
use tower_http::trace::TraceLayer;
use tracing::debug;

use lib::handlers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let pool = PgPoolOptions::new()
        .max_connections(env::var("DATABASE_MAX_CONNECTIONS")?.parse()?)
        .connect(&env::var("DATABASE_URL")?)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    let app = Router::new()
        .route("/api/v1/feeds", get(handlers::api::feeds::get))
        .route("/api/v1/feed", post(handlers::api::feed::post))
        .route("/api/v1/feed/:id", get(handlers::api::feed::get))
        .route("/api/v1/entries", get(handlers::api::entries::get))
        .route("/api/v1/entry", post(handlers::api::entry::post))
        .route("/api/v1/entry/:id", get(handlers::api::entry::get))
        .route("/", get(handlers::home::get))
        .with_state(pool)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();
    let mut watcher = notify::recommended_watcher(move |_| reloader.reload())?;
    watcher.watch(Path::new("target/debug/crawlnicle"), notify::RecursiveMode::Recursive)?;
    let app = app.layer(livereload);

    let addr = (env::var("HOST")? + ":" + &env::var("PORT")?).parse()?;
    debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
