use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};

use anyhow::Result;
use axum::{
    error_handling::HandleErrorLayer,
    response::Response,
    routing::{get, post},
    BoxError, Router,
};
use axum_login::{
    login_required,
    tower_sessions::{fred::prelude::*, Expiry, RedisStore, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use bytes::Bytes;
use clap::Parser;
use dotenvy::dotenv;
use http::StatusCode;
use lettre::transport::smtp::authentication::Credentials;
use lettre::SmtpTransport;
use notify::Watcher;
use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use time::Duration;
use tokio::sync::watch::channel;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_livereload::LiveReloadLayer;
use tracing::debug;

use lib::config::Config;
use lib::domain_locks::DomainLocks;
use lib::handlers;
use lib::log::init_tracing;
use lib::state::AppState;
use lib::USER_AGENT;
use lib::{actors::crawl_scheduler::CrawlSchedulerHandle, auth::Backend};
use lib::{actors::importer::ImporterHandle, htmx::not_htmx_predicate};

async fn serve(app: Router, addr: SocketAddr) -> Result<()> {
    debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
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
    let imports = Arc::new(Mutex::new(HashMap::new()));
    let domain_locks = DomainLocks::new();
    let client = Client::builder().user_agent(USER_AGENT).build()?;

    // TODO: not needed anymore?
    // let secret = config.session_secret.as_bytes();

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&config.database_url)
        .await?;

    let redis_config = RedisConfig::from_url(&config.redis_url)?;
    // TODO: https://github.com/maxcountryman/tower-sessions/issues/92
    // let redis_pool = RedisPool::new(redis_config, None, None, config.redis_pool_size)?;
    // redis_pool.connect();
    // redis_pool.wait_for_connect().await?;
    let redis_client = RedisClient::new(redis_config, None, None, None);
    redis_client.connect();
    redis_client.wait_for_connect().await?;

    let session_store = RedisStore::new(redis_client);
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(!cfg!(debug_assertions))
        .with_expiry(Expiry::OnInactivity(Duration::days(
            config.session_duration_days,
        )));

    let backend = Backend::new(pool.clone());
    let auth_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            StatusCode::BAD_REQUEST
        }))
        .layer(AuthManagerLayerBuilder::new(backend, session_layer).build());

    let creds = Credentials::new(config.smtp_user.clone(), config.smtp_password.clone());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay(&config.smtp_server)
        .unwrap()
        .credentials(creds)
        .build();

    sqlx::migrate!().run(&pool).await?;

    let crawl_scheduler = CrawlSchedulerHandle::new(
        pool.clone(),
        client.clone(),
        domain_locks.clone(),
        config.content_dir.clone(),
        crawls.clone(),
    );
    let _ = crawl_scheduler.bootstrap().await;
    let importer = ImporterHandle::new(pool.clone(), crawl_scheduler.clone(), imports.clone());

    let ip_source_extension = config.ip_source.0.clone().into_extension();

    let addr = format!("{}:{}", &config.host, &config.port).parse()?;
    let mut app = Router::new()
        .route("/account", get(handlers::account::get))
        .route_layer(login_required!(Backend, login_url = "/login"))
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
        .route("/entries", get(handlers::entries::get))
        .route("/entry/:id", get(handlers::entry::get))
        .route("/log", get(handlers::log::get))
        .route("/log/stream", get(handlers::log::stream))
        .route("/import/opml", post(handlers::import::opml))
        .route("/import/:id/stream", get(handlers::import::stream))
        .route("/login", get(handlers::login::get))
        .route("/login", post(handlers::login::post))
        .route("/logout", get(handlers::logout::get))
        .route("/register", get(handlers::register::get))
        .route("/register", post(handlers::register::post))
        .route("/confirm-email", get(handlers::confirm_email::get))
        .route("/confirm-email", post(handlers::confirm_email::post))
        .route("/forgot-password", get(handlers::forgot_password::get))
        .route("/forgot-password", post(handlers::forgot_password::post))
        .route("/reset-password", get(handlers::reset_password::get))
        .route("/reset-password", post(handlers::reset_password::post))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(AppState {
            pool,
            config,
            log_receiver,
            crawls,
            domain_locks,
            client,
            crawl_scheduler,
            importer,
            imports,
            mailer,
        })
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .layer(auth_service)
        .layer(ip_source_extension);

    if cfg!(debug_assertions) {
        debug!("starting livereload");
        let livereload = LiveReloadLayer::new().request_predicate(not_htmx_predicate);
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
