use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::Path;

use bytes::Buf;
use readability::extractor;
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc};
use tracing::{info, instrument};
use url::Url;

use crate::domain_locks::DomainLocks;
use crate::models::entry::Entry;

/// The `EntryCrawler` actor fetches an entry url, extracts the content, and saves the content to
/// the file system and any associated metadata to the database.
///
/// It receives `EntryCrawlerMessage` messages via the `receiver` channel. It communicates back to
/// the sender of those messages via the `respond_to` channel on the `EntryCrawlerMessage`.
///
/// `EntryCrawler` should not be instantiated directly. Instead, use the `EntryCrawlerHandle`.
struct EntryCrawler {
    receiver: mpsc::Receiver<EntryCrawlerMessage>,
    pool: PgPool,
    client: Client,
    domain_locks: DomainLocks,
    content_dir: String,
}

#[derive(Debug)]
enum EntryCrawlerMessage {
    Crawl {
        entry: Entry,
        respond_to: broadcast::Sender<EntryCrawlerHandleMessage>,
    },
}

impl Display for EntryCrawlerMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EntryCrawlerMessage::Crawl { entry, .. } => write!(f, "Crawl({})", entry.url),
        }
    }
}

/// An error type that enumerates possible failures during a crawl and is cloneable and can be sent
/// across threads (does not reference the originating Errors which are usually not cloneable).
#[derive(thiserror::Error, Debug, Clone)]
pub enum EntryCrawlerError {
    #[error("invalid entry url: {0}")]
    InvalidUrl(String),
    #[error("failed to fetch entry: {0}")]
    FetchError(String),
    #[error("failed to extract content for entry: {0}")]
    ExtractError(String),
    #[error("failed to create entry: {0}")]
    CreateEntryError(String),
    #[error("failed to save entry content: {0}")]
    SaveContentError(String),
}
pub type EntryCrawlerResult<T, E = EntryCrawlerError> = ::std::result::Result<T, E>;

impl EntryCrawler {
    fn new(
        receiver: mpsc::Receiver<EntryCrawlerMessage>,
        pool: PgPool,
        client: Client,
        domain_locks: DomainLocks,
        content_dir: String,
    ) -> Self {
        EntryCrawler {
            receiver,
            pool,
            client,
            domain_locks,
            content_dir,
        }
    }

    #[instrument(skip_all, fields(entry = %entry.url))]
    async fn crawl_entry(&self, entry: Entry) -> EntryCrawlerResult<Entry> {
        info!("Fetching and parsing entry");
        let content_dir = Path::new(&self.content_dir);
        let url =
            Url::parse(&entry.url).map_err(|_| EntryCrawlerError::InvalidUrl(entry.url.clone()))?;
        let domain = url
            .domain()
            .ok_or(EntryCrawlerError::InvalidUrl(entry.url.clone()))?;
        let bytes = self
            .domain_locks
            .run_request(domain, async {
                self.client
                    .get(url.clone())
                    .send()
                    .await
                    .map_err(|_| EntryCrawlerError::FetchError(entry.url.clone()))?
                    .bytes()
                    .await
                    .map_err(|_| EntryCrawlerError::FetchError(entry.url.clone()))
            })
            .await?;
        info!("fetched entry");
        let article = extractor::extract(&mut bytes.reader(), &url)
            .map_err(|_| EntryCrawlerError::ExtractError(entry.url.clone()))?;
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
        fs::write(content_dir.join(format!("{}.html", id)), article.content)
            .map_err(|_| EntryCrawlerError::SaveContentError(entry.url.clone()))?;
        fs::write(content_dir.join(format!("{}.txt", id)), article.text)
            .map_err(|_| EntryCrawlerError::SaveContentError(entry.url.clone()))?;
        info!("saved content to filesystem");
        Ok(entry)
    }

    #[instrument(skip_all, fields(msg = %msg))]
    async fn handle_message(&mut self, msg: EntryCrawlerMessage) {
        match msg {
            EntryCrawlerMessage::Crawl { entry, respond_to } => {
                let result = self.crawl_entry(entry).await;
                // ignore the result since the initiator may have cancelled waiting for the
                // response, and that is ok
                let _ = respond_to.send(EntryCrawlerHandleMessage::Entry(result));
            }
        }
    }

    #[instrument(skip_all)]
    async fn run(&mut self) {
        info!("starting entry crawler");
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }
}

/// The `EntryCrawlerHandle` is used to initialize and communicate with a `EntryCrawler` actor.
///
/// The `EntryCrawler` actor fetches a feed url, parses it, and saves it to the database. It runs
/// as a separate asynchronous task from the main web server and communicates via channels.
#[derive(Clone)]
pub struct EntryCrawlerHandle {
    sender: mpsc::Sender<EntryCrawlerMessage>,
}

/// The `EntryCrawlerHandleMessage` is the response to a `EntryCrawlerMessage` sent to the
/// `EntryCrawlerHandle`.
///
/// `EntryCrawlerHandleMessage::Entry` contains the result of crawling an entry url.
#[derive(Clone)]
pub enum EntryCrawlerHandleMessage {
    Entry(EntryCrawlerResult<Entry>),
}

impl EntryCrawlerHandle {
    /// Creates an async actor task that will listen for messages on the `sender` channel.
    pub fn new(
        pool: PgPool,
        client: Client,
        domain_locks: DomainLocks,
        content_dir: String,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let mut crawler = EntryCrawler::new(receiver, pool, client, domain_locks, content_dir);
        tokio::spawn(async move { crawler.run().await });

        Self { sender }
    }

    /// Sends a `EntryCrawlerMessage::Crawl` message to the running `EntryCrawler` actor.
    ///
    /// Listen to the result of the crawl via the returned `broadcast::Receiver`.
    pub async fn crawl(&self, entry: Entry) -> broadcast::Receiver<EntryCrawlerHandleMessage> {
        let (sender, receiver) = broadcast::channel(8);
        let msg = EntryCrawlerMessage::Crawl {
            entry,
            respond_to: sender,
        };

        self.sender
            .send(msg)
            .await
            .expect("entry crawler task has died");
        receiver
    }
}
