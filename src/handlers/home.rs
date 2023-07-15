use axum::extract::State;
use axum::response::Response;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::entry::Entry;
use crate::partials::{layout::Layout, entry_list::entry_list};

pub async fn get(State(pool): State<PgPool>, layout: Layout) -> Result<Response> {
    let entries = Entry::get_all(&pool, Default::default()).await?;
    Ok(layout.render(entry_list(entries)))
}
