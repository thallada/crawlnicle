use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::PgPool;

use crate::error::{Error, Result};
use crate::models::feed::{create_feed, get_feed, delete_feed, CreateFeed, Feed};

pub async fn get(State(pool): State<PgPool>, Path(id): Path<i32>) -> Result<Json<Feed>> {
    Ok(Json(get_feed(&pool, id).await?))
}

pub async fn post(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateFeed>,
) -> Result<Json<Feed>, Error> {
    Ok(Json(create_feed(&pool, payload).await?))
}

pub async fn delete(State(pool): State<PgPool>, Path(id): Path<i32>) -> Result<()> {
    delete_feed(&pool, id).await
}
