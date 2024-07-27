use anyhow::{anyhow, Result};
use apalis::cron::{CronStream, Schedule};
use apalis::layers::retry::{RetryLayer, RetryPolicy};
use apalis::layers::tracing::TraceLayer;
use apalis::prelude::*;
use apalis::redis::RedisStorage;
use chrono::{DateTime, Utc};
use clap::Parser;
use lib::actors::crawl_scheduler::CrawlSchedulerError;
use lib::jobs::AsyncJob;
use lib::models::feed::{Feed, GetFeedsOptions};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceBuilder;
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

impl Job for Crawl {
    const NAME: &'static str = "apalis::Crawl";
}

struct State {
    pool: PgPool,
    apalis: RedisStorage<AsyncJob>,
}

#[instrument(skip_all)]
pub async fn crawl_fn(job: Crawl, state: Data<Arc<State>>) -> Result<()> {
    tracing::info!(job = ?job, "crawl");
    let mut apalis = (state.apalis).clone();
    let mut options = GetFeedsOptions::default();
    loop {
        info!("fetching feeds before: {:?}", options.before);
        let feeds = match Feed::get_all(&state.pool, &options).await {
            Err(err) => {
                return Err(anyhow!(err));
            }
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
                .await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let config = Config::parse();
    let _guard = init_worker_tracing()?;

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&config.database_url)
        .await?;

    // TODO: use redis_pool from above instead of making a new connection
    // See: https://github.com/geofmureithi/apalis/issues/290
    let redis_conn = apalis::redis::connect(config.redis_url.clone()).await?;
    let apalis_config = apalis::redis::Config::default();
    let mut apalis: RedisStorage<AsyncJob> =
        RedisStorage::new_with_config(redis_conn, apalis_config);

    let schedule = Schedule::from_str("0 * * * * *").unwrap();
    // let service = ServiceBuilder::new()
    //     .layer(RetryLayer::new(RetryPolicy::default()))
    //     .layer(TraceLayer::new())
    //     .service(service_fn(crawl_fn));

    let worker = WorkerBuilder::new("crawler")
        .stream(CronStream::new(schedule).into_stream())
        .layer(RetryLayer::new(RetryPolicy::default()))
        .layer(TraceLayer::new())
        .data(Arc::new(State { pool, apalis }))
        .build_fn(crawl_fn);

    Monitor::<TokioExecutor>::new()
        .register(worker)
        .run()
        .await
        .unwrap();

    Ok(())
}
