use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};
use tracing::debug;

pub type DomainLocksMap = Arc<Mutex<HashMap<String, Arc<Mutex<Instant>>>>>;

// TODO: make this configurable per domain and then load into a cache at startup
// bonus points if I also make it changeable at runtime, for example, if a domain returns a 429,
// then I can increase it and make sure it is saved back to the configuration for the next startup.
pub const DOMAIN_LOCK_DURATION: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct DomainLocks {
    map: DomainLocksMap,
}

/// A mechanism to serialize multiple async tasks requesting a single domain. To prevent
/// overloading servers with too many requests run in parallel at once, crawlnicle will only
/// request a domain once a second. All async tasks that wish to scrape a feed or entry must use
/// the `run_request` method on this struct to wait their turn.
///
/// Contains a map of domain names to a lock containing the timestamp of the last request to that
/// domain.
impl DomainLocks {
    pub fn new() -> Self {
        Self {
            map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Run the passed function `f` while holding a lock that gives exclusive access to the passed
    /// domain. If another task running `run_request` currently has the lock to the 
    /// `DomainLocksMap` or the lock to the domain passed, then this function will wait until that
    /// other task is done. Once it has access to the lock, if it has been less than one second 
    /// since the last request to the domain, then this function will sleep until one second has 
    /// passed before calling `f`.
    pub async fn run_request<F, T>(&self, domain: &str, f: F) -> T
    where
        F: Future<Output = T>,
    {
        let domain_last_request = {
            let mut map = self.map.lock().await;
            map.entry(domain.to_owned())
                .or_insert_with(|| Arc::new(Mutex::new(Instant::now() - DOMAIN_LOCK_DURATION)))
                .clone()
        };

        let mut domain_last_request = domain_last_request.lock().await;

        let elapsed = domain_last_request.elapsed();
        if elapsed < DOMAIN_LOCK_DURATION {
            let sleep_duration = DOMAIN_LOCK_DURATION - elapsed;
            debug!(
                domain,
                duration = format!("{} ms", sleep_duration.as_millis()),
                "sleeping before requesting domain",
            );
            sleep(DOMAIN_LOCK_DURATION - elapsed).await;
        }

        let result = f.await;

        *domain_last_request = Instant::now(); // Update the time of the last request.

        result
    }
}

impl Default for DomainLocks {
    fn default() -> Self {
        Self::new()
    }
}
