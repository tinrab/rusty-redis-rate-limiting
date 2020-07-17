use std::error::Error;
use std::time::Duration;

use rusty_redis_rate_limiting::rate_limiter::RateLimiter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut rate_limiter = RateLimiter::create("redis://127.0.0.1:6379/").await?;
    let size = Duration::from_secs(1);

    for _ in 0..3 {
        rate_limiter
            .record_sliding_log("test", "user1", size)
            .await?;
        tokio::time::delay_for(Duration::from_millis(300)).await;
    }
    let count = rate_limiter.fetch_sliding_log("test", "user1").await?;
    assert_eq!(count, 3);

    tokio::time::delay_for(Duration::from_millis(600)).await;
    let count = rate_limiter
        .record_sliding_log("test", "user1", size)
        .await?;
    assert_eq!(count, 2);

    Ok(())
}
