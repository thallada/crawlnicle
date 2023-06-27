use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::PgPool;

use crate::error::{Error, Result};
use crate::models::feed::{create_feed, delete_feed, get_feed, CreateFeed, Feed};
use crate::uuid::Base62Uuid;

pub async fn get(State(pool): State<PgPool>, Path(id): Path<Base62Uuid>) -> Result<Json<Feed>> {
    Ok(Json(get_feed(&pool, id.as_uuid()).await?))
}

pub async fn post(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateFeed>,
) -> Result<Json<Feed>, Error> {
    Ok(Json(create_feed(&pool, payload).await?))
}

pub async fn delete(State(pool): State<PgPool>, Path(id): Path<Base62Uuid>) -> Result<()> {
    delete_feed(&pool, id.as_uuid()).await
}
