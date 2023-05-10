use feed_rs::parser;
use reqwest::Client;
use sqlx::PgPool;
use tracing::info;

use crate::models::feed::get_feeds;
use crate::models::item::{upsert_items, CreateItem};

/// For every feed in the database, fetches the feed, parses it, and saves new items to the
/// database.
pub async fn crawl(pool: &PgPool) -> anyhow::Result<()> {
    let client = Client::new();
    let feeds = get_feeds(pool).await?;
    for feed in feeds {
        let bytes = client.get(feed.url).send().await?.bytes().await?;
        let parsed_feed = parser::parse(&bytes[..])?;
        let mut payload = Vec::with_capacity(parsed_feed.entries.len());
        for entry in parsed_feed.entries {
            let item = CreateItem {
                title: entry
                    .title
                    .map_or_else(|| "No title".to_string(), |t| t.content),
                url: entry
                    .links
                    .get(0)
                    .map_or_else(|| "https://example.com".to_string(), |l| l.href.clone()),
                description: entry.summary.map(|s| s.content),
                feed_id: feed.id,
            };
            payload.push(item);
        }
        let items = upsert_items(pool, payload).await?;
        info!("Created {} items for feed {}", items.len(), feed.id);
    }
    Ok(())
}
