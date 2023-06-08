use axum::{extract::State, Json};
use sqlx::PgPool;

use crate::error::Error;
use crate::models::entry::{get_entries, Entry, GetEntriesOptions};

pub async fn get(State(pool): State<PgPool>) -> Result<Json<Vec<Entry>>, Error> {
    Ok(Json(get_entries(&pool, GetEntriesOptions::default()).await?))
}
