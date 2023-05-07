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
}

pub async fn get_item(pool: PgPool, id: i32) -> Result<Item> {
    sqlx::query_as!(Item, "SELECT * FROM items WHERE id = $1", id)
        .fetch_one(&pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::RowNotFound = error {
                return Error::NotFound("item");
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
    Ok(sqlx::query_as!(
        Item,
        "INSERT INTO items (
            title, url, description, created_at, updated_at
        ) VALUES (
            $1, $2, $3, now(), now()
        ) RETURNING *",
        payload.title,
        payload.url,
        payload.description
    )
    .fetch_one(&pool)
    .await?)
}
