use std::error::Error;
use std::time;
use std::time::SystemTime;

use redis::aio::Connection;

const KEY_PREFIX: &str = "rate-limit";

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
        resource: &str,
        size_secs: u64,
    ) -> Result<u64, Box<dyn Error>> {
        let now_secs = SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let current_period = (now_secs / size_secs) * size_secs;
        let key = format!("{}:{}:{}:{}", KEY_PREFIX, resource, subject, current_period);

        let (count,): (u64,) = redis::pipe()
            .atomic()
            .cmd("INCRBY")
            .arg(&key)
            .arg(1)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(size_secs)
            .ignore()
            .query_async(&mut self.conn)
            .await?;

        Ok(count)
    }

    pub async fn record_sliding_window(
        &mut self,
        subject: &str,
        resource: &str,
        size_secs: u64,
    ) -> Result<u64, Box<dyn Error>> {
        let key = format!("{}:{}:{}", KEY_PREFIX, resource, subject);
        let now_millis = SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let (count,): (u64,) = redis::pipe()
            .atomic()
            .cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg(0)
            .arg(now_millis - size_secs * 1000)
            .ignore()
            .cmd("ZADD")
            .arg(&key)
            .arg(now_millis)
            .arg(now_millis)
            .ignore()
            .cmd("ZCARD")
            .arg(&key)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(size_secs)
            .ignore()
            .query_async(&mut self.conn)
            .await?;

        Ok(count)
    }
}
