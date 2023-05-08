use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    id: i32,
    title: String,
    url: String,
    description: Option<String>,
    feed_id: i32,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateItem {
    #[validate(length(max = 255))]
    title: String,
    #[validate(url)]
    url: String,
    #[validate(length(max = 524288))]
    description: Option<String>,
    #[validate(range(min = 1))]
    feed_id: i32,
}

pub async fn get_item(pool: PgPool, id: i32) -> Result<Item> {
    sqlx::query_as!(Item, "SELECT * FROM items WHERE id = $1", id)
        .fetch_one(&pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::RowNotFound = error {
                return Error::NotFound("item", id);
            }
            Error::Sqlx(error)
        })
}

pub async fn get_items(pool: PgPool) -> sqlx::Result<Vec<Item>> {
    sqlx::query_as!(Item, "SELECT * FROM items")
        .fetch_all(&pool)
        .await
}

pub async fn create_item(pool: PgPool, payload: CreateItem) -> Result<Item> {
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
    .fetch_one(&pool)
    .await
    .map_err(|error| {
        if let sqlx::error::Error::Database(ref psql_error) = error {
            if psql_error.code().as_deref() == Some("23503") {
                return Error::RelationNotFound("feed", payload.feed_id);
            }
        }
        Error::Sqlx(error)
    })
}
