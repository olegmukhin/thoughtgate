//! Token bucket rate limiter for Slack API calls.
//!
//! Implements: REQ-GOV-003/§5.3
//!
//! Provides a simple token bucket rate limiter to prevent exhausting
//! Slack API rate limits (typically 1 request/second for tier 3 methods).

use std::time::{Duration, Instant};
use tokio::sync::Mutex;

// ============================================================================
// Rate Limiter
// ============================================================================

/// Token bucket rate limiter.
///
/// Implements: REQ-GOV-003/§5.3
///
/// Limits the rate of API calls using a token bucket algorithm:
/// - Tokens accumulate at `refill_rate` per second up to `max_tokens`
/// - Each `acquire()` consumes one token
/// - If no tokens available, `acquire()` waits until one is available
pub struct RateLimiter {
    inner: Mutex<RateLimiterInner>,
}

struct RateLimiterInner {
    /// Current number of tokens
    tokens: f64,
    /// Maximum tokens (bucket capacity)
    max_tokens: f64,
    /// Tokens added per second
    refill_rate: f64,
    /// Last time tokens were refilled
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the specified rate (requests per second).
    ///
    /// Implements: REQ-GOV-003/§5.3
    ///
    /// # Arguments
    ///
    /// * `rate_per_second` - Maximum requests per second (e.g., 1.0 for Slack)
    #[must_use]
    pub fn new(rate_per_second: f64) -> Self {
        Self {
            inner: Mutex::new(RateLimiterInner {
                tokens: rate_per_second, // Start with full bucket
                max_tokens: rate_per_second,
                refill_rate: rate_per_second,
                last_refill: Instant::now(),
            }),
        }
    }

    /// Acquire a token, waiting if necessary.
    ///
    /// Implements: REQ-GOV-003/§5.3
    ///
    /// This method will block (async) until a token is available.
    /// It is cancel-safe.
    pub async fn acquire(&self) {
        loop {
            let wait_time = {
                let mut inner = self.inner.lock().await;

                // Refill tokens based on elapsed time
                let now = Instant::now();
                let elapsed = now.duration_since(inner.last_refill);
                inner.tokens += elapsed.as_secs_f64() * inner.refill_rate;
                inner.tokens = inner.tokens.min(inner.max_tokens);
                inner.last_refill = now;

                // Try to acquire a token
                if inner.tokens >= 1.0 {
                    inner.tokens -= 1.0;
                    return;
                }

                // Calculate wait time for one token
                let deficit = 1.0 - inner.tokens;
                Duration::from_secs_f64(deficit / inner.refill_rate)
            };

            // Wait and retry
            tokio::time::sleep(wait_time).await;
        }
    }

    /// Try to acquire a token without waiting.
    ///
    /// Returns `true` if a token was acquired, `false` otherwise.
    #[must_use]
    pub async fn try_acquire(&self) -> bool {
        let mut inner = self.inner.lock().await;

        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(inner.last_refill);
        inner.tokens += elapsed.as_secs_f64() * inner.refill_rate;
        inner.tokens = inner.tokens.min(inner.max_tokens);
        inner.last_refill = now;

        // Try to acquire a token
        if inner.tokens >= 1.0 {
            inner.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that the rate limiter allows burst up to capacity.
    ///
    /// Verifies: REQ-GOV-003/§5.3
    #[tokio::test]
    async fn test_rate_limiter_allows_burst() {
        let limiter = RateLimiter::new(10.0); // 10 per second

        // Should immediately allow 10 requests
        for _ in 0..10 {
            let start = Instant::now();
            limiter.acquire().await;
            assert!(start.elapsed() < Duration::from_millis(50));
        }
    }

    /// Tests that the rate limiter enforces the rate limit.
    ///
    /// Verifies: REQ-GOV-003/§5.3
    #[tokio::test]
    async fn test_rate_limiter_enforces_rate() {
        let limiter = RateLimiter::new(10.0); // 10 per second

        // Drain the bucket
        for _ in 0..10 {
            limiter.acquire().await;
        }

        // Next request should wait ~100ms
        let start = Instant::now();
        limiter.acquire().await;
        assert!(start.elapsed() >= Duration::from_millis(90));
    }

    /// Tests try_acquire returns false when bucket is empty.
    ///
    /// Verifies: REQ-GOV-003/§5.3
    #[tokio::test]
    async fn test_try_acquire_empty_bucket() {
        let limiter = RateLimiter::new(1.0); // 1 per second

        // First should succeed
        assert!(limiter.try_acquire().await);

        // Second should fail immediately
        assert!(!limiter.try_acquire().await);
    }

    /// Tests token refill over time.
    ///
    /// Verifies: REQ-GOV-003/§5.3
    #[tokio::test]
    async fn test_token_refill() {
        let limiter = RateLimiter::new(10.0); // 10 per second

        // Drain the bucket
        for _ in 0..10 {
            limiter.acquire().await;
        }

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should have ~2 tokens now
        assert!(limiter.try_acquire().await);
        assert!(limiter.try_acquire().await);
    }
}
