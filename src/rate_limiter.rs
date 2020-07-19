use std::error::Error;
use std::time;
use std::time::{Duration, SystemTime};

use redis::aio::Connection;
use redis::AsyncCommands;

const KEY_PREFIX: &str = "rate-limit";

pub struct RateLimiter {
    conn: Connection,
}

impl RateLimiter {
    pub async fn open(redis_address: &str) -> Result<Self, Box<dyn Error>> {
        let client = redis::Client::open(redis_address).unwrap();
        let conn = client.get_async_connection().await?;
        Ok(RateLimiter { conn })
    }

    /// Returns the count in the current time window.
    pub async fn fetch_fixed_window(
        &mut self,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, Box<dyn Error>> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let key = format!("{}:{}:{}:{}", KEY_PREFIX, resource, subject, window);

        let count: u64 = self.conn.get(key).await?;
        Ok(count)
    }

    /// Records an access to `resource` by `subject` with fixed window algorithm.
    /// Size of time window equals the `size` in seconds.
    pub async fn record_fixed_window(
        &mut self,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, Box<dyn Error>> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let key = format!("{}:{}:{}:{}", KEY_PREFIX, resource, subject, window);

        let (count,): (u64,) = redis::pipe()
            .atomic()
            .incr(&key, 1)
            .expire(&key, size.as_secs() as usize)
            .ignore()
            .query_async(&mut self.conn)
            .await?;
        Ok(count)
    }

    /// Returns the log's count.
    pub async fn fetch_sliding_log(
        &mut self,
        resource: &str,
        subject: &str,
    ) -> Result<u64, Box<dyn Error>> {
        let key = format!("{}:{}:{}", KEY_PREFIX, resource, subject);

        let count: u64 = self.conn.zcard(key).await?;
        Ok(count)
    }

    /// Records an access to `resource` by `subject` with sliding log algorithm.
    /// Size of time window equals the `size` in seconds.
    pub async fn record_sliding_log(
        &mut self,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, Box<dyn Error>> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let key = format!("{}:{}:{}", KEY_PREFIX, resource, subject);

        let (count,): (u64,) = redis::pipe()
            .atomic()
            .zrembyscore(&key, 0, (now.as_millis() - size.as_millis()) as u64)
            .ignore()
            .zadd(&key, now.as_millis() as u64, now.as_millis() as u64)
            .ignore()
            .zcard(&key)
            .expire(&key, size.as_secs() as usize)
            .ignore()
            .query_async(&mut self.conn)
            .await?;
        Ok(count)
    }

    /// Returns the count in the current time window.
    pub async fn fetch_sliding_window(
        &mut self,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, Box<dyn Error>> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let size_secs = size.as_secs();

        let current_window = (now.as_secs() / size_secs) * size_secs;
        let previous_window = (now.as_secs() / size_secs) * size_secs - size_secs;
        let current_key = format!("{}:{}:{}:{}", KEY_PREFIX, resource, subject, current_window);
        let previous_key = format!(
            "{}:{}:{}:{}",
            KEY_PREFIX, resource, subject, previous_window
        );

        let (previous_count, current_count): (Option<u64>, Option<u64>) =
            self.conn.get(vec![previous_key, current_key]).await?;
        Ok(Self::sliding_window_count(
            previous_count,
            current_count,
            now,
            size,
        ))
    }

    /// Records an access to `resource` by `subject` with sliding window algorithm.
    /// Size of time window equals the `size` in seconds.
    pub async fn record_sliding_window(
        &mut self,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, Box<dyn Error>> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let size_secs = size.as_secs();

        let current_window = (now.as_secs() / size_secs) * size_secs;
        let previous_window = (now.as_secs() / size_secs) * size_secs - size_secs;
        let current_key = format!("{}:{}:{}:{}", KEY_PREFIX, resource, subject, current_window);
        let previous_key = format!(
            "{}:{}:{}:{}",
            KEY_PREFIX, resource, subject, previous_window
        );

        let (previous_count, current_count): (Option<u64>, Option<u64>) = redis::pipe()
            .atomic()
            .get(&previous_key)
            .incr(&current_key, 1)
            .expire(&current_key, (size_secs * 2) as usize)
            .ignore()
            .query_async(&mut self.conn)
            .await?;
        Ok(Self::sliding_window_count(
            previous_count,
            current_count,
            now,
            size,
        ))
    }

    /// Calculates weighted count based on previous and current time windows.
    pub fn sliding_window_count(
        previous: Option<u64>,
        current: Option<u64>,
        now: Duration,
        size: Duration,
    ) -> u64 {
        let current_window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let next_window = current_window + size.as_secs();
        let weight = (Duration::from_secs(next_window).as_millis() - now.as_millis()) as f64
            / size.as_millis() as f64;
        current.unwrap_or(0) + (previous.unwrap_or(0) as f64 * weight).round() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_sliding_window_count() {
        assert_eq!(
            RateLimiter::sliding_window_count(
                Some(5),
                Some(3),
                Duration::from_millis(1800),
                Duration::from_secs(1)
            ),
            4
        );
    }
}
