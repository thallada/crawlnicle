use feed_rs::parser;
use reqwest::Client;
use sqlx::PgPool;
use tracing::info;

use crate::models::feed::get_feeds;
use crate::models::entry::{upsert_entries, CreateEntry};

/// For every feed in the database, fetches the feed, parses it, and saves new entries to the
/// database.
pub async fn crawl(pool: &PgPool) -> anyhow::Result<()> {
    let client = Client::new();
    let feeds = get_feeds(pool).await?;
    for feed in feeds {
        let bytes = client.get(feed.url).send().await?.bytes().await?;
        let parsed_feed = parser::parse(&bytes[..])?;
        let mut payload = Vec::with_capacity(parsed_feed.entries.len());
        for entry in parsed_feed.entries {
            let entry = CreateEntry {
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
            payload.push(entry);
        }
        let entries = upsert_entries(pool, payload).await?;
        info!("Created {} entries for feed {}", entries.len(), feed.id);
    }
    Ok(())
}
