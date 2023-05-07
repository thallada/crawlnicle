use axum::{extract::{State, Path}, Json};
use sqlx::PgPool;

use crate::error::Error;
use crate::models::item::{create_item, get_item, CreateItem, Item};

pub async fn get(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<Item>, Error> {
    Ok(Json(get_item(pool, id).await?))
}

pub async fn post(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateItem>,
) -> Result<Json<Item>, Error> {
    Ok(Json(create_item(pool, payload).await?))
}
