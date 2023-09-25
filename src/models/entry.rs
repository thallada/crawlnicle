use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use validator::{Validate, ValidationErrors};

use crate::error::{Error, Result};

pub const DEFAULT_ENTRIES_PAGE_SIZE: i64 = 50;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entry {
    pub entry_id: Uuid,
    pub title: Option<String>,
    pub url: String,
    pub description: Option<String>,
    pub feed_id: Uuid,
    pub etag_header: Option<String>,
    pub last_modified_header: Option<String>,
    pub published_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateEntry {
    #[validate(length(max = 255))]
    pub title: Option<String>,
    #[validate(url)]
    pub url: String,
    #[validate(length(max = 524288))]
    pub description: Option<String>,
    pub feed_id: Uuid,
    pub published_at: DateTime<Utc>,
}

#[derive(Default, Deserialize)]
pub struct GetEntriesOptions {
    pub feed_id: Option<Uuid>,
    pub published_before: Option<DateTime<Utc>>,
    pub id_before: Option<Uuid>,
    pub limit: Option<i64>,
}

impl Entry {
    pub async fn get(pool: &PgPool, entry_id: Uuid) -> Result<Entry> {
        sqlx::query_as!(Entry, "select * from entry where entry_id = $1", entry_id)
            .fetch_one(pool)
            .await
            .map_err(|error| {
                if let sqlx::error::Error::RowNotFound = error {
                    return Error::NotFoundUuid("entry", entry_id);
                }
                Error::Sqlx(error)
            })
    }

    pub async fn get_all(pool: &PgPool, options: &GetEntriesOptions) -> sqlx::Result<Vec<Entry>> {
        if let Some(feed_id) = options.feed_id {
            if let Some(published_before) = options.published_before {
                if let Some(id_before) = options.id_before {
                    sqlx::query_as!(
                        Entry,
                        "select * from entry
                            where deleted_at is null
                            and feed_id = $1
                            and (published_at, entry_id) < ($2, $3)
                            order by published_at desc, entry_id desc
                            limit $4
                        ",
                        feed_id,
                        published_before,
                        id_before,
                        options.limit.unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE)
                    )
                    .fetch_all(pool)
                    .await
                } else {
                    sqlx::query_as!(
                        Entry,
                        "select * from entry
                            where deleted_at is null
                            and feed_id = $1
                            and published_at < $2
                            order by published_at desc
                            limit $3
                        ",
                        feed_id,
                        published_before,
                        options.limit.unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE)
                    )
                    .fetch_all(pool)
                    .await
                }
            } else {
                sqlx::query_as!(
                    Entry,
                    "select * from entry
                        where deleted_at is null
                        and feed_id = $1
                        order by published_at desc
                        limit $2
                    ",
                    feed_id,
                    options.limit.unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE)
                )
                .fetch_all(pool)
                .await
            }
        } else {
            if let Some(published_before) = options.published_before {
                if let Some(id_before) = options.id_before {
                    sqlx::query_as!(
                        Entry,
                        "select * from entry
                            where deleted_at is null
                            and (published_at, entry_id) < ($1, $2)
                            order by published_at desc, entry_id desc
                            limit $3
                        ",
                        published_before,
                        id_before,
                        options.limit.unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE)
                    )
                    .fetch_all(pool)
                    .await
                } else {
                    sqlx::query_as!(
                        Entry,
                        "select * from entry
                            where deleted_at is null
                            and published_at < $1
                            order by published_at desc
                            limit $2
                        ",
                        published_before,
                        options.limit.unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE)
                    )
                    .fetch_all(pool)
                    .await
                }
            } else {
                sqlx::query_as!(
                    Entry,
                    "select * from entry
                        where deleted_at is null
                        order by published_at desc
                        limit $1
                    ",
                    options.limit.unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE)
                )
                .fetch_all(pool)
                .await
            }
        }
    }

    pub async fn create(pool: &PgPool, payload: CreateEntry) -> Result<Entry> {
        payload.validate()?;
        sqlx::query_as!(
            Entry,
            "insert into entry (
                title, url, description, feed_id, published_at
            ) values (
                $1, $2, $3, $4, $5
            ) returning *",
            payload.title,
            payload.url,
            payload.description,
            payload.feed_id,
            payload.published_at,
        )
        .fetch_one(pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::Database(ref psql_error) = error {
                if psql_error.code().as_deref() == Some("23503") {
                    return Error::RelationNotFound("feed");
                }
            }
            Error::Sqlx(error)
        })
    }

    pub async fn upsert(pool: &PgPool, payload: CreateEntry) -> Result<Entry> {
        payload.validate()?;
        sqlx::query_as!(
            Entry,
            "insert into entry (
                title, url, description, feed_id, published_at
            ) values (
                $1, $2, $3, $4, $5
            ) on conflict (url, feed_id) do update set
                title = excluded.title,
                description = excluded.description,
                published_at = excluded.published_at
            returning *",
            payload.title,
            payload.url,
            payload.description,
            payload.feed_id,
            payload.published_at,
        )
        .fetch_one(pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::Database(ref psql_error) = error {
                if psql_error.code().as_deref() == Some("23503") {
                    return Error::RelationNotFound("feed");
                }
            }
            Error::Sqlx(error)
        })
    }

    pub async fn bulk_create(pool: &PgPool, payload: Vec<CreateEntry>) -> Result<Vec<Entry>> {
        let mut titles = Vec::with_capacity(payload.len());
        let mut urls = Vec::with_capacity(payload.len());
        let mut descriptions: Vec<Option<String>> = Vec::with_capacity(payload.len());
        let mut feed_ids = Vec::with_capacity(payload.len());
        let mut published_ats = Vec::with_capacity(payload.len());
        payload
            .iter()
            .map(|entry| {
                titles.push(entry.title.clone());
                urls.push(entry.url.clone());
                descriptions.push(entry.description.clone());
                feed_ids.push(entry.feed_id);
                published_ats.push(entry.published_at);
                entry.validate()
            })
            .collect::<Result<Vec<()>, ValidationErrors>>()?;
        sqlx::query_as!(
            Entry,
            "insert into entry (
                title, url, description, feed_id, published_at
            ) select * from unnest($1::text[], $2::text[], $3::text[], $4::uuid[], $5::timestamptz[])
            returning *",
            titles.as_slice() as &[Option<String>],
            urls.as_slice(),
            descriptions.as_slice() as &[Option<String>],
            feed_ids.as_slice(),
            published_ats.as_slice(),
        )
        .fetch_all(pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::Database(ref psql_error) = error {
                if psql_error.code().as_deref() == Some("23503") {
                    return Error::RelationNotFound("feed");
                }
            }
            Error::Sqlx(error)
        })
    }

    pub async fn bulk_upsert(pool: &PgPool, payload: Vec<CreateEntry>) -> Result<Vec<Entry>> {
        let mut titles = Vec::with_capacity(payload.len());
        let mut urls = Vec::with_capacity(payload.len());
        let mut descriptions: Vec<Option<String>> = Vec::with_capacity(payload.len());
        let mut feed_ids = Vec::with_capacity(payload.len());
        let mut published_ats = Vec::with_capacity(payload.len());
        payload
            .iter()
            .map(|entry| {
                titles.push(entry.title.clone());
                urls.push(entry.url.clone());
                descriptions.push(entry.description.clone());
                feed_ids.push(entry.feed_id);
                published_ats.push(entry.published_at);
                entry.validate()
            })
            .collect::<Result<Vec<()>, ValidationErrors>>()?;
        sqlx::query_as!(
            Entry,
            "insert into entry (
                title, url, description, feed_id, published_at
            ) select * from unnest($1::text[], $2::text[], $3::text[], $4::uuid[], $5::timestamptz[])
            on conflict (url, feed_id) do update set
                title = excluded.title,
                description = excluded.description,
                published_at = excluded.published_at
            returning *",
            titles.as_slice() as &[Option<String>],
            urls.as_slice(),
            descriptions.as_slice() as &[Option<String>],
            feed_ids.as_slice(),
            published_ats.as_slice(),
        )
        .fetch_all(pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::Database(ref psql_error) = error {
                if psql_error.code().as_deref() == Some("23503") {
                    return Error::RelationNotFound("feed");
                }
            }
            Error::Sqlx(error)
        })
    }

    pub async fn update(pool: &PgPool, payload: Entry) -> Result<Entry> {
        sqlx::query_as!(
            Entry,
            "update entry set
                title = $2,
                url = $3,
                description = $4,
                feed_id = $5,
                etag_header = $6,
                last_modified_header = $7,
                published_at = $8
            where entry_id = $1
            returning *
            ",
            payload.entry_id,
            payload.title,
            payload.url,
            payload.description,
            payload.feed_id,
            payload.etag_header,
            payload.last_modified_header,
            payload.published_at,
        )
        .fetch_one(pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::Database(ref psql_error) = error {
                if psql_error.code().as_deref() == Some("23503") {
                    return Error::RelationNotFound("feed");
                }
            }
            Error::Sqlx(error)
        })
    }

    pub async fn delete(pool: &PgPool, entry_id: Uuid) -> Result<()> {
        sqlx::query!(
            "update entry set deleted_at = now() where entry_id = $1",
            entry_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
