use maud::{html, Markup};

use crate::models::feed::{Feed, GetFeedsOptions, DEFAULT_FEEDS_PAGE_SIZE};
use crate::partials::feed_link::feed_link;

pub fn feed_list(feeds: Vec<Feed>, options: GetFeedsOptions) -> Markup {
    let len = feeds.len() as i64;
    html! {
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
    }
}
