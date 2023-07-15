use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use validator::Validate;

use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, Copy)]
#[sqlx(type_name = "feed_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum FeedType {
    Atom,
    JSON,
    RSS0,
    RSS1,
    RSS2,
    Unknown,
}

impl FromStr for FeedType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "atom" => Ok(FeedType::Atom),
            "json" => Ok(FeedType::JSON),
            "rss0" => Ok(FeedType::RSS0),
            "rss1" => Ok(FeedType::RSS1),
            "rss2" => Ok(FeedType::RSS2),
            "unknown" => Ok(FeedType::Unknown),
            _ => Err(format!("invalid feed type: {}", s)),
        }
    }
}

impl From<feed_rs::model::FeedType> for FeedType {
    fn from(value: feed_rs::model::FeedType) -> Self {
        match value {
            feed_rs::model::FeedType::Atom => FeedType::Atom,
            feed_rs::model::FeedType::JSON => FeedType::JSON,
            feed_rs::model::FeedType::RSS0 => FeedType::RSS0,
            feed_rs::model::FeedType::RSS1 => FeedType::RSS1,
            feed_rs::model::FeedType::RSS2 => FeedType::RSS2,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Feed {
    pub feed_id: Uuid,
    pub title: Option<String>,
    pub url: String,
    #[serde(rename = "type")]
    pub feed_type: FeedType,
    pub description: Option<String>,
    pub crawl_interval_minutes: i32,
    pub last_crawl_error: Option<String>,
    pub last_crawled_at: Option<DateTime<Utc>>,
    pub last_entry_published_at: Option<DateTime<Utc>>,
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
    #[validate(length(max = 524288))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpsertFeed {
    #[validate(length(max = 255))]
    pub title: Option<String>,
    #[validate(url)]
    pub url: String,
    pub feed_type: Option<FeedType>,
    #[validate(length(max = 524288))]
    pub description: Option<String>,
    pub last_crawl_error: Option<String>,
    pub last_crawled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Default, Validate)]
pub struct UpdateFeed {
    #[validate(length(max = 255))]
    pub title: Option<Option<String>>,
    #[validate(url)]
    pub url: Option<String>,
    pub feed_type: Option<FeedType>,
    #[validate(length(max = 524288))]
    pub description: Option<Option<String>>,
    pub crawl_interval_minutes: Option<i32>,
    pub last_crawl_error: Option<Option<String>>,
    pub last_crawled_at: Option<Option<DateTime<Utc>>>,
    pub last_entry_published_at: Option<Option<DateTime<Utc>>>,
}

impl Feed {
    pub async fn get(pool: &PgPool, feed_id: Uuid) -> Result<Feed> {
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
                crawl_interval_minutes,
                last_crawl_error,
                last_crawled_at,
                last_entry_published_at,
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

    pub async fn get_all(pool: &PgPool) -> sqlx::Result<Vec<Feed>> {
        sqlx::query_as!(
            Feed,
            r#"select
                feed_id,
                title,
                url,
                type as "feed_type: FeedType",
                description,
                crawl_interval_minutes,
                last_crawl_error,
                last_crawled_at,
                last_entry_published_at,
                created_at,
                updated_at,
                deleted_at
            from feed
            where deleted_at is null"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(pool: &PgPool, payload: CreateFeed) -> Result<Feed> {
        payload.validate()?;
        Ok(sqlx::query_as!(
            Feed,
            r#"insert into feed (
                title, url, description
            ) values (
                $1, $2, $3
            ) returning
                feed_id,
                title,
                url,
                type as "feed_type: FeedType",
                description,
                crawl_interval_minutes,
                last_crawl_error,
                last_crawled_at,
                last_entry_published_at,
                created_at,
                updated_at,
                deleted_at
            "#,
            payload.title,
            payload.url,
            payload.description
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn upsert(pool: &PgPool, payload: UpsertFeed) -> Result<Feed> {
        payload.validate()?;
        Ok(sqlx::query_as!(
            Feed,
            r#"insert into feed (
                title, url, type, description
            ) values (
                $1, $2, $3, $4
            ) on conflict (url) do update set
                title = excluded.title,
                url = excluded.url,
                type = COALESCE(excluded.type, feed.type),
                description = excluded.description
            returning
                feed_id,
                title,
                url,
                type as "feed_type: FeedType",
                description,
                crawl_interval_minutes,
                last_crawl_error,
                last_crawled_at,
                last_entry_published_at,
                created_at,
                updated_at,
                deleted_at
            "#,
            payload.title,
            payload.url,
            payload.feed_type as Option<FeedType>,
            payload.description
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn update(pool: &PgPool, feed_id: Uuid, payload: UpdateFeed) -> Result<Feed> {
        payload.validate()?;
        let mut query = sqlx::QueryBuilder::new("UPDATE feed SET ");

        let mut updates = query.separated(", ");
        if let Some(title) = payload.title {
            updates.push_unseparated("title = ");
            updates.push_bind(title);
        }
        if let Some(url) = payload.url {
            updates.push_unseparated("url = ");
            updates.push_bind(url);
        }
        if let Some(description) = payload.description {
            updates.push_unseparated("description = ");
            updates.push_bind(description);
        }
        if let Some(crawl_interval_minutes) = payload.crawl_interval_minutes {
            updates.push("crawl_interval_minutes = ");
            updates.push_bind(crawl_interval_minutes);
        }
        if let Some(last_crawl_error) = payload.last_crawl_error {
            updates.push_unseparated("last_crawl_error = ");
            updates.push_bind(last_crawl_error);
        }
        if let Some(last_crawled_at) = payload.last_crawled_at {
            updates.push_unseparated("last_crawled_at = ");
            updates.push_bind(last_crawled_at);
        }
        if let Some(last_entry_published_at) = payload.last_entry_published_at {
            updates.push_unseparated("last_entry_published_at = ");
            updates.push_bind(last_entry_published_at);
        }

        query.push(" WHERE id = ");
        query.push_bind(feed_id);
        query.push(" RETURNING *");

        let query = query.build_query_as();

        Ok(query.fetch_one(pool).await?)
    }

    pub async fn delete(pool: &PgPool, feed_id: Uuid) -> Result<()> {
        sqlx::query!(
            "update feed set deleted_at = now() where feed_id = $1",
            feed_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn save(&self, pool: &PgPool) -> Result<Feed> {
        Ok(sqlx::query_as!(
            Feed,
            r#"update feed set
                title = $2,
                url = $3,
                type = $4,
                description = $5,
                crawl_interval_minutes = $6,
                last_crawl_error = $7,
                last_crawled_at = $8,
                last_entry_published_at = $9
            where feed_id = $1
            returning
                feed_id,
                title,
                url,
                type as "feed_type: FeedType",
                description,
                crawl_interval_minutes,
                last_crawl_error,
                last_crawled_at,
                last_entry_published_at,
                created_at,
                updated_at,
                deleted_at
            "#,
            self.feed_id,
            self.title,
            self.url,
            self.feed_type as FeedType,
            self.description,
            self.crawl_interval_minutes,
            self.last_crawl_error,
            self.last_crawled_at,
            self.last_entry_published_at,
        )
        .fetch_one(pool)
        .await?)
    }
}
