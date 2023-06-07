use article_scraper::ArticleScraper;
use feed_rs::parser;
use reqwest::{Client, Url};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::feed::get_feeds;
use crate::models::entry::{upsert_entries, CreateEntry};

/// For every feed in the database, fetches the feed, parses it, and saves new entries to the
/// database.
pub async fn crawl(pool: &PgPool) -> anyhow::Result<()> {
    let scraper = ArticleScraper::new(None).await;
    let client = Client::new();
    let feeds = get_feeds(pool).await?;
    for feed in feeds {
        info!("Fetching feed {}: {}", feed.id, feed.url);
        let bytes = client.get(feed.url).send().await?.bytes().await?;
        let parsed_feed = parser::parse(&bytes[..])?;
        let mut payload = Vec::with_capacity(parsed_feed.entries.len());
        for entry in parsed_feed.entries {
            if let Some(link) = entry.links.get(0) {
                info!("Fetching entry article: {}", link.href);
                let article = scraper.parse(&Url::parse(&link.href)?, true, &client, None).await?;
                let entry = CreateEntry {
                    title: entry.title.map(|t| t.content),
                    url: link.href.clone(),
                    description: entry.summary.map(|s| s.content),
                    html_content: article.get_content(),
                    feed_id: feed.id,
                };
                payload.push(entry);
            } else {
                warn!("Feed entry has no links: {:?}", entry);
            }
        }
        let entries = upsert_entries(pool, payload).await?;
        info!("Created {} entries for feed {}", entries.len(), feed.id);
    }
    Ok(())
}
