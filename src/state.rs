use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::{broadcast, watch};

use axum::extract::FromRef;
use bytes::Bytes;
use sqlx::PgPool;
use uuid::Uuid;

use crate::actors::feed_crawler::FeedCrawlerHandleMessage;
use crate::config::Config;
use crate::domain_locks::DomainLocks;

/// A map of feed IDs to a channel receiver for the active `FeedCrawler` running a crawl for that 
/// feed.
///
/// Currently, the only purpose of this is to keep track of active crawls so that axum handlers can 
/// subscribe to the result of the crawl via the receiver channel which are then sent to end-users 
/// as a stream of server-sent events.
/// 
/// This map should only contain crawls that have just been created but not yet subscribed to. 
/// Entries are only added when a user adds a feed in the UI and entries are removed by the same 
/// user once a server-sent event connection is established.
pub type Crawls = Arc<Mutex<HashMap<Uuid, broadcast::Receiver<FeedCrawlerHandleMessage>>>>;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub log_receiver: watch::Receiver<Bytes>,
    pub crawls: Crawls,
    pub domain_locks: DomainLocks,
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
