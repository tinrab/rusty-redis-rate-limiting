use std::error::Error;

use rusty_redis_rate_limiting::rate_limiter::RateLimiter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut rate_limiter = RateLimiter::create("redis://127.0.0.1:6379/").await?;
    rate_limiter.record_sliding_window().await?;

    Ok(())
}
