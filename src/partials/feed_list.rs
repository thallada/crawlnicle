use maud::{html, Markup};

use crate::models::feed::{Feed, GetFeedsOptions, DEFAULT_FEEDS_PAGE_SIZE};
use crate::partials::feed_link::feed_link;

pub fn feed_list(feeds: Vec<Feed>, options: &GetFeedsOptions) -> Markup {
    let len = feeds.len() as i64;
    if len == 0 {
        return html! { p { "No feeds found." } };
    }

    let mut more_query = None;
    if len == options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE) {
        let last_feed = feeds.last().unwrap();
        more_query = Some(format!(
            "/api/v1/feeds?sort=CreatedAt&before={}&id_before={}",
            last_feed.created_at,
            last_feed.feed_id
        ));
    }

    html! {
        @for (i, feed) in feeds.iter().enumerate() {
            @if i == feeds.len() - 1 {
                @if let Some(ref more_query) = more_query {
                    li class="feed" hx-get=(more_query) hx-trigger="revealed" hx-swap="afterend" {
                        (feed_link(feed, false))
                        div class="htmx-indicator list-loading" {
                            img class="loading" src="/static/img/three-dots.svg" alt="Loading...";
                        }
                    }
                } @else {
                    li class="feed" { (feed_link(feed, false)) }
                }
            } @else {
                li class="feed" { (feed_link(feed, false)) }
            }
        }
    }
}
