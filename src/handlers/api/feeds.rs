use axum::{extract::State, Json};
use sqlx::PgPool;

use crate::error::Error;
use crate::models::feed::{get_feeds, Feed};

pub async fn get(State(pool): State<PgPool>) -> Result<Json<Vec<Feed>>, Error> {
    Ok(Json(get_feeds(&pool).await?))
}
