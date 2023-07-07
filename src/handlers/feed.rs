use axum::extract::{Path, State};
use axum::response::Response;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::entry::get_entries_for_feed;
use crate::models::feed::get_feed;
use crate::partials::{entry_list::entry_list, layout::Layout};
use crate::uuid::Base62Uuid;

pub async fn get(
    Path(id): Path<Base62Uuid>,
    State(pool): State<PgPool>,
    layout: Layout,
) -> Result<Response> {
    let feed = get_feed(&pool, id.as_uuid()).await?;
    let entries = get_entries_for_feed(&pool, feed.feed_id, Default::default()).await?;
    Ok(layout.render(html! {
        h1 { (feed.title.unwrap_or_else(|| "Untitled Feed".to_string())) }
        (entry_list(entries))
    }))
}
