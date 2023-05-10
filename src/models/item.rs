use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::{Validate, ValidationErrors};

use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub feed_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateItem {
    #[validate(length(max = 255))]
    pub title: String,
    #[validate(url)]
    pub url: String,
    #[validate(length(max = 524288))]
    pub description: Option<String>,
    #[validate(range(min = 1))]
    pub feed_id: i32,
}

pub async fn get_item(pool: &PgPool, id: i32) -> Result<Item> {
    sqlx::query_as!(Item, "SELECT * FROM items WHERE id = $1", id)
        .fetch_one(pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::RowNotFound = error {
                return Error::NotFound("item", id);
            }
            Error::Sqlx(error)
        })
}

pub async fn get_items(pool: &PgPool) -> sqlx::Result<Vec<Item>> {
    sqlx::query_as!(Item, "SELECT * FROM items WHERE deleted_at IS NULL")
        .fetch_all(pool)
        .await
}

pub async fn create_item(pool: &PgPool, payload: CreateItem) -> Result<Item> {
    payload.validate()?;
    sqlx::query_as!(
        Item,
        "INSERT INTO items (
            title, url, description, feed_id, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, now(), now()
        ) RETURNING *",
        payload.title,
        payload.url,
        payload.description,
        payload.feed_id,
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

pub async fn create_items(pool: &PgPool, payload: Vec<CreateItem>) -> Result<Vec<Item>> {
    let mut titles = Vec::with_capacity(payload.len());
    let mut urls = Vec::with_capacity(payload.len());
    let mut descriptions: Vec<Option<String>> = Vec::with_capacity(payload.len());
    let mut feed_ids = Vec::with_capacity(payload.len());
    payload.iter().map(|item| {
        titles.push(item.title.clone());
        urls.push(item.url.clone());
        descriptions.push(item.description.clone());
        feed_ids.push(item.feed_id);
        item.validate()
    }).collect::<Result<Vec<()>, ValidationErrors>>()?;
    sqlx::query_as!(
        Item,
        "INSERT INTO items (
            title, url, description, feed_id, created_at, updated_at
        ) SELECT *, now(), now() FROM UNNEST($1::text[], $2::text[], $3::text[], $4::int[])
        RETURNING *",
        titles.as_slice(),
        urls.as_slice(),
        descriptions.as_slice() as &[Option<String>],
        feed_ids.as_slice(),
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

pub async fn upsert_items(pool: &PgPool, payload: Vec<CreateItem>) -> Result<Vec<Item>> {
    let mut titles = Vec::with_capacity(payload.len());
    let mut urls = Vec::with_capacity(payload.len());
    let mut descriptions: Vec<Option<String>> = Vec::with_capacity(payload.len());
    let mut feed_ids = Vec::with_capacity(payload.len());
    payload.iter().map(|item| {
        titles.push(item.title.clone());
        urls.push(item.url.clone());
        descriptions.push(item.description.clone());
        feed_ids.push(item.feed_id);
        item.validate()
    }).collect::<Result<Vec<()>, ValidationErrors>>()?;
    sqlx::query_as!(
        Item,
        "INSERT INTO items (
            title, url, description, feed_id, created_at, updated_at
        ) SELECT *, now(), now() FROM UNNEST($1::text[], $2::text[], $3::text[], $4::int[])
        ON CONFLICT DO NOTHING
        RETURNING *",
        titles.as_slice(),
        urls.as_slice(),
        descriptions.as_slice() as &[Option<String>],
        feed_ids.as_slice(),
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

pub async fn delete_item(pool: &PgPool, id: i32) -> Result<()> {
    sqlx::query!("UPDATE items SET deleted_at = now() WHERE id = $1", id)
        .execute(pool)
        .await?;
    Ok(())
}
