//! Middleware to retry failed HTTP requests built on [`rquest_middleware`].
//!
//! Use [`RetryTransientMiddleware`] to retry failed HTTP requests. Retry control flow is managed
//! by a [`RetryPolicy`].
//!
//! ## Example
//!
//! ```
//! use rquest_middleware::{ClientBuilder, ClientWithMiddleware};
//! use rquest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
//!
//! async fn run_retries() {
//!     // Retry up to 3 times with increasing intervals between attempts.
//!     let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
//!     let client = ClientBuilder::new(rquest::Client::new())
//!         .with(RetryTransientMiddleware::new_with_policy(retry_policy))
//!         .build();
//!
//!     client
//!         .get("https://truelayer.com")
//!         .header("foo", "bar")
//!         .send()
//!         .await
//!         .unwrap();
//! }
//! ```

mod middleware;
mod retryable;
mod retryable_strategy;

pub use retry_policies::{policies, Jitter, RetryDecision, RetryPolicy};
use thiserror::Error;

pub use middleware::RetryTransientMiddleware;
pub use retryable::Retryable;
pub use retryable_strategy::{
    default_on_request_failure, default_on_request_success, DefaultRetryableStrategy,
    RetryableStrategy,
};

/// Custom error type to attach the number of retries to the error message.
#[derive(Debug, Error)]
pub enum RetryError {
    #[error("Request failed after {retries} retries")]
    WithRetries {
        retries: u32,
        #[source]
        err: rquest_middleware::Error,
    },
    #[error(transparent)]
    Error(rquest_middleware::Error),
}
