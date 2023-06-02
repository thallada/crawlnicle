use axum::extract::State;
use axum::response::Response;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::entry::get_entries;
use crate::partials::layout::Layout;

pub async fn get(State(pool): State<PgPool>, layout: Layout) -> Result<Response> {
    let entries = get_entries(&pool).await?;
    Ok(layout.render(html! {
        ul {
            @for entry in entries {
                @let title = entry.title.unwrap_or_else(|| "Untitled".to_string());
                li { (title) }
            }
        }
    }))
}
