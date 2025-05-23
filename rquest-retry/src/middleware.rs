//! `RetryTransientMiddleware` implements retrying requests on transient errors.
use std::time::{Duration, SystemTime};

use crate::retryable_strategy::RetryableStrategy;
use crate::{retryable::Retryable, retryable_strategy::DefaultRetryableStrategy, RetryError};
use anyhow::anyhow;
use http::Extensions;
use retry_policies::RetryPolicy;
use rquest::Request;
use rquest_middleware::{Error, Middleware, Next, Result, Response};

#[doc(hidden)]
// We need this macro because tracing expects the level to be const:
// https://github.com/tokio-rs/tracing/issues/2730
#[cfg(feature = "tracing")]
macro_rules! log_retry {
    ($level:expr, $($args:tt)*) => {{
        match $level {
            ::tracing::Level::TRACE => ::tracing::trace!($($args)*),
            ::tracing::Level::DEBUG => ::tracing::debug!($($args)*),
            ::tracing::Level::INFO => ::tracing::info!($($args)*),
            ::tracing::Level::WARN => ::tracing::warn!($($args)*),
            ::tracing::Level::ERROR => ::tracing::error!($($args)*),
        }
    }};
}

/// `RetryTransientMiddleware` offers retry logic for requests that fail in a transient manner
/// and can be safely executed again.
///
/// Currently, it allows setting a [RetryPolicy] algorithm for calculating the __wait_time__
/// between each request retry. Sleeping on non-`wasm32` archs is performed using
/// [`tokio::time::sleep`], therefore it will respect pauses/auto-advance if run under a
/// runtime that supports them.
///
///```rust
///     use std::time::Duration;
///     use rquest_middleware::ClientBuilder;
///     use retry_policies::{RetryDecision, RetryPolicy, Jitter};
///     use retry_policies::policies::ExponentialBackoff;
///     use rquest_retry::RetryTransientMiddleware;
///     use rquest::Client;
///
///     // We create a ExponentialBackoff retry policy which implements `RetryPolicy`.
///     let retry_policy = ExponentialBackoff::builder()
///         .retry_bounds(Duration::from_secs(1), Duration::from_secs(60))
///         .jitter(Jitter::Bounded)
///         .base(2)
///         .build_with_total_retry_duration(Duration::from_secs(24 * 60 * 60));
///
///     let retry_transient_middleware = RetryTransientMiddleware::new_with_policy(retry_policy);
///     let client = ClientBuilder::new(Client::new()).with(retry_transient_middleware).build();
///```
///
/// # Note
///
/// This middleware always errors when given requests with streaming bodies, before even executing
/// the request. When this happens you'll get an [`Error::Middleware`] with the message
/// 'Request object is not cloneable. Are you passing a streaming body?'.
///
/// Some workaround suggestions:
/// * If you can fit the data in memory, you can instead build static request bodies e.g. with
///   `Body`'s `From<String>` or `From<Bytes>` implementations.
/// * You can wrap this middleware in a custom one which skips retries for streaming requests.
/// * You can write a custom retry middleware that builds new streaming requests from the data
///   source directly, avoiding the issue of streaming requests not being cloneable.
pub struct RetryTransientMiddleware<
    T: RetryPolicy + Send + Sync + 'static,
    R: RetryableStrategy + Send + Sync + 'static = DefaultRetryableStrategy,
> {
    retry_policy: T,
    retryable_strategy: R,
    #[cfg(feature = "tracing")]
    retry_log_level: tracing::Level,
}

impl<T: RetryPolicy + Send + Sync> RetryTransientMiddleware<T, DefaultRetryableStrategy> {
    /// Construct `RetryTransientMiddleware` with  a [retry_policy][RetryPolicy].
    pub fn new_with_policy(retry_policy: T) -> Self {
        Self::new_with_policy_and_strategy(retry_policy, DefaultRetryableStrategy)
    }

    /// Set the log [level][tracing::Level] for retry events.
    /// The default is [`WARN`][tracing::Level::WARN].
    #[cfg(feature = "tracing")]
    pub fn with_retry_log_level(mut self, level: tracing::Level) -> Self {
        self.retry_log_level = level;
        self
    }
}

impl<T, R> RetryTransientMiddleware<T, R>
where
    T: RetryPolicy + Send + Sync,
    R: RetryableStrategy + Send + Sync,
{
    /// Construct `RetryTransientMiddleware` with  a [retry_policy][RetryPolicy] and [retryable_strategy](RetryableStrategy).
    pub fn new_with_policy_and_strategy(retry_policy: T, retryable_strategy: R) -> Self {
        Self {
            retry_policy,
            retryable_strategy,
            #[cfg(feature = "tracing")]
            retry_log_level: tracing::Level::WARN,
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl<T, R> Middleware for RetryTransientMiddleware<T, R>
where
    T: RetryPolicy + Send + Sync,
    R: RetryableStrategy + Send + Sync + 'static,
{
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        // TODO: Ideally we should create a new instance of the `Extensions` map to pass
        // downstream. This will guard against previous retries polluting `Extensions`.
        // That is, we only return what's populated in the typemap for the last retry attempt
        // and copy those into the the `global` Extensions map.
        self.execute_with_retry(req, next, extensions).await
    }
}

impl<T, R> RetryTransientMiddleware<T, R>
where
    T: RetryPolicy + Send + Sync,
    R: RetryableStrategy + Send + Sync,
{
    /// This function will try to execute the request, if it fails
    /// with an error classified as transient it will call itself
    /// to retry the request.
    async fn execute_with_retry<'a>(
        &'a self,
        req: Request,
        next: Next<'a>,
        ext: &'a mut Extensions,
    ) -> Result<Response> {
        let mut n_past_retries = 0;
        let start_time = SystemTime::now();
        loop {
            // Cloning the request object before-the-fact is not ideal..
            // However, if the body of the request is not static, e.g of type `Bytes`,
            // the Clone operation should be of constant complexity and not O(N)
            // since the byte abstraction is a shared pointer over a buffer.
            let duplicate_request = req.try_clone().ok_or_else(|| {
                Error::Middleware(anyhow!(
                    "Request object is not cloneable. Are you passing a streaming body?"
                        .to_string()
                ))
            })?;

            let result = next.clone().run(duplicate_request, ext).await;

            // We classify the response which will return None if not
            // errors were returned.
            if let Some(Retryable::Transient) = self.retryable_strategy.handle(&result) {
                // If the response failed and the error type was transient
                // we can safely try to retry the request.
                let retry_decision = self.retry_policy.should_retry(start_time, n_past_retries);
                if let retry_policies::RetryDecision::Retry { execute_after } = retry_decision {
                    let duration = execute_after
                        .duration_since(SystemTime::now())
                        .unwrap_or_else(|_| Duration::default());
                    // Sleep the requested amount before we try again.
                    #[cfg(feature = "tracing")]
                    log_retry!(
                        self.retry_log_level,
                        "Retry attempt #{}. Sleeping {:?} before the next attempt",
                        n_past_retries,
                        duration
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(duration).await;
                    #[cfg(target_arch = "wasm32")]
                    wasmtimer::tokio::sleep(duration).await;

                    n_past_retries += 1;
                    continue;
                }
            };

            // Report whether we failed with or without retries.
            break if n_past_retries > 0 {
                result.map_err(|err| {
                    Error::Middleware(
                        RetryError::WithRetries {
                            retries: n_past_retries,
                            err,
                        }
                        .into(),
                    )
                })
            } else {
                result.map_err(|err| Error::Middleware(RetryError::Error(err).into()))
            };
        }
    }
}
