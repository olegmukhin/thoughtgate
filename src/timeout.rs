//! Timeout wrapper for HTTP bodies to prevent slow-drip attacks.
//!
//! # Traceability
//! - Implements: REQ-CORE-001 F-005 (Timeout Handling)

use bytes::Bytes;
use http_body::{Body, Frame};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::{sleep, Sleep};

/// Timeout configuration for streaming bodies.
///
/// # Traceability
/// - Implements: REQ-CORE-001 F-005 (Timeout Handling)
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Timeout for reading/writing each chunk
    pub chunk_timeout: Duration,
    /// Total timeout for the entire stream
    pub total_timeout: Duration,
}

impl TimeoutConfig {
    /// Create a new timeout configuration.
    pub fn new(chunk_timeout: Duration, total_timeout: Duration) -> Self {
        Self {
            chunk_timeout,
            total_timeout,
        }
    }
}

/// Wrapper that adds timeout enforcement to a body stream.
///
/// This wrapper ensures that:
/// - Each chunk read/write completes within `chunk_timeout`
/// - The total stream duration doesn't exceed `total_timeout`
///
/// # Traceability
/// - Implements: REQ-CORE-001 F-005 (Timeout Handling)
pub struct TimeoutBody<B> {
    inner: B,
    config: TimeoutConfig,
    chunk_timeout: Pin<Box<Sleep>>,
    total_timeout: Pin<Box<Sleep>>,
    started: bool,
}

impl<B> TimeoutBody<B> {
    /// Create a new timeout-wrapped body.
    pub fn new(inner: B, config: TimeoutConfig) -> Self {
        Self {
            inner,
            config: config.clone(),
            chunk_timeout: Box::pin(sleep(config.chunk_timeout)),
            total_timeout: Box::pin(sleep(config.total_timeout)),
            started: false,
        }
    }

    /// Get a reference to the timeout configuration.
    pub fn config(&self) -> &TimeoutConfig {
        &self.config
    }
}

impl<B> Body for TimeoutBody<B>
where
    B: Body<Data = Bytes> + Unpin,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Data = Bytes;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = &mut *self;

        // Start total timeout on first poll
        if !this.started {
            this.started = true;
            let deadline = tokio::time::Instant::now() + this.config.total_timeout;
            this.total_timeout.as_mut().reset(deadline);
        }

        // Check total timeout first
        if this.total_timeout.as_mut().poll(cx).is_ready() {
            let timeout_duration = this.config.total_timeout;
            return Poll::Ready(Some(Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("Total stream timeout exceeded ({:?})", timeout_duration),
            )
            .into())));
        }

        // Reset chunk timeout for this poll
        let chunk_deadline = tokio::time::Instant::now() + this.config.chunk_timeout;
        this.chunk_timeout.as_mut().reset(chunk_deadline);

        // Poll inner body with chunk timeout
        match Pin::new(&mut this.inner).poll_frame(cx) {
            Poll::Ready(result) => Poll::Ready(result.map(|r| r.map_err(|e| e.into()))),
            Poll::Pending => {
                // Check if chunk timeout expired
                if this.chunk_timeout.as_mut().poll(cx).is_ready() {
                    let timeout_duration = this.config.chunk_timeout;
                    Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("Chunk timeout exceeded ({:?})", timeout_duration),
                    )
                    .into())))
                } else {
                    Poll::Pending
                }
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_timeout_body_forwards_data() {
        let data = Bytes::from("test data");
        let body = Full::new(data.clone());
        let config = TimeoutConfig::new(Duration::from_secs(1), Duration::from_secs(5));

        let timeout_body = TimeoutBody::new(body, config);

        // Collect all frames
        let collected = timeout_body.collect().await.unwrap().to_bytes();

        assert_eq!(collected, data);
    }

    #[tokio::test]
    async fn test_timeout_config() {
        // Test timeout configuration
        let config = TimeoutConfig::new(Duration::from_secs(5), Duration::from_secs(60));
        assert_eq!(config.chunk_timeout, Duration::from_secs(5));
        assert_eq!(config.total_timeout, Duration::from_secs(60));
    }
}
