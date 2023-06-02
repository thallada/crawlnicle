use axum::extract::{State, Path};
use axum::response::Response;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::entry::get_entry;
use crate::partials::layout::Layout;

pub async fn get(Path(id): Path<i32>, State(pool): State<PgPool>, layout: Layout) -> Result<Response> {
    let entry = get_entry(&pool, id).await?;
    Ok(layout.render(html! {
        @let title = entry.title.unwrap_or_else(|| "Untitled".to_string());
        h1 { a href=(entry.url) { (title) } }
        @let description = entry.description.unwrap_or_else(|| "No description".to_string());
        p { (description) }
    }))
}
