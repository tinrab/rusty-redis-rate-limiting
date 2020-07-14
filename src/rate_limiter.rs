use std::error::Error;
use std::time;
use std::time::SystemTime;

use redis::aio::Connection;

const KEY_PREFIX: &str = "ratelimit";

pub struct RateLimiter {
    conn: Connection,
}

impl RateLimiter {
    pub async fn create(redis_address: &str) -> Result<Self, Box<dyn Error>> {
        let client = redis::Client::open(redis_address).unwrap();
        let conn = client.get_async_connection().await?;

        Ok(RateLimiter { conn })
    }

    pub async fn record_fixed_window(
        &mut self,
        subject: &str,
        bucket: &str,
        period_secs: u64,
    ) -> Result<u64, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let current_period = (now / period_secs) * period_secs;
        let key = format!("{}:{}:{}:{}", KEY_PREFIX, bucket, subject, current_period);

        let (count,): (u64,) = redis::pipe()
            .atomic()
            .cmd("INCRBY")
            .arg(&key)
            .arg(1)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(period_secs)
            .ignore()
            .query_async(&mut self.conn)
            .await?;

        Ok(count)
    }

    pub async fn record_sliding_window(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
