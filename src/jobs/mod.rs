use apalis::prelude::*;
use apalis_redis::RedisStorage;
use fred::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use tracing::{error, info, instrument};

mod crawl_entry;
mod crawl_feed;

pub use crawl_entry::CrawlEntryJob;
pub use crawl_feed::CrawlFeedJob;

use crate::{config::Config, domain_request_limiter::DomainRequestLimiter};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AsyncJob {
    HelloWorld(String),
    CrawlFeed(CrawlFeedJob),
    CrawlEntry(CrawlEntryJob),
}

#[derive(Debug, Error)]
pub enum AsyncJobError {
    #[error("error executing job")]
    JobError(#[from] anyhow::Error),
}

#[instrument(skip_all, fields(worker_id = ?worker_id, task_id = ?task_id))]
pub async fn handle_async_job(
    job: AsyncJob,
    worker_id: Data<WorkerId>,
    task_id: Data<TaskId>,
    http_client: Data<Client>,
    db: Data<PgPool>,
    domain_request_limiter: Data<DomainRequestLimiter>,
    config: Data<Config>,
    apalis: Data<RedisStorage<AsyncJob>>,
    redis: Data<RedisPool>,
) -> Result<(), AsyncJobError> {
    let result = match job {
        AsyncJob::HelloWorld(name) => {
            info!("Hello, {}!", name);
            Ok(())
        }
        AsyncJob::CrawlFeed(job) => {
            crawl_feed::crawl_feed(job, http_client, db, domain_request_limiter, apalis, redis)
                .await
        }
        AsyncJob::CrawlEntry(job) => {
            crawl_entry::crawl_entry(job, http_client, db, domain_request_limiter, config, redis)
                .await
        }
    };

    match result {
        Ok(_) => info!("Job completed successfully"),
        Err(err) => {
            error!("Job failed: {err:?}");
            return Err(AsyncJobError::JobError(err));
        }
    };
    Ok(())
}
