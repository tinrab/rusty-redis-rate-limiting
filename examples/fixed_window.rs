use std::error::Error;
use std::time::Duration;

use rusty_redis_rate_limiting::rate_limiter::RateLimiter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut rate_limiter = RateLimiter::create("redis://127.0.0.1:6379/").await?;
    let size = Duration::from_secs(1);

    for i in 1..=3 {
        let count = rate_limiter
            .record_fixed_window("test", "user1", size)
            .await?;
        assert_eq!(count, i);
    }

    tokio::time::delay_for(size).await;

    let count = rate_limiter
        .record_fixed_window("test", "user1", size)
        .await?;
    assert_eq!(count, 1);

    Ok(())
}
