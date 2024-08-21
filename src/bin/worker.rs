use anyhow::Result;
use apalis::layers::tracing::TraceLayer;
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use clap::Parser;

use dotenvy::dotenv;
use lib::config::Config;
use lib::jobs::AsyncJob;
use lib::log::init_worker_tracing;

pub async fn worker_fn(job: AsyncJob) {
    tracing::info!(job = ?job, "Hello, world!");
}

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

    Monitor::<TokioExecutor>::new()
        .register_with_count(2, {
            WorkerBuilder::new("worker")
                .layer(TraceLayer::new())
                .backend(apalis_storage)
                .build_fn(worker_fn)
        })
        .run()
        .await
        .unwrap();
    Ok(())
}
