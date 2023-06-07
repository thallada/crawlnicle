use axum::extract::{State, Path};
use axum::response::Response;
use maud::{html, PreEscaped};
use sqlx::PgPool;

use crate::error::Result;
use crate::models::entry::get_entry;
use crate::partials::layout::Layout;

pub async fn get(Path(id): Path<i32>, State(pool): State<PgPool>, layout: Layout) -> Result<Response> {
    let entry = get_entry(&pool, id).await?;
    Ok(layout.render(html! {
        @let title = entry.title.unwrap_or_else(|| "Untitled".to_string());
        h1 { a href=(entry.url) { (title) } }
        @let content = entry.html_content.unwrap_or_else(|| "No content".to_string());
        (PreEscaped(content))
    }))
}
