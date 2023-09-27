use std::fs;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::TypedHeader;
use maud::{html, PreEscaped};
use sqlx::PgPool;

use crate::config::Config;
use crate::error::Result;
use crate::htmx::HXBoosted;
use crate::models::entry::Entry;
use crate::partials::layout::Layout;
use crate::uuid::Base62Uuid;

pub async fn get(
    Path(id): Path<Base62Uuid>,
    State(pool): State<PgPool>,
    State(config): State<Config>,
    hx_boosted: Option<TypedHeader<HXBoosted>>,
    layout: Layout,
) -> Result<Response> {
    let entry = Entry::get(&pool, id.as_uuid()).await?;
    let content_dir = std::path::Path::new(&config.content_dir);
    let content_path = content_dir.join(format!("{}.html", entry.entry_id));
    let title = entry.title.unwrap_or_else(|| "Untitled Entry".to_string());
    let published_at = entry
        .published_at
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let content = fs::read_to_string(content_path).unwrap_or_else(|_| "No content".to_string());
    Ok(layout
        .with_subtitle(&title)
        .boosted(hx_boosted)
        .render(html! {
            article {
                header {
                    h2 class="title" { a href=(entry.url) { (title) } }
                }
                div {
                    span class="published" {
                        strong { "Published: " }
                        time datetime=(published_at) class="local-time" {
                            (published_at)
                        }
                    }
                }
                (PreEscaped(content))
            }
        }))
}
