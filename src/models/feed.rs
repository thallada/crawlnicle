use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "feed_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum FeedType {
    Atom,
    Rss,
}

impl FromStr for FeedType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "atom" => Ok(FeedType::Atom),
            "rss" => Ok(FeedType::Rss),
            _ => Err(format!("invalid feed type: {}", s)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Feed {
    pub feed_id: Uuid,
    pub title: Option<String>,
    pub url: String,
    #[serde(rename = "type")]
    pub feed_type: FeedType,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateFeed {
    #[validate(length(max = 255))]
    pub title: Option<String>,
    #[validate(url)]
    pub url: String,
    #[serde(rename = "type")]
    pub feed_type: FeedType,
    #[validate(length(max = 524288))]
    pub description: Option<String>,
}

pub async fn get_feed(pool: &PgPool, feed_id: Uuid) -> Result<Feed> {
    sqlx::query_as!(
        Feed,
        // Unable to SELECT * here due to https://github.com/launchbadge/sqlx/issues/1004
        // language=PostGreSQL
        r#"select
            feed_id,
            title,
            url,
            type as "feed_type: FeedType",
            description,
            created_at,
            updated_at,
            deleted_at
        from feed where feed_id = $1"#,
        feed_id
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        if let sqlx::error::Error::RowNotFound = error {
            return Error::NotFound("feed", feed_id);
        }
        Error::Sqlx(error)
    })
}

pub async fn get_feeds(pool: &PgPool) -> sqlx::Result<Vec<Feed>> {
    sqlx::query_as!(
        Feed,
        r#"select
            feed_id,
            title,
            url,
            type as "feed_type: FeedType",
            description,
            created_at,
            updated_at,
            deleted_at
        from feed
        where deleted_at is null"#
    )
    .fetch_all(pool)
    .await
}

pub async fn create_feed(pool: &PgPool, payload: CreateFeed) -> Result<Feed> {
    payload.validate()?;
    Ok(sqlx::query_as!(
        Feed,
        r#"insert into feed (
            title, url, type, description
        ) values (
            $1, $2, $3, $4
        ) returning
            feed_id,
            title,
            url,
            type as "feed_type: FeedType",
            description,
            created_at,
            updated_at,
            deleted_at
        "#,
        payload.title,
        payload.url,
        payload.feed_type as FeedType,
        payload.description
    )
    .fetch_one(pool)
    .await?)
}

pub async fn delete_feed(pool: &PgPool, feed_id: Uuid) -> Result<()> {
    sqlx::query!("update feed set deleted_at = now() where feed_id = $1", feed_id)
        .execute(pool)
        .await?;
    Ok(())
}
