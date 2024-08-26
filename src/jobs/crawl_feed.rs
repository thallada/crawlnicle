use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::models::feed::Feed;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CrawlFeedJob {
    pub feed: Feed,
}

#[instrument(skip_all, fields(feed_id = %feed.feed_id))]
pub async fn crawl_feed(CrawlFeedJob { feed }: CrawlFeedJob) -> Result<()> {
    info!("Crawling feed: {:?}", feed.feed_id);
    Err(anyhow::anyhow!("Not implemented"))
}
