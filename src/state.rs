use tokio::sync::watch::Receiver;

use axum::extract::FromRef;
use bytes::Bytes;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub log_receiver: Receiver<Bytes>,
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

impl FromRef<AppState> for Receiver<Bytes> {
    fn from_ref(state: &AppState) -> Self {
        state.log_receiver.clone()
    }
}
