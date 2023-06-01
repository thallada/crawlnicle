use axum::extract::State;
use maud::{html, Markup};
use sqlx::PgPool;

use crate::error::Result;
use crate::models::entry::get_entries;

pub async fn get(State(pool): State<PgPool>) -> Result<Markup> {
    let entries = get_entries(&pool).await?;
    Ok(html! {
        h1 { "crawlnicle" }
        ul {
            @for entry in entries {
                @let title = entry.title.unwrap_or_else(|| "Untitled".to_string());
                li { (title) }
            }
        }
    })
}
