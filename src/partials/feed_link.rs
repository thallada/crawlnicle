use maud::{html, Markup};

use crate::models::feed::Feed;
use crate::partials::link::{link, LinkProps};
use crate::uuid::Base62Uuid;

pub struct FeedLink<'a> {
    pub feed: &'a Feed,
    pub pending_crawl: bool,
    pub reset_htmx_target: bool,
}

impl FeedLink<'_> {
    pub fn new(feed: &Feed) -> FeedLink {
        FeedLink {
            feed,
            pending_crawl: false,
            reset_htmx_target: false,
        }
    }

    pub fn pending_crawl(&mut self) -> &mut Self {
        self.pending_crawl = true;
        self
    }

    pub fn reset_htmx_target(&mut self) -> &mut Self {
        self.reset_htmx_target = true;
        self
    }

    pub fn render(&self) -> Markup {
        let title = self.feed.title.clone().unwrap_or_else(|| {
            if self.pending_crawl {
                "Crawling feed...".to_string()
            } else {
                "Untitled Feed".to_string()
            }
        });
        let feed_url = format!("/feed/{}", Base62Uuid::from(self.feed.feed_id));
        html! {
            (link(LinkProps { destination: &feed_url, title: &title, reset_htmx_target: self.reset_htmx_target }))
        }
    }
}

pub fn feed_link(feed: &Feed) -> Markup {
    FeedLink::new(feed).render()
}
