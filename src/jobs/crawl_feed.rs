use serde::{Deserialize, Serialize};

use crate::models::feed::Feed;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CrawlFeedJob {
    pub feed: Feed,
}
