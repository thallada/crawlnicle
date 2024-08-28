use std::cmp::Ordering;

use anyhow::{anyhow, Result};
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use chrono::{Duration, Utc};
use feed_rs::parser;
use http::{header, HeaderMap, StatusCode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, info_span, instrument, warn};
use url::Url;
use uuid::Uuid;

use crate::domain_request_limiter::DomainRequestLimiter;
use crate::jobs::{AsyncJob, CrawlEntryJob};
use crate::models::entry::{CreateEntry, Entry};
use crate::models::feed::{Feed, MAX_CRAWL_INTERVAL_MINUTES, MIN_CRAWL_INTERVAL_MINUTES};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CrawlFeedJob {
    pub feed_id: Uuid,
}

#[instrument(skip_all, fields(feed_id = %feed_id))]
pub async fn crawl_feed(
    CrawlFeedJob { feed_id }: CrawlFeedJob,
    http_client: Data<Client>,
    db: Data<PgPool>,
    domain_request_limiter: Data<DomainRequestLimiter>,
    apalis: Data<RedisStorage<AsyncJob>>,
) -> Result<()> {
    let mut feed = Feed::get(&*db, feed_id).await?;
    info!("got feed from db");
    let url = Url::parse(&feed.url)?;
    let domain = url
        .domain()
        .ok_or(anyhow!("invalid url: {:?}", feed.url.clone()))?;
    let mut headers = HeaderMap::new();
    if let Some(etag) = &feed.etag_header {
        if let Ok(etag) = etag.parse() {
            headers.insert(header::IF_NONE_MATCH, etag);
        } else {
            warn!(%etag, "failed to parse saved etag header");
        }
    }
    if let Some(last_modified) = &feed.last_modified_header {
        if let Ok(last_modified) = last_modified.parse() {
            headers.insert(header::IF_MODIFIED_SINCE, last_modified);
        } else {
            warn!(
                %last_modified,
                "failed to parse saved last_modified header",
            );
        }
    }

    info!(url=%url, "starting fetch");
    domain_request_limiter.acquire(domain).await?;
    let resp = http_client.get(url.clone()).headers(headers).send().await?;
    let headers = resp.headers();
    if let Some(etag) = headers.get(header::ETAG) {
        if let Ok(etag) = etag.to_str() {
            feed.etag_header = Some(etag.to_string());
        } else {
            warn!(?etag, "failed to convert response etag header to string");
        }
    }
    if let Some(last_modified) = headers.get(header::LAST_MODIFIED) {
        if let Ok(last_modified) = last_modified.to_str() {
            feed.last_modified_header = Some(last_modified.to_string());
        } else {
            warn!(
                ?last_modified,
                "failed to convert response last_modified header to string",
            );
        }
    }
    info!(url=%url, "fetched feed");
    if resp.status() == StatusCode::NOT_MODIFIED {
        info!("feed returned not modified status");
        feed.last_crawled_at = Some(Utc::now());
        feed.last_crawl_error = None;
        feed.save(&*db).await?;
        info!("updated feed in db");
        return Ok(());
    } else if !resp.status().is_success() {
        warn!("feed returned non-successful status");
        feed.last_crawled_at = Some(Utc::now());
        feed.last_crawl_error = resp.status().canonical_reason().map(|s| s.to_string());
        feed.save(&*db).await?;
        info!("updated feed in db");
        return Ok(());
    }

    let bytes = resp.bytes().await?;

    let parsed_feed = parser::parse(&bytes[..])?;
    info!("parsed feed");
    feed.url = url.to_string();
    feed.feed_type = parsed_feed.feed_type.into();
    feed.last_crawled_at = Some(Utc::now());
    feed.last_crawl_error = None;
    if let Some(title) = parsed_feed.title {
        feed.title = Some(title.content);
    }
    if let Some(description) = parsed_feed.description {
        feed.description = Some(description.content);
    }
    let last_entry_published_at = parsed_feed.entries.iter().filter_map(|e| e.published).max();
    if let Some(prev_last_entry_published_at) = feed.last_entry_published_at {
        if let Some(published_at) = last_entry_published_at {
            let time_since_last_entry = if published_at == prev_last_entry_published_at {
                // No new entry since last crawl, compare current time to last publish instead
                Utc::now() - prev_last_entry_published_at
            } else {
                // Compare new entry publish time to previous publish time
                published_at - prev_last_entry_published_at
            };
            match time_since_last_entry.cmp(&Duration::minutes(feed.crawl_interval_minutes.into()))
            {
                Ordering::Greater => {
                    feed.crawl_interval_minutes = i32::max(
                        (feed.crawl_interval_minutes as f32 * 1.2).ceil() as i32,
                        MAX_CRAWL_INTERVAL_MINUTES,
                    );
                    info!(
                        interval = feed.crawl_interval_minutes,
                        "increased crawl interval"
                    );
                }
                Ordering::Less => {
                    feed.crawl_interval_minutes = i32::max(
                        (feed.crawl_interval_minutes as f32 / 1.2).ceil() as i32,
                        MIN_CRAWL_INTERVAL_MINUTES,
                    );
                    info!(
                        interval = feed.crawl_interval_minutes,
                        "decreased crawl interval"
                    );
                }
                Ordering::Equal => {}
            }
        }
    }
    feed.last_entry_published_at = last_entry_published_at;
    let feed = feed.save(&*db).await?;
    info!("updated feed in db");

    let mut payload = Vec::with_capacity(parsed_feed.entries.len());
    for entry in parsed_feed.entries {
        let entry_span = info_span!("entry", id = entry.id);
        let _entry_span_guard = entry_span.enter();
        if let Some(link) = entry.links.first() {
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
            warn!("skipping feed entry with no links");
        }
    }
    let entries = Entry::bulk_upsert(&*db, payload).await?;
    let (new, updated) = entries
        .into_iter()
        .partition::<Vec<_>, _>(|entry| entry.updated_at.is_none());
    info!(new = new.len(), updated = updated.len(), "saved entries");

    for entry in new {
        (*apalis)
            .clone() // TODO: clone bad?
            .push(AsyncJob::CrawlEntry(CrawlEntryJob {
                entry_id: entry.entry_id,
            }))
            .await?;
    }
    Ok(())
}
