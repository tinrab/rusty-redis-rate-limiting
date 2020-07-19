use std::error::Error;
use std::time;
use std::time::{Duration, SystemTime};

use rusty_redis_rate_limiting::rate_limiter::RateLimiter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut rate_limiter = RateLimiter::open("redis://127.0.0.1:6379/").await?;
    let size = Duration::from_secs(1);

    for _ in 0..5 {
        rate_limiter
            .record_sliding_window("test", "user1", size)
            .await?;
    }
    let count = rate_limiter
        .fetch_sliding_window("test", "user1", size)
        .await?;
    assert_eq!(count, 5);

    tokio::time::delay_for(size).await;

    rate_limiter
        .record_sliding_window("test", "user1", size)
        .await?;
    let count = rate_limiter
        .fetch_sliding_window("test", "user1", size)
        .await?;

    let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
    let expected_count = RateLimiter::sliding_window_count(Some(5), Some(1), now, size);
    assert_eq!(count, expected_count);

    Ok(())
}
