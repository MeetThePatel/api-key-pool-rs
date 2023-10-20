//! Pool of API keys to circumvent rate-limits.
//!
//! This package provides an easy way to use multiple API keys to bypass draconian rate-limit policies.
//!
//! # Example
//!
//! ```
//! use chrono::Duration;
//! use tokio::time;
//!
//! use api_key_pool::*;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a RateLimitPolicy to be applied to all API keys.
//!     // Note: An APIPool can have APIKeys with different RateLimitPolicies.
//!     //       For the sake of simplicity, this example assumes identical policies.
//!     let pol = RateLimitPolicy::new(1, Duration::seconds(2));
//!
//!     // Create the APIKeys.
//!     let api1 = APIKey::new("1", pol);
//!     let api2 = APIKey::new("2", pol);
//!     let api3 = APIKey::new("3", pol);
//!
//!     // Create the APIKeyPool.
//!     let mut pool = APIKeyPool::new();
//!     pool.add_key(api1).await;
//!     pool.add_key(api2).await;
//!     pool.add_key(api3).await;
//!
//!     // Simulate 20 requests.
//!     let mut ctr = 0;
//!     while ctr < 20 {
//!         // Use the APIKey if available (according to its respective RateLimitPolicy) or sleep.
//!         if let Some(key) = pool.poll_for_key().await {
//!             println!("{}", key);
//!             ctr += 1;
//!         } else {
//!             println!("Have to sleep.");
//!             time::sleep(time::Duration::from_millis(500)).await;
//!         }
//!     }
//! }
//! ```


use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

/// A pool of API keys.
#[derive(Default)]
pub struct APIKeyPool {
    /// Collection holding the API keys.
    api_keys: Arc<Mutex<Vec<APIKey>>>,
}

impl APIKeyPool {
    /// Returns an empty API key pool.
    pub fn new() -> Self {
        Self {
            api_keys: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Adds an API key to an API key pool.
    ///
    /// # Arguments
    ///
    /// * `key` - the API key to be added.
    pub async fn add_key(&mut self, key: APIKey) {
        self.api_keys.lock().await.push(key);
    }

    /// Checks the API key pool for any available API keys, and returns the API key if available.
    pub async fn poll_for_key(&mut self) -> Option<String> {
        // TODO: Performance can be improved by keeping track of index of last used key.
        for key in &mut self.api_keys.lock().await.iter_mut() {
            if key.is_ready().await {
                return Some(key.use_key().await);
            }
        }
        None
    }
}

/// An API key, with its associated RateLimitPolicy
pub struct APIKey {
    /// The API key code.
    key: String,
    /// The rate limit policy that governs this API key.
    policy: RateLimitPolicy,
    /// Min-heap used to calculate if the key is available.
    times: Arc<Mutex<BinaryHeap<Reverse<DateTime<Utc>>>>>,
}

impl APIKey {
    /// Returns an API key with the given policy and code.
    ///
    /// # Arguments
    ///
    /// * `key` - the API key code.
    /// * `policy` - the rate limit policy governing the API key.
    pub fn new(key: &str, policy: RateLimitPolicy) -> Self {
        let mut _times = BinaryHeap::new();
        _times.reserve(policy.count);
        let times = Arc::new(Mutex::new(_times));
        Self {
            key: String::from(key),
            policy,
            times,
        }
    }

    /// Returns the code of an API key.
    fn get_key(&self) -> String {
        self.key.clone()
    }

    /// Checks to see if the API key is available for use.
    async fn is_ready(&self) -> bool {
        // If we have used the API key less than N times, we can use it again.
        if self.times.lock().await.len() < self.policy.count {
            return true;
        }
        if let Some(oldest) = self.times.lock().await.peek() {
            // If the oldest time used is at least D duration ago.
            if oldest.0 < Utc::now() - self.policy.per {
                return true;
            }
        }
        false
    }

    /// Uses the key.
    async fn use_key(&mut self) -> String {
        if self.times.lock().await.len() >= self.policy.count {
            self.times.lock().await.pop();
        }
        self.times.lock().await.push(Reverse(Utc::now()));
        self.get_key().clone()
    }
}

/// A policy for rate-limiting an API key.
#[derive(Clone, Copy)]
pub struct RateLimitPolicy {
    /// The number of times an API key can be used in the specified duration.
    pub count: usize,
    /// The duration.
    pub per: chrono::Duration,
}

impl RateLimitPolicy {
    /// Returns a rate-limit policy with the parameters.
    ///
    /// # Arguments
    ///
    /// * `count` - N times
    /// * `per` - per D duration
    pub fn new(count: usize, per: chrono::Duration) -> Self {
        Self { count, per }
    }
}
