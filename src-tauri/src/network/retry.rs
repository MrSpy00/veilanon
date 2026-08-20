//! Retry logic with exponential backoff for network operations
#![allow(dead_code)] // Scaffold module — wired by future UI-facing commands

use std::time::Duration;
use tokio::time::sleep;
use tracing::{warn, debug};
use crate::error::{VeilError, VeilResult};

/// Retry a future with exponential backoff
/// Max retries: 5, starting from 1s, up to 32s
pub async fn with_retry<F, Fut, T>(mut f: F) -> VeilResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = VeilResult<T>>,
{
    let max_retries = 5u32;
    let mut delay = Duration::from_secs(1);

    for attempt in 0..=max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(VeilError::RateLimitError) => {
                if attempt < max_retries {
                    warn!("Rate limited, waiting {}s before retry", delay.as_secs());
                    sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(60));
                }
            }
            Err(VeilError::NetworkError(e)) => {
                if attempt < max_retries {
                    debug!("Network error on attempt {}, retrying in {}s", attempt + 1, delay.as_secs());
                    sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(32));
                } else {
                    return Err(VeilError::NetworkError(e));
                }
            }
            Err(e) => return Err(e), // Non-retryable errors
        }
    }
    
    Err(VeilError::NetworkError(
        reqwest::get("").await.unwrap_err() // placeholder — real impl returns proper error
    ))
}
