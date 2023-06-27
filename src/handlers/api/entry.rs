use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::PgPool;

use crate::error::Error;
use crate::models::entry::{create_entry, get_entry, CreateEntry, Entry};
use crate::uuid::Base62Uuid;

pub async fn get(
    State(pool): State<PgPool>,
    Path(id): Path<Base62Uuid>,
) -> Result<Json<Entry>, Error> {
    Ok(Json(get_entry(&pool, id.as_uuid()).await?))
}

pub async fn post(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateEntry>,
) -> Result<Json<Entry>, Error> {
    Ok(Json(create_entry(&pool, payload).await?))
}
