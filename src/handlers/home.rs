use axum::extract::State;
use axum::response::Response;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::entry::Entry;
use crate::partials::{layout::Layout, entry_list::entry_list};

pub async fn get(State(pool): State<PgPool>, layout: Layout) -> Result<Response> {
    let options = Default::default();
    let entries = Entry::get_all(&pool, &options).await?;
    Ok(layout.render(html! {
        ul class="entries" {
            (entry_list(entries, &options))
        }
    }))
}
