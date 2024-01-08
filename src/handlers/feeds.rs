use axum::extract::State;
use axum::response::Response;
use axum_extra::TypedHeader;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::htmx::HXTarget;
use crate::models::feed::{Feed, GetFeedsOptions};
use crate::partials::add_feed_form::add_feed_form;
use crate::partials::feed_list::feed_list;
use crate::partials::layout::Layout;
use crate::partials::opml_import_form::opml_import_form;

pub async fn get(
    State(pool): State<PgPool>,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
) -> Result<Response> {
    let options = GetFeedsOptions::default();
    let feeds = Feed::get_all(&pool, &options).await?;
    Ok(layout
        .with_subtitle("feeds")
        .targeted(hx_target)
        .render(html! {
            header { h2 class="mb-4 text-2xl font-medium" { "Feeds" } }
            div class="flex flex-col gap-6 lg:flex-row md:justify-between" {
                ul id="feed-list" class="list-none flex flex-col gap-4" {
                    (feed_list(feeds, &options, true))
                }
                div class="flex flex-col gap-6 max-w-md" {
                    h3 class="text-xl font-medium" { "Add Feed" }
                    (add_feed_form())
                    (opml_import_form())
                }
            }
        }))
}
