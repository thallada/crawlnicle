use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};

use anyhow::Result;
use async_redis_session::RedisSessionStore;
use axum::{
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use axum_login::{
    axum_sessions::SessionLayer,
    AuthLayer, PostgresStore, RequireAuthorizationLayer,
};
use bytes::Bytes;
use clap::Parser;
use dotenvy::dotenv;
use lettre::transport::smtp::authentication::Credentials;
use lettre::SmtpTransport;
use notify::Watcher;
use rand::Rng;
use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::watch::channel;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_livereload::LiveReloadLayer;
use tracing::debug;

use lib::actors::crawl_scheduler::CrawlSchedulerHandle;
use lib::actors::importer::ImporterHandle;
use lib::config::Config;
use lib::domain_locks::DomainLocks;
use lib::handlers;
use lib::log::init_tracing;
use lib::models::user::User;
use lib::state::AppState;
use lib::USER_AGENT;
use uuid::Uuid;

async fn serve(app: Router, addr: SocketAddr) -> Result<()> {
    debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn protected_handler(Extension(user): Extension<User>) -> impl IntoResponse {
    format!(
        "Logged in as: {}",
        user.name.unwrap_or_else(|| "No name".to_string())
    )
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

    let secret = rand::thread_rng().gen::<[u8; 64]>();

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;

    let session_store = RedisSessionStore::new(config.redis_url.clone())?;
    let session_layer = SessionLayer::new(session_store, &secret).with_secure(false);
    let user_store = PostgresStore::<User>::new(pool.clone())
        .with_query("select * from users where user_id = $1");
    let auth_layer = AuthLayer::new(user_store, &secret);

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

    let addr = format!("{}:{}", &config.host, &config.port).parse()?;
    let mut app = Router::new()
        .route("/protected", get(protected_handler))
        .route_layer(RequireAuthorizationLayer::<Uuid, User>::login())
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
        .layer(auth_layer)
        .layer(session_layer);

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
