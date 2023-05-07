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

mod error;
mod handlers;
mod models;

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
        .route("/items", get(handlers::items::get))
        .route("/item", post(handlers::item::post))
        .route("/item/:id", get(handlers::item::get))
        .with_state(pool)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let addr = (env::var("HOST")? + ":" + &env::var("PORT")?).parse()?;
    debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
