use std::{
    collections::HashMap,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use chrono::{Duration, Utc};
use clap::Parser;
use dotenvy::dotenv;
use notify::Watcher;
use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::watch::channel;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_livereload::LiveReloadLayer;
use tracing::{debug, info};

use lib::handlers;
use lib::log::init_tracing;
use lib::state::AppState;
use lib::{actors::feed_crawler::FeedCrawlerHandle, config::Config, models::feed::Feed};
use lib::{domain_locks::DomainLocks, models::feed::GetFeedsOptions};

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

    let crawls = Arc::new(Mutex::new(HashMap::new()));
    let domain_locks = DomainLocks::new();

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
        .route("/feed", post(handlers::feed::post))
        .route("/feed/:id", get(handlers::feed::get))
        .route("/feed/:id/stream", get(handlers::feed::stream))
        .route("/feed/:id/delete", post(handlers::feed::delete))
        .route("/entry/:id", get(handlers::entry::get))
        .route("/log", get(handlers::log::get))
        .route("/log/stream", get(handlers::log::stream))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(AppState {
            pool: pool.clone(),
            config: config.clone(),
            log_receiver,
            crawls,
            domain_locks: domain_locks.clone(),
        })
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    info!("starting crawlers");
    let mut options = GetFeedsOptions::default();
    loop {
        let feeds = Feed::get_all(&pool, options.clone()).await?;
        if feeds.is_empty() {
            break;
        }
        for feed in feeds.iter() {
            let client = Client::new(); // TODO: store in state and reuse
            if let Some(last_crawled_at) = feed.last_crawled_at {
                if last_crawled_at
                    >= Utc::now() - Duration::minutes(feed.crawl_interval_minutes.into())
                {
                    continue;
                }
            }
            let feed_crawler = FeedCrawlerHandle::new(
                pool.clone(),
                client.clone(),
                domain_locks.clone(),
                config.content_dir.clone(),
            );
            let _ = feed_crawler.crawl(feed.feed_id).await;
        }
        options.before = feeds.last().map(|f| f.created_at);
    }
    info!("done starting crawlers");

    if cfg!(debug_assertions) {
        debug!("starting livereload");
        let livereload = LiveReloadLayer::new();
        let reloader = livereload.reloader();
        let mut watcher = notify::recommended_watcher(move |_| reloader.reload())?;
        watcher.watch(Path::new("static"), notify::RecursiveMode::Recursive)?;
        app = app.layer(livereload);
        serve(app, addr).await?;
    } else {
        serve(app, addr).await?;
    }

    Ok(())
}
