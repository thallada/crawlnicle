use sqlx::PgPool;

use crate::models::feed::{create_feed, CreateFeed, Feed};
use crate::error::Result;

pub async fn add_feed(pool: PgPool, payload: CreateFeed) -> Result<Feed> {
    create_feed(pool, payload).await
}
