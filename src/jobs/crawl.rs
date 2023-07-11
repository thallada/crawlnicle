use std::fs;
use std::env;
use std::path::Path;

use article_scraper::ArticleScraper;
use chrono::Utc;
use feed_rs::parser;
use reqwest::{Client, Url};
use sqlx::PgPool;
use tracing::{info, info_span, warn};

use crate::models::feed::get_feeds;
use crate::models::entry::{update_entry, upsert_entries, CreateEntry};
use crate::uuid::Base62Uuid;

/// DEPRECATED: Use FeedCrawler instead, keeping this for reference until I set up scheduled jobs.
/// For every feed in the database, fetches the feed, parses it, and saves new entries to the
/// database.
pub async fn crawl(pool: &PgPool) -> anyhow::Result<()> {
    let scraper = ArticleScraper::new(None).await;
    let client = Client::new();
    let content_dir = env::var("CONTENT_DIR")?;
    let content_dir = Path::new(&content_dir);
    let feeds = get_feeds(pool).await?;
    for feed in feeds {
        let feed_id_str: String = Base62Uuid::from(feed.feed_id).into();
        let feed_span = info_span!("feed", id = feed_id_str, url = feed.url.as_str());
        let _feed_span_guard = feed_span.enter();
        info!("Fetching feed");
        // TODO: handle these results
        let bytes = client.get(feed.url).send().await?.bytes().await?;
        info!("Parsing feed");
        let parsed_feed = parser::parse(&bytes[..])?;
        let mut payload = Vec::with_capacity(parsed_feed.entries.len());
        for entry in parsed_feed.entries {
            let entry_span = info_span!("entry", id = entry.id, title = entry.title.clone().map(|t| t.content));
            let _entry_span_guard = entry_span.enter();
            if let Some(link) = entry.links.get(0) {
                // if no scraped or feed date is available, fallback to the current time
                let published_at = entry.published.unwrap_or_else(Utc::now);
                let entry = CreateEntry {
                    title: entry.title.map(|t| t.content),
                    url: link.href.clone(),
                    description: entry.summary.map(|s| s.content),
                    feed_id: feed.feed_id,
                    published_at,
                };
                payload.push(entry);
            } else {
                warn!("Skipping feed entry with no links");
            }
        }
        let entries = upsert_entries(pool, payload).await?;
        info!("Created {} entries", entries.len());

        // TODO: figure out how to do this in parallel. ArticleScraper uses some libxml thing that 
        // doesn't implement Send so this isn't trivial.
        for mut entry in entries {
            info!("Fetching and parsing entry link: {}", entry.url);
            if let Ok(article) = scraper.parse(&Url::parse(&entry.url)?, true, &client, None).await {
                let id = entry.entry_id;
                if let Some(date) = article.date {
                    // prefer scraped date over rss feed date
                    entry.published_at = date;
                    update_entry(pool, entry).await?;
                };
                let html_content = article.get_content();
                if let Some(content) = html_content {
                    fs::write(content_dir.join(format!("{}.html", id)), content)?;
                }
            } else {
                warn!("Failed to fetch article for entry: {:?}", &entry.url);
            }
        }
    }
    Ok(())
}
