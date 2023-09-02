use axum::extract::{Query, State};
use maud::Markup;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::entry::{Entry, GetEntriesOptions};
use crate::partials::entry_list::entry_list;

pub async fn get(
    Query(options): Query<GetEntriesOptions>,
    State(pool): State<PgPool>,
) -> Result<Markup> {
    let entries = Entry::get_all(&pool, &options).await?;
    Ok(entry_list(entries, &options))
}
