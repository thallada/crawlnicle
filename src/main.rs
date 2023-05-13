use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tower::ServiceBuilder;
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
        .route("/v1/feeds", get(handlers::feeds::get))
        .route("/v1/feed", post(handlers::feed::post))
        .route("/v1/feed/:id", get(handlers::feed::get))
        .route("/v1/entries", get(handlers::entries::get))
        .route("/v1/entry", post(handlers::entry::post))
        .route("/v1/entry/:id", get(handlers::entry::get))
        .with_state(pool)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let addr = (env::var("HOST")? + ":" + &env::var("PORT")?).parse()?;
    debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
