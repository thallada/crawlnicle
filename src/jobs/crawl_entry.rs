use std::fs;
use std::path::Path;

use ammonia::clean;
use anyhow::{anyhow, Result};
use apalis::prelude::*;
use bytes::Buf;
use fred::prelude::*;
use readability::extractor;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, instrument};
use url::Url;
use uuid::Uuid;

use crate::config::Config;
use crate::domain_request_limiter::DomainRequestLimiter;
use crate::models::entry::Entry;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CrawlEntryJob {
    pub entry_id: Uuid,
}

#[instrument(skip_all, fields(entry_id = %entry_id))]
pub async fn crawl_entry(
    CrawlEntryJob { entry_id }: CrawlEntryJob,
    http_client: Data<Client>,
    db: Data<PgPool>,
    domain_request_limiter: Data<DomainRequestLimiter>,
    config: Data<Config>,
    redis: Data<RedisPool>,
) -> Result<()> {
    let entry = Entry::get(&*db, entry_id).await?;
    info!("got entry from db");
    let content_dir = Path::new(&*config.content_dir);
    let url = Url::parse(&entry.url)?;
    let domain = url
        .domain()
        .ok_or(anyhow!("invalid url: {:?}", entry.url.clone()))?;
    info!(url=%url, "starting fetch");
    domain_request_limiter.acquire(domain).await?;
    let bytes = http_client.get(url.clone()).send().await?.bytes().await?;
    info!(url=%url, "fetched entry");
    let article = extractor::extract(&mut bytes.reader(), &url)?;
    info!("extracted content");
    let id = entry.entry_id;
    // TODO: update entry with scraped data
    // if let Some(date) = article.date {
    //     // prefer scraped date over rss feed date
    //     let mut updated_entry = entry.clone();
    //     updated_entry.published_at = date;
    //     entry = update_entry(&self.pool, updated_entry)
    //         .await
    //         .map_err(|_| EntryCrawlerError::CreateEntryError(entry.url.clone()))?;
    // };
    let content = clean(&article.content);
    info!("sanitized content");
    fs::write(content_dir.join(format!("{}.html", id)), content)?;
    fs::write(content_dir.join(format!("{}.txt", id)), article.text)?;
    info!("saved content to filesystem");
    redis
        .next()
        .publish("entries", entry_id.to_string())
        .await?;
    Ok(())
}
