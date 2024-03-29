use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgQueryResult, Executor, FromRow, Postgres};
use uuid::Uuid;
use validator::Validate;

use crate::error::{Error, Result};

pub const DEFAULT_FEEDS_PAGE_SIZE: i64 = 50;

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

pub const MIN_CRAWL_INTERVAL_MINUTES: i32 = 1;
pub const MAX_CRAWL_INTERVAL_MINUTES: i32 = 5040;

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
    pub etag_header: Option<String>,
    pub last_modified_header: Option<String>,
    pub last_crawled_at: Option<DateTime<Utc>>,
    pub last_entry_published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Default, Validate)]
pub struct CreateFeed {
    #[validate(length(max = 255))]
    pub title: Option<String>,
    #[validate(url)]
    pub url: String,
    #[validate(length(max = 524288))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Default, Validate)]
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

#[derive(Debug, Clone, Deserialize)]
pub enum GetFeedsSort {
    Title,
    CreatedAt,
    LastCrawledAt,
    LastEntryPublishedAt,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct GetFeedsOptions {
    pub sort: Option<GetFeedsSort>,
    pub before: Option<DateTime<Utc>>,
    pub after_title: Option<String>,
    pub before_id: Option<Uuid>,
    pub limit: Option<i64>,
}

impl Feed {
    pub fn next_crawl_time(&self) -> Option<DateTime<Utc>> {
        self.last_crawled_at.map(|last_crawled_at| {
            last_crawled_at + chrono::Duration::minutes(self.crawl_interval_minutes as i64)
        })
    }

    pub async fn get(db: impl Executor<'_, Database = Postgres>, feed_id: Uuid) -> Result<Feed> {
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
                etag_header,
                last_modified_header,
                last_crawled_at,
                last_entry_published_at,
                created_at,
                updated_at,
                deleted_at
            from feed where feed_id = $1"#,
            feed_id
        )
        .fetch_one(db)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::RowNotFound = error {
                return Error::NotFoundUuid("feed", feed_id);
            }
            Error::Sqlx(error)
        })
    }

    pub async fn get_all(
        db: impl Executor<'_, Database = Postgres>,
        options: &GetFeedsOptions,
    ) -> sqlx::Result<Vec<Feed>> {
        // TODO: make sure there are indices for all of these sort options
        match options.sort.as_ref().unwrap_or(&GetFeedsSort::CreatedAt) {
            GetFeedsSort::Title => {
                if let Some(after_title) = &options.after_title {
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
                            etag_header,
                            last_modified_header,
                            last_crawled_at,
                            last_entry_published_at,
                            created_at,
                            updated_at,
                            deleted_at
                        from feed
                        where deleted_at is null
                        and (title, feed_id) > ($1, $2)
                        order by title asc, feed_id asc
                        limit $3
                        "#,
                        after_title,
                        options.before_id.unwrap_or(Uuid::nil()),
                        options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE),
                    )
                    .fetch_all(db)
                    .await
                } else {
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
                            etag_header,
                            last_modified_header,
                            last_crawled_at,
                            last_entry_published_at,
                            created_at,
                            updated_at,
                            deleted_at
                        from feed
                        where deleted_at is null
                        order by title asc, feed_id asc
                        limit $1
                        "#,
                        options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE),
                    )
                    .fetch_all(db)
                    .await
                }
            }
            GetFeedsSort::CreatedAt => {
                if let Some(created_before) = options.before {
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
                            etag_header,
                            last_modified_header,
                            last_crawled_at,
                            last_entry_published_at,
                            created_at,
                            updated_at,
                            deleted_at
                        from feed
                        where deleted_at is null
                        and (created_at, feed_id) < ($1, $2)
                        order by created_at desc, feed_id desc
                        limit $3
                        "#,
                        created_before,
                        options.before_id.unwrap_or(Uuid::nil()),
                        options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE),
                    )
                    .fetch_all(db)
                    .await
                } else {
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
                            etag_header,
                            last_modified_header,
                            last_crawled_at,
                            last_entry_published_at,
                            created_at,
                            updated_at,
                            deleted_at
                        from feed
                        where deleted_at is null
                        order by created_at desc, feed_id desc
                        limit $1
                        "#,
                        options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE),
                    )
                    .fetch_all(db)
                    .await
                }
            }
            GetFeedsSort::LastCrawledAt => {
                if let Some(crawled_before) = options.before {
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
                            etag_header,
                            last_modified_header,
                            last_crawled_at,
                            last_entry_published_at,
                            created_at,
                            updated_at,
                            deleted_at
                        from feed
                        where deleted_at is null
                        and (last_crawled_at, feed_id) < ($1, $2)
                        order by last_crawled_at desc, feed_id desc
                        limit $3
                        "#,
                        crawled_before,
                        options.before_id.unwrap_or(Uuid::nil()),
                        options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE),
                    )
                    .fetch_all(db)
                    .await
                } else {
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
                            etag_header,
                            last_modified_header,
                            last_crawled_at,
                            last_entry_published_at,
                            created_at,
                            updated_at,
                            deleted_at
                        from feed
                        where deleted_at is null
                        order by last_crawled_at desc, feed_id desc
                        limit $1
                        "#,
                        options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE),
                    )
                    .fetch_all(db)
                    .await
                }
            }
            GetFeedsSort::LastEntryPublishedAt => {
                if let Some(published_before) = options.before {
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
                            etag_header,
                            last_modified_header,
                            last_crawled_at,
                            last_entry_published_at,
                            created_at,
                            updated_at,
                            deleted_at
                        from feed
                        where deleted_at is null
                        and (last_entry_published_at, feed_id) < ($1, $2)
                        order by last_entry_published_at desc, feed_id desc
                        limit $3
                        "#,
                        published_before,
                        options.before_id.unwrap_or(Uuid::nil()),
                        options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE),
                    )
                    .fetch_all(db)
                    .await
                } else {
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
                            etag_header,
                            last_modified_header,
                            last_crawled_at,
                            last_entry_published_at,
                            created_at,
                            updated_at,
                            deleted_at
                        from feed
                        where deleted_at is null
                        order by last_entry_published_at desc, feed_id desc
                        limit $1
                        "#,
                        options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE),
                    )
                    .fetch_all(db)
                    .await
                }
            }
        }
    }

    pub async fn create(
        db: impl Executor<'_, Database = Postgres>,
        payload: CreateFeed,
    ) -> Result<Feed> {
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
                etag_header,
                last_modified_header,
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
        .fetch_one(db)
        .await?)
    }

    pub async fn upsert(
        db: impl Executor<'_, Database = Postgres>,
        payload: UpsertFeed,
    ) -> Result<Feed> {
        payload.validate()?;
        Ok(sqlx::query_as!(
            Feed,
            r#"insert into feed (
                title, url, type, description
            ) values (
                $1, $2, COALESCE($3, 'unknown'::feed_type), $4
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
                etag_header,
                last_modified_header,
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
        .fetch_one(db)
        .await?)
    }

    pub async fn update(
        db: impl Executor<'_, Database = Postgres>,
        feed_id: Uuid,
        payload: UpdateFeed,
    ) -> Result<Feed> {
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

        Ok(query.fetch_one(db).await?)
    }

    pub async fn delete(db: impl Executor<'_, Database = Postgres>, feed_id: Uuid) -> Result<()> {
        sqlx::query!(
            "update feed set deleted_at = now() where feed_id = $1",
            feed_id
        )
        .execute(db)
        .await?;
        Ok(())
    }

    pub async fn update_crawl_error(
        db: impl Executor<'_, Database = Postgres>,
        feed_id: Uuid,
        last_crawl_error: String,
    ) -> Result<PgQueryResult> {
        Ok(sqlx::query!(
            r#"update feed set
                last_crawl_error = $2
            where feed_id = $1"#,
            feed_id,
            last_crawl_error,
        )
        .execute(db)
        .await?)
    }

    pub async fn save(&self, db: impl Executor<'_, Database = Postgres>) -> Result<Feed> {
        Ok(sqlx::query_as!(
            Feed,
            r#"update feed set
                title = $2,
                url = $3,
                type = $4,
                description = $5,
                crawl_interval_minutes = $6,
                last_crawl_error = $7,
                etag_header = $8,
                last_modified_header = $9,
                last_crawled_at = $10,
                last_entry_published_at = $11
            where feed_id = $1
            returning
                feed_id,
                title,
                url,
                type as "feed_type: FeedType",
                description,
                crawl_interval_minutes,
                last_crawl_error,
                etag_header,
                last_modified_header,
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
            self.etag_header,
            self.last_modified_header,
            self.last_crawled_at,
            self.last_entry_published_at,
        )
        .fetch_one(db)
        .await?)
    }
}
