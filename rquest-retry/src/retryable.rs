use crate::retryable_strategy::{DefaultRetryableStrategy, RetryableStrategy};
use rquest_middleware::Error;

/// Classification of an error/status returned by request.
#[derive(PartialEq, Eq)]
pub enum Retryable {
    /// The failure was due to something that might resolve in the future.
    Transient,
    /// Unresolvable error.
    Fatal,
}

impl Retryable {
    /// Try to map a `rquest` response into `Retryable`.
    ///
    /// Returns `None` if the response object does not contain any errors.
    ///
    pub fn from_rquest_response(res: &Result<rquest::Response, Error>) -> Option<Self> {
        DefaultRetryableStrategy.handle(res)
    }
}

impl From<&rquest::Error> for Retryable {
    fn from(_status: &rquest::Error) -> Retryable {
        Retryable::Transient
    }
}
