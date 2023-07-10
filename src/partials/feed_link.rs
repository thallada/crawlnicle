use maud::{html, Markup};

use crate::models::feed::Feed;
use crate::uuid::Base62Uuid;

pub fn feed_link(feed: &Feed, pending_crawl: bool) -> Markup {
    let title = feed.title.clone().unwrap_or_else(|| {
        if pending_crawl {
            "Crawling feed...".to_string()
        } else {
            "Untitled Feed".to_string()
        }
    });
    let feed_url = format!("/feed/{}", Base62Uuid::from(feed.feed_id));
    html! {
        a href=(feed_url) { (title) }
    }
}
