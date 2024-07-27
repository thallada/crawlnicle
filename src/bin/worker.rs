use anyhow::Result;
use apalis::layers::tracing::TraceLayer;
use apalis::prelude::*;
use apalis::redis::RedisStorage;
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
    let redis_conn = apalis::redis::connect(config.redis_url.clone()).await?;
    let apalis_config = apalis::redis::Config::default();
    let apalis: RedisStorage<AsyncJob> = RedisStorage::new_with_config(redis_conn, apalis_config);

    Monitor::<TokioExecutor>::new()
        .register_with_count(2, {
            WorkerBuilder::new("worker")
                .layer(TraceLayer::new())
                .with_storage(apalis.clone())
                .build_fn(worker_fn)
        })
        .run()
        .await
        .unwrap();
    Ok(())
}
