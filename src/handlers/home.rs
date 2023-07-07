use axum::extract::State;
use axum::response::Response;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::entry::{get_entries, GetEntriesOptions};
use crate::partials::{layout::Layout, entry_list::entry_list};

pub async fn get(State(pool): State<PgPool>, layout: Layout) -> Result<Response> {
    let entries = get_entries(&pool, GetEntriesOptions::default()).await?;
    Ok(layout.render(entry_list(entries)))
}
