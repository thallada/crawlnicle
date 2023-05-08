use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "feed_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum FeedType {
    Atom,
    Rss,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Feed {
    id: i32,
    title: String,
    url: String,
    #[serde(rename = "type")]
    feed_type: FeedType,
    description: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateFeed {
    #[validate(length(max = 255))]
    title: String,
    #[validate(url)]
    url: String,
    #[serde(rename = "type")]
    feed_type: FeedType,
    #[validate(length(max = 524288))]
    description: Option<String>,
}

pub async fn get_feed(pool: PgPool, id: i32) -> Result<Feed> {
    sqlx::query_as!(
        Feed,
        // Unable to SELECT * here due to https://github.com/launchbadge/sqlx/issues/1004
        r#"SELECT
            id,
            title,
            url,
            type as "feed_type: FeedType",
            description,
            created_at,
            updated_at,
            deleted_at
        FROM feeds WHERE id = $1"#,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|error| {
        if let sqlx::error::Error::RowNotFound = error {
            return Error::NotFound("feed", id);
        }
        Error::Sqlx(error)
    })
}

pub async fn get_feeds(pool: PgPool) -> sqlx::Result<Vec<Feed>> {
    sqlx::query_as!(
        Feed,
        r#"SELECT
            id,
            title,
            url,
            type as "feed_type: FeedType",
            description,
            created_at,
            updated_at,
            deleted_at
        FROM feeds"#)
        .fetch_all(&pool)
        .await
}

pub async fn create_feed(pool: PgPool, payload: CreateFeed) -> Result<Feed> {
    payload.validate()?;
    Ok(sqlx::query_as!(
        Feed,
        r#"INSERT INTO feeds (
            title, url, type, description, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, now(), now()
        ) RETURNING
            id,
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
    .fetch_one(&pool)
    .await?)
}
