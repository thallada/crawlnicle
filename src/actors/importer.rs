use std::fmt::{self, Display, Formatter};
use std::io::Cursor;

use bytes::Bytes;
use opml::OPML;
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinSet;
use tracing::{debug, error, instrument};
use uuid::Uuid;

use crate::actors::crawl_scheduler::{CrawlSchedulerHandle, CrawlSchedulerHandleMessage};
use crate::error::Error;
use crate::models::feed::{Feed, CreateFeed};
use crate::state::Imports;
use crate::uuid::Base62Uuid;

/// The `Importer` actor parses OPML bytes, loops through the document to find all feed URLs, then
/// creates a DB entry for each and initiates a new crawl if the feed is new.
///
/// It receives `ImporterMessage` messages via the `receiver` channel. It communicates back to
/// the sender of those messages via the `respond_to` channel on the `ImporterMessage`.
///
/// `Importer` should not be instantiated directly. Instead, use the `ImporterHandle`.
struct Importer {
    receiver: mpsc::Receiver<ImporterMessage>,
    pool: PgPool,
    crawl_scheduler: CrawlSchedulerHandle,
    imports: Imports,
}

#[derive(Debug)]
enum ImporterMessage {
    Import {
        import_id: Uuid,
        file_name: Option<String>,
        bytes: Bytes,
        respond_to: broadcast::Sender<ImporterHandleMessage>,
    },
}

impl Display for ImporterMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ImporterMessage::Import {
                import_id, bytes, ..
            } => write!(f, "Import({}: {} bytes)", import_id, bytes.len()),
        }
    }
}

async fn listen_to_crawl(
    feed_id: Uuid,
    crawl_scheduler: CrawlSchedulerHandle,
    respond_to: broadcast::Sender<ImporterHandleMessage>,
) -> Uuid {
    let mut receiver = crawl_scheduler.schedule(feed_id).await;
    while let Ok(msg) = receiver.recv().await {
        let _ = respond_to.send(ImporterHandleMessage::CrawlScheduler(msg));
    }
    feed_id
}

/// An error type that enumerates possible failures during a crawl and is cloneable and can be sent
/// across threads (does not reference the originating Errors which are usually not cloneable).
#[derive(thiserror::Error, Debug, Clone)]
pub enum ImporterError {
    #[error("invalid OPML file: {0}")]
    InvalidOPML(String),
    #[error("failed to create feed: {0}")]
    CreateFeedError(String),
}
pub type ImporterResult<T, E = ImporterError> = ::std::result::Result<T, E>;

impl Importer {
    fn new(
        receiver: mpsc::Receiver<ImporterMessage>,
        pool: PgPool,
        crawl_scheduler: CrawlSchedulerHandle,
        imports: Imports,
    ) -> Self {
        Importer {
            receiver,
            pool,
            crawl_scheduler,
            imports,
        }
    }

    #[instrument(skip_all, fields(import_id = %import_id, file_name = ?file_name))]
    async fn import_opml(
        &self,
        import_id: Uuid,
        file_name: Option<String>,
        bytes: Bytes,
        respond_to: broadcast::Sender<ImporterHandleMessage>,
    ) -> ImporterResult<()> {
        let document = OPML::from_reader(&mut Cursor::new(bytes)).map_err(|_| {
            ImporterError::InvalidOPML(file_name.unwrap_or(Base62Uuid::from(import_id).to_string()))
        })?;
        let mut crawls = JoinSet::new();
        for url in Self::gather_feed_urls(document.body.outlines) {
            let feed = Feed::create(
                &self.pool,
                CreateFeed {
                    url: url.clone(),
                    ..Default::default()
                },
            )
            .await;
            if let Err(Error::Sqlx(sqlx::error::Error::Database(err))) = feed {
                if err.is_unique_violation() {
                    let _ = respond_to.send(ImporterHandleMessage::AlreadyImported(url));
                }
            } else if let Ok(feed) = feed {
                crawls.spawn(listen_to_crawl(
                    feed.feed_id,
                    self.crawl_scheduler.clone(),
                    respond_to.clone(),
                ));
            } else {
                let _ = respond_to.send(ImporterHandleMessage::CreateFeedError(url));
            }
        }

        while crawls.join_next().await.is_some() {}

        Ok(())
    }

    fn gather_feed_urls(outlines: Vec<opml::Outline>) -> Vec<String> {
        let mut urls = Vec::new();
        for outline in outlines.into_iter() {
            if let Some(url) = outline.xml_url {
                urls.push(url);
            }
            urls.append(&mut Self::gather_feed_urls(outline.outlines));
        }
        urls
    }

    #[instrument(skip_all, fields(msg = %msg))]
    async fn handle_message(&mut self, msg: ImporterMessage) {
        match msg {
            ImporterMessage::Import {
                import_id,
                file_name,
                bytes,
                respond_to,
            } => {
                let result = self
                    .import_opml(import_id, file_name, bytes, respond_to.clone())
                    .await;
                {
                    let mut imports = self.imports.lock().await;
                    imports.remove(&import_id);
                }

                // ignore the result since the initiator may have cancelled waiting for the
                // response, and that is ok
                let _ = respond_to.send(ImporterHandleMessage::Import(result));
            }
        }
    }

    #[instrument(skip_all)]
    async fn run(&mut self) {
        debug!("starting importer");
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }
}

/// The `ImporterHandle` is used to initialize and communicate with a `Importer` actor.
///
/// The `Importer` actor parses OPML bytes, loops through the document to find all feed URLs, then
/// creates a DB entry for each and initiates a new crawl if the feed is new.
#[derive(Clone)]
pub struct ImporterHandle {
    sender: mpsc::Sender<ImporterMessage>,
}

/// The `ImporterHandleMessage` is the response to a `ImporterMessage` sent to the
/// `ImporterHandle`.
///
/// `ImporterHandleMessage::Import` contains the result of importing the OPML file.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum ImporterHandleMessage {
    Import(ImporterResult<()>),
    CreateFeedError(String),
    AlreadyImported(String),
    CrawlScheduler(CrawlSchedulerHandleMessage),
}

impl ImporterHandle {
    /// Creates an async actor task that will listen for messages on the `sender` channel.
    pub fn new(pool: PgPool, crawl_scheduler: CrawlSchedulerHandle, imports: Imports) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let mut importer = Importer::new(receiver, pool, crawl_scheduler, imports);
        tokio::spawn(async move { importer.run().await });

        Self { sender }
    }

    /// Sends a `ImporterMessage::Import` message to the running `Importer` actor.
    ///
    /// Listen to the result of the import via the returned `broadcast::Receiver`.
    pub async fn import(
        &self,
        import_id: Uuid,
        file_name: Option<String>,
        bytes: Bytes,
    ) -> broadcast::Receiver<ImporterHandleMessage> {
        let (sender, receiver) = broadcast::channel(8);
        let msg = ImporterMessage::Import {
            import_id,
            file_name,
            bytes,
            respond_to: sender,
        };

        self.sender.send(msg).await.expect("importer task has died");
        receiver
    }
}
