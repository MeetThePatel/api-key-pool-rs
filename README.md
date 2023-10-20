# api-key-pool.rs

Pool of API keys to circumvent rate-limits.

This package provides an easy way to use multiple API keys to bypass draconian rate-limit policies.

### Example

```rust
use chrono::Duration;
use tokio::time;

use api_key_pool::*;

#[tokio::main]
async fn main() {
    // Create a RateLimitPolicy to be applied to all API keys.
    // Note: An APIPool can have APIKeys with different RateLimitPolicies.
    //       For the sake of simplicity, this example assumes identical policies.
    let pol = RateLimitPolicy::new(1, Duration::seconds(2));

    // Create the APIKeys.
    let api1 = APIKey::new("1", pol);
    let api2 = APIKey::new("2", pol);
    let api3 = APIKey::new("3", pol);

    // Create the APIKeyPool.
    let mut pool = APIKeyPool::new();
    pool.add_key(api1).await;
    pool.add_key(api2).await;
    pool.add_key(api3).await;

    // Simulate 20 requests.
    let mut ctr = 0;
    while ctr < 20 {
        // Use the APIKey if available (according to its respective RateLimitPolicy) or sleep.
        if let Some(key) = pool.poll_for_key().await {
            println!("{}", key);
            ctr += 1;
        } else {
            println!("Have to sleep.");
            time::sleep(time::Duration::from_millis(500)).await;
        }
    }
}
```

