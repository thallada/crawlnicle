use axum::{extract::State, Json};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::item::{get_items, Item};

pub async fn get(State(pool): State<PgPool>) -> Result<Json<Vec<Item>>, AppError> {
    Ok(Json(get_items(pool).await?))
}
