use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{interval_at, Instant};
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

use crate::actors::feed_crawler::{FeedCrawlerError, FeedCrawlerHandle, FeedCrawlerHandleMessage};
use crate::domain_locks::DomainLocks;
use crate::models::feed::{Feed, GetFeedsOptions};
use crate::state::Crawls;

struct CrawlScheduler {
    receiver: mpsc::Receiver<CrawlSchedulerMessage>,
    pool: PgPool,
    client: Client,
    domain_locks: DomainLocks,
    content_dir: String,
    crawls: Crawls,
}

#[derive(Debug)]
enum CrawlSchedulerMessage {
    Schedule {
        feed_id: Uuid,
        respond_to: broadcast::Sender<CrawlSchedulerHandleMessage>,
    },
    Bootstrap {
        respond_to: broadcast::Sender<CrawlSchedulerHandleMessage>,
    },
}

impl Display for CrawlSchedulerMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CrawlSchedulerMessage::Schedule { feed_id, .. } => write!(f, "Schedule({})", feed_id),
            CrawlSchedulerMessage::Bootstrap { .. } => write!(f, "Bootstrap"),
        }
    }
}

/// An error type that enumerates possible failures during a crawl and is cloneable and can be sent
/// across threads (does not reference the originating Errors which are usually not cloneable).
#[derive(thiserror::Error, Debug, Clone)]
pub enum CrawlSchedulerError {
    #[error("failed to fetch feed from database: {0}")]
    FetchFeedError(String),
    #[error("failed to fetch feeds from database: {0}")]
    FetchFeedsError(String),
    #[error("failed to crawl feed: {0}")]
    FeedCrawlerError(FeedCrawlerError),
}
pub type CrawlSchedulerResult<T, E = CrawlSchedulerError> = ::std::result::Result<T, E>;

impl CrawlScheduler {
    fn new(
        receiver: mpsc::Receiver<CrawlSchedulerMessage>,
        pool: PgPool,
        client: Client,
        domain_locks: DomainLocks,
        content_dir: String,
        crawls: Crawls,
    ) -> Self {
        CrawlScheduler {
            receiver,
            pool,
            client,
            domain_locks,
            content_dir,
            crawls,
        }
    }

    #[instrument(skip_all)]
    async fn bootstrap(
        &self,
        respond_to: broadcast::Sender<CrawlSchedulerHandleMessage>,
    ) -> CrawlSchedulerResult<()> {
        debug!("scheduling crawlers");
        let mut options = GetFeedsOptions::default();
        loop {
            info!("fetching feeds before: {:?}", options.before);
            let feeds = match Feed::get_all(&self.pool, options.clone()).await {
                Err(err) => {
                    return Err(CrawlSchedulerError::FetchFeedsError(err.to_string()));
                }
                Ok(feeds) if feeds.is_empty() => {
                    info!("no more feeds found");
                    break;
                }
                Ok(feeds) => feeds,
            };
            info!("found {} feeds", feeds.len());
            options.before = feeds.last().map(|f| f.created_at);

            for feed in feeds.into_iter() {
                self.spawn_crawler_loop(feed, respond_to.clone());
            }
        }
        debug!("done scheduling crawlers");
        Ok(())
    }

    #[instrument(skip_all, fields(feed_id = %feed_id))]
    async fn schedule(
        &self,
        feed_id: Uuid,
        respond_to: broadcast::Sender<CrawlSchedulerHandleMessage>,
    ) -> CrawlSchedulerResult<()> {
        let feed = Feed::get(&self.pool, feed_id)
            .await
            .map_err(|err| CrawlSchedulerError::FetchFeedError(err.to_string()))?;
        self.spawn_crawler_loop(feed, respond_to);
        Ok(())
    }

    #[instrument(skip_all, fields(feed_id = %feed.feed_id))]
    fn spawn_crawler_loop(
        &self,
        feed: Feed,
        respond_to: broadcast::Sender<CrawlSchedulerHandleMessage>,
    ) {
        let crawl_interval = Duration::from_secs(feed.crawl_interval_minutes as u64 * 60);
        let mut interval = tokio::time::interval(crawl_interval);
        if let Some(last_crawled_at) = feed.last_crawled_at {
            if let Ok(duration_since_last_crawl) = (Utc::now() - last_crawled_at).to_std() {
                if duration_since_last_crawl < crawl_interval {
                    info!(
                        "last crawled at {:?}, crawling again in {:?}",
                        last_crawled_at,
                        crawl_interval - duration_since_last_crawl
                    );
                    interval = interval_at(
                        Instant::now() + (crawl_interval - duration_since_last_crawl),
                        crawl_interval,
                    );
                }
            }
        }
        let feed_crawler = FeedCrawlerHandle::new(
            self.pool.clone(),
            self.client.clone(),
            self.domain_locks.clone(),
            self.content_dir.clone(),
            self.crawls.clone(),
        );
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                let mut receiver = feed_crawler.crawl(feed.feed_id).await;
                while let Ok(msg) = receiver.recv().await {
                    match msg {
                        FeedCrawlerHandleMessage::Feed(Ok(feed)) => {
                            let crawl_interval =
                                Duration::from_secs(feed.crawl_interval_minutes as u64 * 60);
                            interval = interval_at(Instant::now() + crawl_interval, crawl_interval);
                            info!(
                                minutes = feed.crawl_interval_minutes,
                                "updated crawl interval"
                            );
                            let _ = respond_to.send(CrawlSchedulerHandleMessage::FeedCrawler(
                                FeedCrawlerHandleMessage::Feed(Ok(feed)),
                            ));
                        }
                        result => {
                            let _ =
                                respond_to.send(CrawlSchedulerHandleMessage::FeedCrawler(result));
                        }
                    }
                }
            }
        });
    }

    #[instrument(skip_all, fields(msg = %msg))]
    async fn handle_message(&mut self, msg: CrawlSchedulerMessage) {
        match msg {
            CrawlSchedulerMessage::Bootstrap { respond_to } => {
                let result = self.bootstrap(respond_to.clone()).await;
                if let Err(err) = &result {
                    error!("failed to bootstrap: {}", err);
                }

                // ignore the result since the initiator may have cancelled waiting for the
                // response, and that is ok
                let _ = respond_to.send(CrawlSchedulerHandleMessage::Bootstrap(result));
            }
            CrawlSchedulerMessage::Schedule {
                feed_id,
                respond_to,
            } => {
                let result = self.schedule(feed_id, respond_to.clone()).await;
                if let Err(err) = &result {
                    error!("failed to schedule: {}", err);
                }

                // ignore the result since the initiator may have cancelled waiting for the
                // response, and that is ok
                let _ = respond_to.send(CrawlSchedulerHandleMessage::Schedule(result));
            }
        }
    }

    #[instrument(skip_all)]
    async fn run(&mut self) {
        debug!("starting crawl scheduler");
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }
}

/// The `CrawlSchedulerHandle` is used to initialize and communicate with a `CrawlScheduler` actor.
///
/// Spawns an async task separate from the main web server that fetches all feeds from the database
/// and then spawns a long-lived async task for each feed that repeatedly crawls the feed at the
/// interval specified by each feeds' `crawl_interval_minutes`.
///
/// Initially, all feeds will immediately be crawled unless the `last_crawled_at` timestamp set in
/// the database is less than the current time minus its `crawl_interval` in which case the crawl
/// will be scheduled in the future.
///
/// After each crawl, the interval may be updated based on the result of the crawl.
#[derive(Clone)]
pub struct CrawlSchedulerHandle {
    sender: mpsc::Sender<CrawlSchedulerMessage>,
}

/// The `CrawlSchedulerHandleMessage` is the response to a `CrawlSchedulerMessage` sent to the
/// `CrawlSchedulerHandle`.
///
/// `CrawlSchedulerHandleMessage::Feed` contains the result of crawling a feed url.
#[derive(Debug, Clone)]
pub enum CrawlSchedulerHandleMessage {
    Bootstrap(CrawlSchedulerResult<()>),
    Schedule(CrawlSchedulerResult<()>),
    FeedCrawler(FeedCrawlerHandleMessage),
}

impl CrawlSchedulerHandle {
    /// Creates an async actor task that will listen for messages on the `sender` channel.
    pub fn new(
        pool: PgPool,
        client: Client,
        domain_locks: DomainLocks,
        content_dir: String,
        crawls: Crawls,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let mut scheduler =
            CrawlScheduler::new(receiver, pool, client, domain_locks, content_dir, crawls);
        tokio::spawn(async move { scheduler.run().await });

        Self { sender }
    }

    /// Sends a `CrawlSchedulerMessage::Bootstrap` message to the running `CrawlScheduler` actor.
    ///
    /// Listen to the result of the scheduling via the returned `broadcast::Receiver`.
    pub async fn bootstrap(&self) -> broadcast::Receiver<CrawlSchedulerHandleMessage> {
        let (sender, receiver) = broadcast::channel(8);
        let msg = CrawlSchedulerMessage::Bootstrap { respond_to: sender };

        self.sender
            .send(msg)
            .await
            .expect("crawl scheduler task has died");
        receiver
    }

    /// Sends a `CrawlSchedulerMessage::Schedule` message to the running `CrawlScheduler` actor.
    ///
    /// Listen to the result of the scheduling via the returned `broadcast::Receiver`.
    pub async fn schedule(
        &self,
        feed_id: Uuid,
    ) -> broadcast::Receiver<CrawlSchedulerHandleMessage> {
        let (sender, receiver) = broadcast::channel(8);
        let msg = CrawlSchedulerMessage::Schedule {
            feed_id,
            respond_to: sender,
        };

        self.sender
            .send(msg)
            .await
            .expect("crawl scheduler task has died");
        receiver
    }
}
