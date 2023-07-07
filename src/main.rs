use std::{path::Path, net::SocketAddr};

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use clap::Parser;
use dotenvy::dotenv;
use notify::Watcher;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::watch::channel;
use tower::ServiceBuilder;
use tower_http::{trace::TraceLayer, services::ServeDir};
use tower_livereload::LiveReloadLayer;
use tracing::debug;

use lib::config::Config;
use lib::handlers;
use lib::log::init_tracing;
use lib::state::AppState;

async fn serve(app: Router, addr: SocketAddr) -> Result<()> {
    debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = Config::parse();

    let (log_sender, log_receiver) = channel::<Bytes>(Bytes::new());
    let _guards = init_tracing(&config, log_sender)?;

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
        .route("/feed/:id", get(handlers::feed::get))
        .route("/entry/:id", get(handlers::entry::get))
        .route("/log", get(handlers::log::get))
        .route("/log/stream", get(handlers::log::stream))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(AppState {
            pool,
            config,
            log_receiver,
        })
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    if cfg!(debug_assertions) {
        debug!("starting livereload");
        let livereload = LiveReloadLayer::new();
        let reloader = livereload.reloader();
        let mut watcher = notify::recommended_watcher(move |_| reloader.reload())?;
        watcher.watch(
            Path::new("static"),
            notify::RecursiveMode::Recursive,
        )?;
        app = app.layer(livereload);
        serve(app, addr).await?;
    } else {
        serve(app, addr).await?;
    }

    Ok(())
}
