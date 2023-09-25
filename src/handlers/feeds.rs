use axum::extract::State;
use axum::response::Response;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::feed::{Feed, GetFeedsOptions};
use crate::partials::add_feed_form::add_feed_form;
use crate::partials::feed_list::feed_list;
use crate::partials::layout::Layout;
use crate::partials::opml_import_form::opml_import_form;

pub async fn get(State(pool): State<PgPool>, layout: Layout) -> Result<Response> {
    let options = GetFeedsOptions::default();
    let feeds = Feed::get_all(&pool, &options).await?;
    Ok(layout.with_subtitle("feeds").render(html! {
        header { h2 { "Feeds" } }
        div class="feeds" {
            ul id="feeds" {
                (feed_list(feeds, &options))
            }
            div class="add-feed" {
                h3 { "Add Feed" }
                (add_feed_form())
                (opml_import_form())
            }
        }
    }))
}
