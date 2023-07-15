use axum::extract::State;
use axum::response::Response;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::feed::{Feed, GetFeedsOptions, DEFAULT_FEEDS_PAGE_SIZE};
use crate::partials::{feed_link::feed_link, layout::Layout};

pub async fn get(State(pool): State<PgPool>, layout: Layout) -> Result<Response> {
    let options = GetFeedsOptions::default();
    let feeds = Feed::get_all(&pool, options.clone()).await?;
    let len = feeds.len() as i64;
    Ok(layout.render(html! {
        h2 { "Feeds" }
        div class="feeds" {
            div class="feeds-list" {
                @if len == 0 {
                    p id="no-feeds" { "No feeds found." }
                } else {
                    ul id="feeds" {
                        @for feed in feeds {
                            li { (feed_link(&feed, false)) }
                        }
                    }
                }
                // TODO: pagination
                @if len == options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE) {
                    button id="load-more-feeds" { "Load More" }
                }
            }
            div class="add-feed" {
                h3 { "Add Feed" }
                form action="/feed" method="post" class="add-feed-form" {
                    div class="form-grid" {
                        label for="url" { "URL (required): " }
                        input type="text" id="url" name="url" placeholder="https://example.com/feed.xml" required="true";
                        label for="title" { "Title: " }
                        input type="text" id="title" name="title" placeholder="Feed title";
                        label { "Description: " }
                        textarea id="description" name="description" placeholder="Feed description" {}
                    }
                    button type="submit" { "Add Feed" }
                }
            } 
        }
    }))
}
