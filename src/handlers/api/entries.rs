use axum::{extract::State, Json};
use sqlx::PgPool;

use crate::error::Error;
use crate::models::entry::Entry;

pub async fn get(State(pool): State<PgPool>) -> Result<Json<Vec<Entry>>, Error> {
    Ok(Json(Entry::get_all(&pool, Default::default()).await?))
}
