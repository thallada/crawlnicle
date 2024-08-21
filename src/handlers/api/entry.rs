use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::PgPool;

use crate::error::Error;
use crate::models::entry::{CreateEntry, Entry};
use crate::uuid::Base62Uuid;

pub async fn get(
    State(db): State<PgPool>,
    Path(id): Path<Base62Uuid>,
) -> Result<Json<Entry>, Error> {
    Ok(Json(Entry::get(&db, id.as_uuid()).await?))
}

pub async fn post(
    State(db): State<PgPool>,
    Json(payload): Json<CreateEntry>,
) -> Result<Json<Entry>, Error> {
    Ok(Json(Entry::create(&db, payload).await?))
}
