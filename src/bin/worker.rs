use anyhow::Result;
use apalis::layers::retry::RetryPolicy;
use apalis::layers::tracing::TraceLayer;
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use clap::Parser;
use dotenvy::dotenv;
use fred::prelude::*;
use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use tower::retry::RetryLayer;

use lib::config::Config;
use lib::domain_request_limiter::DomainRequestLimiter;
use lib::jobs::{handle_async_job, AsyncJob};
use lib::log::init_worker_tracing;
use lib::USER_AGENT;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let config = Config::parse();
    let _guard = init_worker_tracing()?;
    // TODO: create connection from redis_pool for each job instead using a single connection
    // See: https://github.com/geofmureithi/apalis/issues/290
    let redis_conn = apalis_redis::connect(config.redis_url.clone()).await?;
    let apalis_config = apalis_redis::Config::default();
    let apalis_storage: RedisStorage<AsyncJob> =
        RedisStorage::new_with_config(redis_conn, apalis_config);

    let redis_config = RedisConfig::from_url(&config.redis_url)?;
    let redis_pool = RedisPool::new(redis_config, None, None, None, 5)?;
    redis_pool.init().await?;
    let domain_request_limiter = DomainRequestLimiter::new(redis_pool.clone(), 10, 5, 100, 0.5);

    let http_client = Client::builder().user_agent(USER_AGENT).build()?;
    let db = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&config.database_url)
        .await?;

    Monitor::<TokioExecutor>::new()
        .register_with_count(2, {
            WorkerBuilder::new("worker")
                .layer(RetryLayer::new(RetryPolicy::default()))
                .layer(TraceLayer::new())
                .data(http_client)
                .data(db)
                .data(domain_request_limiter)
                .data(config)
                .data(apalis_storage.clone())
                .data(redis_pool)
                .backend(apalis_storage)
                .build_fn(handle_async_job)
        })
        .run()
        .await
        .unwrap();
    Ok(())
}
