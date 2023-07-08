use std::fs;

use axum::extract::{Path, State};
use axum::response::Response;
use maud::{html, PreEscaped};
use sqlx::PgPool;

use crate::config::Config;
use crate::error::Result;
use crate::models::entry::get_entry;
use crate::partials::layout::Layout;
use crate::uuid::Base62Uuid;

pub async fn get(
    Path(id): Path<Base62Uuid>,
    State(pool): State<PgPool>,
    State(config): State<Config>,
    layout: Layout,
) -> Result<Response> {
    let entry = get_entry(&pool, id.as_uuid()).await?;
    let content_dir = std::path::Path::new(&config.content_dir);
    let content_path = content_dir.join(format!("{}.html", entry.entry_id));
    Ok(layout.render(html! {
        article {
            @let title = entry.title.unwrap_or_else(|| "Untitled".to_string());
            h2 { a href=(entry.url) { (title) } }
            @let published_at = entry.published_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            span class="published" {
                strong { "Published: " }
                time datetime=(published_at) data-controller="local-time" {
                    (published_at)
                }
            }
            @let content = fs::read_to_string(content_path).unwrap_or_else(|_| "No content".to_string());
            (PreEscaped(content))
        }
    }))
}
