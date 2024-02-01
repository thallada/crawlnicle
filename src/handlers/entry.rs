use std::fs;

use axum::extract::{Path, State};
use axum::response::Response;
use axum_extra::TypedHeader;
use maud::{html, PreEscaped};
use sqlx::PgPool;

use crate::config::Config;
use crate::error::Result;
use crate::htmx::HXTarget;
use crate::models::entry::Entry;
use crate::partials::layout::Layout;
use crate::partials::time::date_time;
use crate::uuid::Base62Uuid;

pub async fn get(
    Path(id): Path<Base62Uuid>,
    State(pool): State<PgPool>,
    State(config): State<Config>,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
) -> Result<Response> {
    let entry = Entry::get(&pool, id.as_uuid()).await?;
    let content_dir = std::path::Path::new(&config.content_dir);
    let content_path = content_dir.join(format!("{}.html", entry.entry_id));
    let title = entry.title.unwrap_or_else(|| "Untitled Entry".to_string());
    let content = fs::read_to_string(content_path).unwrap_or_else(|_| "No content".to_string());
    Ok(layout
        .with_subtitle(&title)
        .targeted(hx_target)
        .render(html! {
            article class="prose lg:prose-xl my-6 mx-auto prose-a:text-blue-600 prose-a:no-underline visited:prose-a:text-purple-600 hover:prose-a:underline" {
                header {
                    h2 class="mb-4 text-2xl font-medium" {
                        a href=(entry.url) { (title) }
                    }
                }
                div {
                    span class="text-sm text-gray-600" {
                        strong { "Published: " }
                        (date_time(entry.published_at))
                    }
                }
                (PreEscaped(content))
            }
        }))
}
