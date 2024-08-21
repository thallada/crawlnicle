use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::PgPool;

use crate::error::{Error, Result};
use crate::models::feed::{CreateFeed, Feed};
use crate::uuid::Base62Uuid;

pub async fn get(State(db): State<PgPool>, Path(id): Path<Base62Uuid>) -> Result<Json<Feed>> {
    Ok(Json(Feed::get(&db, id.as_uuid()).await?))
}

pub async fn post(
    State(db): State<PgPool>,
    Json(payload): Json<CreateFeed>,
) -> Result<Json<Feed>, Error> {
    Ok(Json(Feed::create(&db, payload).await?))
}

pub async fn delete(State(db): State<PgPool>, Path(id): Path<Base62Uuid>) -> Result<()> {
    Feed::delete(&db, id.as_uuid()).await
}
