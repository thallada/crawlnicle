use anyhow::{anyhow, Result};
use fred::{clients::RedisPool, interfaces::KeysInterface, prelude::*};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};

/// A Redis-based rate limiter for domain-specific requests with jittered retry delay.
///
/// This limiter uses a fixed window algorithm with a 1-second window and applies
/// jitter to the retry delay to help prevent synchronized retries in distributed systems.
/// It uses fred's RedisPool for efficient connection management.
///
/// Limitations:
/// 1. Fixed window: The limit resets every second, potentially allowing short traffic bursts
///    at window boundaries.
/// 2. No token bucket: Doesn't accumulate unused capacity from quiet periods.
/// 3. Potential overcounting: In distributed systems, there's a small chance of overcounting
///    near window ends due to race conditions.
/// 4. Redis dependency: Rate limiting fails open if Redis is unavailable.
/// 5. Blocking: The acquire method will block until a request is allowed or max_retries is reached.
///
/// Usage example:
/// ```
/// use fred::prelude::*;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let config = RedisConfig::default();
///     let pool = RedisPool::new(config, None, None, 5)?;
///     pool.connect();
///     pool.wait_for_connect().await?;
///
///     let limiter = DomainRequestLimiter::new(pool, 10, 5, 100, 0.5);
///     let domain = "example.com";
///
///     for _ in 0..15 {
///         match limiter.acquire(domain).await {
///             Ok(()) => println!("Request allowed"),
///             Err(_) => println!("Max retries reached, request denied"),
///         }
///     }
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct DomainRequestLimiter {
    redis_pool: RedisPool,
    requests_per_second: u32,
    max_retries: u32,
    base_retry_delay_ms: u64,
    jitter_factor: f64,
    // TODO: I think I can get rid of this if I instantiate a DomainRequestLimiter per-worker, but
    // I'm not sure how to do that in apalis (then I could just use thread_rng)
    rng: Arc<Mutex<SmallRng>>,
}

impl DomainRequestLimiter {
    /// Create a new DomainRequestLimiter.
    ///
    /// # Arguments
    /// * `redis_pool` - A fred RedisPool.
    /// * `requests_per_second` - Maximum allowed requests per second per domain.
    /// * `max_retries` - Maximum number of retries before giving up.
    /// * `base_retry_delay_ms` - Base delay between retries in milliseconds.
    /// * `jitter_factor` - Factor to determine the maximum jitter (0.0 to 1.0).
    pub fn new(
        redis_pool: RedisPool,
        requests_per_second: u32,
        max_retries: u32,
        base_retry_delay_ms: u64,
        jitter_factor: f64,
    ) -> Self {
        Self {
            redis_pool,
            requests_per_second,
            max_retries,
            base_retry_delay_ms,
            jitter_factor: jitter_factor.clamp(0.0, 1.0),
            rng: Arc::new(Mutex::new(SmallRng::from_entropy())),
        }
    }

    /// Attempt to acquire permission for a request, retrying if necessary.
    ///
    /// This method will attempt to acquire permission up to max_retries times,
    /// sleeping for a jittered delay between each attempt.
    ///
    /// # Arguments
    /// * `domain` - The domain for which to check the rate limit.
    ///
    /// # Returns
    /// Ok(()) if permission is granted, or an error if max retries are exceeded.
    pub async fn acquire(&self, domain: &str) -> Result<()> {
        for attempt in 0..=self.max_retries {
            if self.try_acquire(domain).await? {
                return Ok(());
            }
            if attempt < self.max_retries {
                let mut rng = self.rng.lock().await;
                let jitter =
                    rng.gen::<f64>() * self.jitter_factor * self.base_retry_delay_ms as f64;
                let delay = self.base_retry_delay_ms + jitter as u64;
                sleep(Duration::from_millis(delay)).await;
            }
        }
        Err(anyhow!(
            "Max retries exceeded for domain: {:?}, request denied",
            domain
        ))
    }

    async fn try_acquire(&self, domain: &str) -> Result<bool, RedisError> {
        let key = format!("rate_limit:{}", domain);

        let count: u32 = self.redis_pool.incr(&key).await?;
        if count == 1 {
            self.redis_pool.expire(&key, 1).await?;
        }

        Ok(count <= self.requests_per_second)
    }
}
