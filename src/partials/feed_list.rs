use maud::{html, Markup};

use crate::models::feed::{Feed, GetFeedsOptions, DEFAULT_FEEDS_PAGE_SIZE};
use crate::partials::feed_link::{feed_link, FeedLink};

pub fn feed_list(feeds: Vec<Feed>, options: &GetFeedsOptions, first_page: bool) -> Markup {
    let len = feeds.len() as i64;
    if first_page && len == 0 {
        return html! { p { "No feeds found." } };
    }

    let mut more_query = None;
    let limit = options.limit.unwrap_or(DEFAULT_FEEDS_PAGE_SIZE);
    if len == limit {
        let last_feed = feeds.last().unwrap();
        more_query = Some(format!(
            "/api/v1/feeds?sort=CreatedAt&before={}&id_before={}&limit={}",
            last_feed.created_at, last_feed.feed_id, limit
        ));
    }

    html! {
        @for (i, feed) in feeds.iter().enumerate() {
            @if i == feeds.len() - 1 {
                @if let Some(ref more_query) = more_query {
                    li hx-get=(more_query) hx-trigger="revealed" hx-target="this" hx-swap="afterend" {
                        (FeedLink::new(feed).reset_htmx_target().render())
                        div class="list-loading" {
                            img class="mt-4 max-h-4 invert" src="/static/img/three-dots.svg" alt="Loading...";
                        }
                    }
                } @else {
                    li { (feed_link(feed)) }
                }
            } @else {
                li { (feed_link(feed)) }
            }
        }
    }
}
