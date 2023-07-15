use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};

use chrono::{Duration, Utc};
use feed_rs::parser;
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc};
use tracing::log::warn;
use tracing::{error, info, info_span, instrument};
use url::Url;
use uuid::Uuid;

use crate::actors::entry_crawler::EntryCrawlerHandle;
use crate::domain_locks::DomainLocks;
use crate::models::entry::{CreateEntry, Entry};
use crate::models::feed::{Feed, MAX_CRAWL_INTERVAL_MINUTES, MIN_CRAWL_INTERVAL_MINUTES};
use crate::uuid::Base62Uuid;

/// The `FeedCrawler` actor fetches a feed url, parses it, and saves it to the database.
///
/// It receives `FeedCrawlerMessage` messages via the `receiver` channel. It communicates back to
/// the sender of those messages via the `respond_to` channel on the `FeedCrawlerMessage`.
///
/// `FeedCrawler` should not be instantiated directly. Instead, use the `FeedCrawlerHandle`.
struct FeedCrawler {
    receiver: mpsc::Receiver<FeedCrawlerMessage>,
    pool: PgPool,
    client: Client,
    domain_locks: DomainLocks,
    content_dir: String,
}

#[derive(Debug)]
enum FeedCrawlerMessage {
    Crawl {
        feed_id: Uuid,
        respond_to: broadcast::Sender<FeedCrawlerHandleMessage>,
    },
}

impl Display for FeedCrawlerMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FeedCrawlerMessage::Crawl { feed_id, .. } => write!(f, "Crawl({})", feed_id),
        }
    }
}

/// An error type that enumerates possible failures during a crawl and is cloneable and can be sent
/// across threads (does not reference the originating Errors which are usually not cloneable).
#[derive(thiserror::Error, Debug, Clone)]
pub enum FeedCrawlerError {
    #[error("invalid feed url: {0}")]
    InvalidUrl(String),
    #[error("failed to fetch feed: {0}")]
    FetchError(Url),
    #[error("failed to parse feed: {0}")]
    ParseError(Url),
    #[error("failed to find feed in database: {0}")]
    GetFeedError(Base62Uuid),
    #[error("failed to create feed: {0}")]
    CreateFeedError(Url),
    #[error("failed to create feed entries: {0}")]
    CreateFeedEntriesError(Url),
}
pub type FeedCrawlerResult<T, E = FeedCrawlerError> = ::std::result::Result<T, E>;

impl FeedCrawler {
    fn new(
        receiver: mpsc::Receiver<FeedCrawlerMessage>,
        pool: PgPool,
        client: Client,
        domain_locks: DomainLocks,
        content_dir: String,
    ) -> Self {
        FeedCrawler {
            receiver,
            pool,
            client,
            domain_locks,
            content_dir,
        }
    }

    #[instrument(skip_all, fields(feed_id = %feed_id))]
    async fn crawl_feed(&self, feed_id: Uuid) -> FeedCrawlerResult<Feed> {
        let mut feed = Feed::get(&self.pool, feed_id)
            .await
            .map_err(|_| FeedCrawlerError::GetFeedError(Base62Uuid::from(feed_id)))?;
        info!("got feed from db");
        let url =
            Url::parse(&feed.url).map_err(|_| FeedCrawlerError::InvalidUrl(feed.url.clone()))?;
        let domain = url
            .domain()
            .ok_or(FeedCrawlerError::InvalidUrl(feed.url.clone()))?;
        let bytes = self
            .domain_locks
            .run_request(domain, async {
                self.client
                    .get(url.clone())
                    .send()
                    .await
                    .map_err(|_| FeedCrawlerError::FetchError(url.clone()))?
                    .bytes()
                    .await
                    .map_err(|_| FeedCrawlerError::FetchError(url.clone()))
            })
            .await?;
        info!(url=%url, "fetched feed");
        let parsed_feed =
            parser::parse(&bytes[..]).map_err(|_| FeedCrawlerError::ParseError(url.clone()))?;
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
                let time_since_last_entry = published_at - prev_last_entry_published_at;
                match time_since_last_entry
                    .cmp(&Duration::minutes(feed.crawl_interval_minutes.into()))
                {
                    Ordering::Greater => {
                        feed.crawl_interval_minutes =
                            i32::max(feed.crawl_interval_minutes * 2, MAX_CRAWL_INTERVAL_MINUTES);
                    },
                    Ordering::Less => {
                        feed.crawl_interval_minutes =
                            i32::max(feed.crawl_interval_minutes / 2, MIN_CRAWL_INTERVAL_MINUTES);
                    },
                    Ordering::Equal => {},
                }
            }
        }
        let feed = feed
            .save(&self.pool)
            .await
            .map_err(|_| FeedCrawlerError::CreateFeedError(url.clone()))?;
        info!("updated feed in db");

        let mut payload = Vec::with_capacity(parsed_feed.entries.len());
        for entry in parsed_feed.entries {
            let entry_span = info_span!("entry", id = entry.id);
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
        let entries = Entry::bulk_upsert(&self.pool, payload)
            .await
            .map_err(|_| FeedCrawlerError::CreateFeedEntriesError(url.clone()))?;
        let (new, updated) = entries
            .into_iter()
            .partition::<Vec<_>, _>(|entry| entry.updated_at.is_none());
        info!(new = new.len(), updated = updated.len(), "saved entries");

        for entry in new {
            let entry_crawler = EntryCrawlerHandle::new(
                self.pool.clone(),
                self.client.clone(),
                self.domain_locks.clone(),
                self.content_dir.clone(),
            );
            // TODO: ignoring this receiver for the time being, pipe through events eventually
            let _ = entry_crawler.crawl(entry).await;
        }
        Ok(feed)
    }

    #[instrument(skip_all, fields(msg = %msg))]
    async fn handle_message(&mut self, msg: FeedCrawlerMessage) {
        match msg {
            FeedCrawlerMessage::Crawl {
                feed_id,
                respond_to,
            } => {
                let result = self.crawl_feed(feed_id).await;
                if let Err(error) = &result {
                    match Feed::update_crawl_error(&self.pool, feed_id, format!("{}", error)).await {
                        Ok(_) => info!("updated feed last_crawl_error"),
                        Err(e) => error!("failed to update feed last_crawl_error: {}", e),
                    }
                }
                
                // ignore the result since the initiator may have cancelled waiting for the
                // response, and that is ok
                let _ = respond_to.send(FeedCrawlerHandleMessage::Feed(result));
            }
        }
    }

    #[instrument(skip_all)]
    async fn run(&mut self) {
        info!("starting feed crawler");
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }
}

/// The `FeedCrawlerHandle` is used to initialize and communicate with a `FeedCrawler` actor.
///
/// The `FeedCrawler` actor fetches a feed url, parses it, and saves it to the database. It runs as
/// a separate asynchronous task from the main web server and communicates via channels.
#[derive(Clone)]
pub struct FeedCrawlerHandle {
    sender: mpsc::Sender<FeedCrawlerMessage>,
}

/// The `FeedCrawlerHandleMessage` is the response to a `FeedCrawlerMessage` sent to the
/// `FeedCrawlerHandle`.
///
/// `FeedCrawlerHandleMessage::Feed` contains the result of crawling a feed url.
/// `FeedCrawlerHandleMessage::Entry` contains the result of crawling an entry url within the feed.
#[derive(Clone)]
pub enum FeedCrawlerHandleMessage {
    Feed(FeedCrawlerResult<Feed>),
    Entry(FeedCrawlerResult<Entry>),
}

impl FeedCrawlerHandle {
    /// Creates an async actor task that will listen for messages on the `sender` channel.
    pub fn new(
        pool: PgPool,
        client: Client,
        domain_locks: DomainLocks,
        content_dir: String,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let mut crawler = FeedCrawler::new(receiver, pool, client, domain_locks, content_dir);
        tokio::spawn(async move { crawler.run().await });

        Self { sender }
    }

    /// Sends a `FeedCrawlerMessage::Crawl` message to the running `FeedCrawler` actor.
    ///
    /// Listen to the result of the crawl via the returned `broadcast::Receiver`.
    pub async fn crawl(&self, feed_id: Uuid) -> broadcast::Receiver<FeedCrawlerHandleMessage> {
        let (sender, receiver) = broadcast::channel(8);
        let msg = FeedCrawlerMessage::Crawl {
            feed_id,
            respond_to: sender,
        };

        self.sender
            .send(msg)
            .await
            .expect("feed crawler task has died");
        receiver
    }
}