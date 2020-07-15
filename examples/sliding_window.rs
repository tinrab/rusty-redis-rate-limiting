use std::error::Error;

use rusty_redis_rate_limiting::rate_limiter::RateLimiter;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut rate_limiter = RateLimiter::create("redis://127.0.0.1:6379/").await?;

    for _ in 0..3 {
        rate_limiter
            .record_sliding_window("user1", "test", 1)
            .await?;
    }
    let count = rate_limiter
        .record_sliding_window("user1", "test", 1)
        .await?;
    assert_eq!(count, 2);

    for _ in 0..10 {
        rate_limiter
            .record_sliding_window("user1", "test", 1)
            .await?;
    }
    tokio::time::delay_for(Duration::from_secs(1)).await;
    let count = rate_limiter
        .record_sliding_window("user1", "test", 1)
        .await?;
    assert_eq!(count, 1);

    Ok(())
}
