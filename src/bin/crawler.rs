use apalis::layers::retry::{RetryLayer, RetryPolicy};
use apalis::layers::tracing::TraceLayer;
use apalis::prelude::*;
use apalis_cron::{CronStream, Schedule};
use apalis_redis::RedisStorage;
use chrono::{DateTime, Utc};
use clap::Parser;
use lib::jobs::AsyncJob;
use lib::models::feed::{Feed, GetFeedsOptions};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, instrument};

use dotenvy::dotenv;
use lib::config::Config;
use lib::log::init_worker_tracing;

#[derive(Default, Debug, Clone)]
struct Crawl(DateTime<Utc>);

impl From<DateTime<Utc>> for Crawl {
    fn from(t: DateTime<Utc>) -> Self {
        Crawl(t)
    }
}

#[derive(Debug, Error)]
enum CrawlError {
    #[error("error fetching feeds")]
    FetchFeedsError(#[from] sqlx::Error),
    #[error("error queueing crawl feed job")]
    QueueJobError(String),
}

#[derive(Clone)]
struct State {
    pool: PgPool,
    apalis: RedisStorage<AsyncJob>,
}

#[instrument(skip_all)]
pub async fn crawl_fn(job: Crawl, state: Data<Arc<State>>) -> Result<(), CrawlError> {
    tracing::info!(job = ?job, "crawl");
    let mut apalis = (state.apalis).clone();
    let mut options = GetFeedsOptions::default();
    loop {
        info!("fetching feeds before: {:?}", options.before);
        // TODO: filter to feeds where:
        // now >= feed.last_crawled_at + feed.crawl_interval_minutes
        // may need more indices...
        let feeds = match Feed::get_all(&state.pool, &options).await {
            Err(err) => return Err(CrawlError::FetchFeedsError(err)),
            Ok(feeds) if feeds.is_empty() => {
                info!("no more feeds found");
                break;
            }
            Ok(feeds) => feeds,
        };
        info!("found {} feeds", feeds.len());
        options.before = feeds.last().map(|f| f.created_at);

        for feed in feeds.into_iter() {
            // self.spawn_crawler_loop(feed, respond_to.clone());
            apalis
                .push(AsyncJob::HelloWorld(feed.feed_id.to_string()))
                .await
                .map_err(|err| CrawlError::QueueJobError(err.to_string()))?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let config = Config::parse();
    let _guard = init_worker_tracing()?;

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&config.database_url)
        .await?;

    // TODO: create connection from redis_pool for each job instead using a single connection
    // See: https://github.com/geofmureithi/apalis/issues/290
    let redis_conn = apalis_redis::connect(config.redis_url.clone()).await?;
    let apalis_config = apalis_redis::Config::default();
    let apalis_storage = RedisStorage::new_with_config(redis_conn, apalis_config);

    let state = Arc::new(State {
        pool,
        apalis: apalis_storage.clone(),
    });

    let schedule = Schedule::from_str("0 * * * * *").unwrap();

    let worker = WorkerBuilder::new("crawler")
        .layer(RetryLayer::new(RetryPolicy::default()))
        .layer(TraceLayer::new())
        .data(state)
        .backend(CronStream::new(schedule))
        .build_fn(crawl_fn);

    Monitor::<TokioExecutor>::new()
        .register(worker)
        .run()
        .await
        .unwrap();

    Ok(())
}
