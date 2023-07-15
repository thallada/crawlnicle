use axum::{extract::State, Json};
use sqlx::PgPool;

use crate::error::Error;
use crate::models::feed::Feed;

pub async fn get(State(pool): State<PgPool>) -> Result<Json<Vec<Feed>>, Error> {
    // TODO: pagination
    Ok(Json(Feed::get_all(&pool).await?))
}
