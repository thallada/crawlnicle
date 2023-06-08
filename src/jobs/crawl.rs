use article_scraper::ArticleScraper;
use chrono::Utc;
use feed_rs::parser;
use reqwest::{Client, Url};
use sqlx::PgPool;
use tracing::{info, info_span, warn};

use crate::models::feed::get_feeds;
use crate::models::entry::{upsert_entries, CreateEntry};

/// For every feed in the database, fetches the feed, parses it, and saves new entries to the
/// database.
pub async fn crawl(pool: &PgPool) -> anyhow::Result<()> {
    let scraper = ArticleScraper::new(None).await;
    let client = Client::new();
    let feeds = get_feeds(pool).await?;
    for feed in feeds {
        let _feed_span = info_span!("feed", id = feed.id, url = feed.url.as_str());
        info!("Fetching feed");
        // TODO: handle these results
        let bytes = client.get(feed.url).send().await?.bytes().await?;
        info!("Parsing feed");
        let parsed_feed = parser::parse(&bytes[..])?;
        let mut payload = Vec::with_capacity(parsed_feed.entries.len());
        for entry in parsed_feed.entries {
            let _entry_span = info_span!("entry", id = entry.id, title = entry.title.clone().map(|t| t.content));
            if let Some(link) = entry.links.get(0) {
                // if no scraped or feed date is available, fallback to the current time
                let published_at = entry.published.unwrap_or_else(Utc::now).naive_utc();
                let mut entry = CreateEntry {
                    title: entry.title.map(|t| t.content),
                    url: link.href.clone(),
                    description: entry.summary.map(|s| s.content),
                    html_content: None,
                    feed_id: feed.id,
                    published_at,
                };
                info!("Fetching and parsing entry link: {}", link.href);
                if let Ok(article) = scraper.parse(&Url::parse(&link.href)?, true, &client, None).await {
                    if let Some(date) = article.date {
                        // prefer scraped date over rss feed date
                        entry.published_at = date.naive_utc()
                    };
                    entry.html_content = article.get_content();
                } else {
                    warn!("Failed to fetch article for entry: {:?}", link);
                }
                payload.push(entry);
            } else {
                warn!("Skipping feed entry with no links");
            }
        }
        let entries = upsert_entries(pool, payload).await?;
        info!("Created {} entries for feed {}", entries.len(), feed.id);
    }
    Ok(())
}
