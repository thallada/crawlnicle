use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::FromRef;
use bytes::Bytes;
use lettre::SmtpTransport;
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::{broadcast, watch, Mutex};
use uuid::Uuid;

use crate::actors::importer::{ImporterHandle, ImporterHandleMessage};
use crate::actors::crawl_scheduler::{CrawlSchedulerHandle, CrawlSchedulerHandleMessage};
use crate::config::Config;
use crate::domain_locks::DomainLocks;

/// A map of feed IDs to a channel receiver for the active `CrawlScheduler` running a feed crawl
/// for that feed.
///
/// Currently, the only purpose of this is to keep track of active crawls so that axum handlers can
/// subscribe to the result of the crawl via the receiver channel which are then sent to end-users
/// as a stream of server-sent events.
///
/// This map should only contain crawls that have just been created but not yet subscribed to.
/// Entries are only added when a user adds a feed in the UI and entries are removed by the same
/// user once a server-sent event connection is established.
pub type Crawls = Arc<Mutex<HashMap<Uuid, broadcast::Receiver<CrawlSchedulerHandleMessage>>>>;

/// A map of unique import IDs to a channel receiver for the active `Importer` running that import.
///
/// Same as the `Crawls` map, the only purpose of this is to keep track of active imports so that 
/// axum handlers can subscribe to the result of the import via the receiver channel which are then 
/// sent to end-users as a stream of server-sent events.
///
/// This map should only contain imports that have just been created but not yet subscribed to.
/// Entries are only added when a user adds uploads an OPML to import and entries are removed by 
/// the same user once a server-sent event connection is established.
pub type Imports = Arc<Mutex<HashMap<Uuid, broadcast::Receiver<ImporterHandleMessage>>>>;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub log_receiver: watch::Receiver<Bytes>,
    pub crawls: Crawls,
    pub domain_locks: DomainLocks,
    pub client: Client,
    pub crawl_scheduler: CrawlSchedulerHandle,
    pub importer: ImporterHandle,
    pub imports: Imports,
    pub mailer: SmtpTransport,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl FromRef<AppState> for watch::Receiver<Bytes> {
    fn from_ref(state: &AppState) -> Self {
        state.log_receiver.clone()
    }
}

impl FromRef<AppState> for Crawls {
    fn from_ref(state: &AppState) -> Self {
        state.crawls.clone()
    }
}

impl FromRef<AppState> for DomainLocks {
    fn from_ref(state: &AppState) -> Self {
        state.domain_locks.clone()
    }
}

impl FromRef<AppState> for Client {
    fn from_ref(state: &AppState) -> Self {
        state.client.clone()
    }
}

impl FromRef<AppState> for CrawlSchedulerHandle {
    fn from_ref(state: &AppState) -> Self {
        state.crawl_scheduler.clone()
    }
}

impl FromRef<AppState> for ImporterHandle {
    fn from_ref(state: &AppState) -> Self {
        state.importer.clone()
    }
}

impl FromRef<AppState> for Imports {
    fn from_ref(state: &AppState) -> Self {
        state.imports.clone()
    }
}

impl FromRef<AppState> for SmtpTransport {
    fn from_ref(state: &AppState) -> Self {
        state.mailer.clone()
    }
}
