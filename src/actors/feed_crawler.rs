use std::fmt::{self, Display, Formatter};

use feed_rs::parser;
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc};
use tracing::{info, instrument};
use url::Url;

use crate::models::entry::Entry;
use crate::models::feed::{upsert_feed, CreateFeed, Feed};

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
}

#[derive(Debug)]
enum FeedCrawlerMessage {
    Crawl {
        url: Url,
        respond_to: broadcast::Sender<FeedCrawlerHandleMessage>,
    },
}

impl Display for FeedCrawlerMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FeedCrawlerMessage::Crawl { url, .. } => write!(f, "Crawl({})", url),
        }
    }
}

/// An error type that enumerates possible failures during a crawl and is cloneable and can be sent 
/// across threads (does not reference the originating Errors which are usually not cloneable).
#[derive(thiserror::Error, Debug, Clone)]
pub enum FeedCrawlerError {
    #[error("failed to fetch feed: {0}")]
    FetchError(Url),
    #[error("failed to parse feed: {0}")]
    ParseError(Url),
    #[error("failed to create feed: {0}")]
    CreateFeedError(Url),
}
pub type FeedCrawlerResult<T, E = FeedCrawlerError> = ::std::result::Result<T, E>;

impl FeedCrawler {
    fn new(receiver: mpsc::Receiver<FeedCrawlerMessage>, pool: PgPool, client: Client) -> Self {
        FeedCrawler {
            receiver,
            pool,
            client,
        }
    }

    #[instrument(skip_all, fields(url = %url))]
    async fn crawl_feed(&self, url: Url) -> FeedCrawlerResult<Feed> {
        let bytes = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|_| FeedCrawlerError::FetchError(url.clone()))?
            .bytes()
            .await
            .map_err(|_| FeedCrawlerError::FetchError(url.clone()))?;
        info!("fetched feed");
        let parsed_feed =
            parser::parse(&bytes[..]).map_err(|_| FeedCrawlerError::ParseError(url.clone()))?;
        info!("parsed feed");
        let feed = upsert_feed(
            &self.pool,
            CreateFeed {
                title: parsed_feed.title.map(|text| text.content),
                url: url.to_string(),
                feed_type: parsed_feed.feed_type.into(),
                description: parsed_feed.description.map(|text| text.content),
            },
        )
        .await
        .map_err(|_| FeedCrawlerError::CreateFeedError(url.clone()))?;
        info!(%feed.feed_id, "upserted feed");
        Ok(feed)
    }

    #[instrument(skip_all, fields(msg = %msg))]
    async fn handle_message(&mut self, msg: FeedCrawlerMessage) {
        match msg {
            FeedCrawlerMessage::Crawl { url, respond_to } => {
                let result = self.crawl_feed(url).await;
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
/// `FeedCrawlerHandleMessage::Entry` contains the result of crawling an entry url.
#[derive(Clone)]
pub enum FeedCrawlerHandleMessage {
    Feed(FeedCrawlerResult<Feed>),
    Entry(FeedCrawlerResult<Entry>),
}

impl FeedCrawlerHandle {
    /// Creates an async actor task that will listen for messages on the `sender` channel.
    pub fn new(pool: PgPool, client: Client) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let mut crawler = FeedCrawler::new(receiver, pool, client);
        tokio::spawn(async move { crawler.run().await });

        Self { sender }
    }

    /// Sends a `FeedCrawlerMessage::Crawl` message to the running `FeedCrawler` actor.
    ///
    /// Listen to the result of the crawl via the returned `broadcast::Receiver`.
    pub async fn crawl(&self, url: Url) -> broadcast::Receiver<FeedCrawlerHandleMessage> {
        let (sender, receiver) = broadcast::channel(8);
        let msg = FeedCrawlerMessage::Crawl {
            url,
            respond_to: sender,
        };

        self.sender
            .send(msg)
            .await
            .expect("feed crawler task has died");
        receiver
    }
}
