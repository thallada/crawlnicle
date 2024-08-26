use apalis::prelude::*;
use apalis_redis::RedisStorage;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info, instrument};

mod crawl_feed;

pub use crawl_feed::CrawlFeedJob;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AsyncJob {
    HelloWorld(String),
    CrawlFeed(CrawlFeedJob),
}

#[derive(Debug, Error)]
pub enum AsyncJobError {
    #[error("error executing job")]
    JobError(#[from] anyhow::Error),
}

#[instrument(skip(job, worker_id), fields(worker_id = %worker_id))]
pub async fn handle_async_job(
    job: AsyncJob,
    worker_id: WorkerId,
    // TODO: add task_id to tracing instrumentation context
    // it casuses a panic in 0.6.0 currently, see: https://github.com/geofmureithi/apalis/issues/398
    // task_id: Data<TaskId>,
) -> Result<(), AsyncJobError> {
    let result = match job {
        AsyncJob::HelloWorld(name) => {
            info!("Hello, {}!", name);
            Ok(())
        }
        AsyncJob::CrawlFeed(job) => crawl_feed::crawl_feed(job).await,
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
