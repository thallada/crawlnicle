use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

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

#[derive(Deserialize)]
pub struct CreateItem {
    title: String,
    url: String,
    description: Option<String>,
}

pub async fn get_item(pool: PgPool, id: i32) -> sqlx::Result<Item> {
    sqlx::query_as!(Item, "SELECT * FROM items WHERE id = $1", id)
        .fetch_one(&pool)
        .await
}

pub async fn get_items(pool: PgPool) -> sqlx::Result<Vec<Item>> {
    sqlx::query_as!(Item, "SELECT * FROM items")
        .fetch_all(&pool)
        .await
}

pub async fn create_item(pool: PgPool, payload: CreateItem) -> sqlx::Result<Item> {
    sqlx::query_as!(
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
    .await
}
