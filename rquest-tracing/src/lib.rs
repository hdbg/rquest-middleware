//! Opentracing middleware implementation for [`rquest_middleware`].
//!
//! Attach [`TracingMiddleware`] to your client to automatically trace HTTP requests.
//!
//! The simplest possible usage:
//! ```no_run
//! # use rquest_middleware::Result;
//! use rquest_middleware::{ClientBuilder};
//! use rquest_tracing::TracingMiddleware;
//!
//! # async fn example() -> Result<()> {
//! let rquest_client = rquest::Client::builder().build().unwrap();
//! let client = ClientBuilder::new(rquest_client)
//!    // Insert the tracing middleware
//!    .with(TracingMiddleware::default())
//!    .build();
//!
//! let resp = client.get("https://truelayer.com").send().await.unwrap();
//! # Ok(())
//! # }
//! ```
//!
//! To customise the span names use [`OtelName`].
//! ```no_run
//! # use rquest_middleware::Result;
//! use rquest_middleware::{ClientBuilder, Extension};
//! use rquest_tracing::{
//!     TracingMiddleware, OtelName
//! };
//! # async fn example() -> Result<()> {
//! let rquest_client = rquest::Client::builder().build().unwrap();
//! let client = ClientBuilder::new(rquest_client)
//!    // Inserts the extension before the request is started
//!    .with_init(Extension(OtelName("my-client".into())))
//!    // Makes use of that extension to specify the otel name
//!    .with(TracingMiddleware::default())
//!    .build();
//!
//! let resp = client.get("https://truelayer.com").send().await.unwrap();
//!
//! // Or specify it on the individual request (will take priority)
//! let resp = client.post("https://api.truelayer.com/payment")
//!     .with_extension(OtelName("POST /payment".into()))
//!    .send()
//!    .await
//!    .unwrap();
//! # Ok(())
//! # }
//! ```
//!
//! In this example we define a custom span builder to calculate the request time elapsed and we register the [`TracingMiddleware`].
//!
//! Note that Opentelemetry tracks start and stop already, there is no need to have a custom builder like this.
//! ```rust
//! use rquest_middleware::Result;
//! use http::Extensions;
//! use rquest::{Request, Response};
//! use rquest_middleware::ClientBuilder;
//! use rquest_tracing::{
//!     default_on_request_end, rquest_otel_span, ReqwestOtelSpanBackend, TracingMiddleware
//! };
//! use tracing::Span;
//! use std::time::{Duration, Instant};
//!
//! pub struct TimeTrace;
//!
//! impl ReqwestOtelSpanBackend for TimeTrace {
//!     fn on_request_start(req: &Request, extension: &mut Extensions) -> Span {
//!         extension.insert(Instant::now());
//!         rquest_otel_span!(name="example-request", req, time_elapsed = tracing::field::Empty)
//!     }
//!
//!     fn on_request_end(span: &Span, outcome: &Result<Response>, extension: &mut Extensions) {
//!         let time_elapsed = extension.get::<Instant>().unwrap().elapsed().as_millis() as i64;
//!         default_on_request_end(span, outcome);
//!         span.record("time_elapsed", &time_elapsed);
//!     }
//! }
//!
//! let http = ClientBuilder::new(rquest::Client::new())
//!     .with(TracingMiddleware::<TimeTrace>::new())
//!     .build();
//! ```

mod middleware;
#[cfg(any(
    feature = "opentelemetry_0_20",
    feature = "opentelemetry_0_21",
    feature = "opentelemetry_0_22",
    feature = "opentelemetry_0_23",
    feature = "opentelemetry_0_24",
    feature = "opentelemetry_0_25",
    feature = "opentelemetry_0_26",
    feature = "opentelemetry_0_27",
    feature = "opentelemetry_0_28",
    feature = "opentelemetry_0_29",
))]
mod otel;
mod rquest_otel_span_builder;
pub use middleware::TracingMiddleware;
pub use rquest_otel_span_builder::{
    default_on_request_end, default_on_request_failure, default_on_request_success,
    default_span_name, DefaultSpanBackend, DisableOtelPropagation, OtelName, OtelPathNames,
    ReqwestOtelSpanBackend, SpanBackendWithUrl, ERROR_CAUSE_CHAIN, ERROR_MESSAGE,
    HTTP_REQUEST_METHOD, HTTP_RESPONSE_STATUS_CODE, OTEL_KIND, OTEL_NAME, OTEL_STATUS_CODE,
    SERVER_ADDRESS, SERVER_PORT, URL_FULL, URL_SCHEME, USER_AGENT_ORIGINAL,
};

#[cfg(feature = "deprecated_attributes")]
pub use rquest_otel_span_builder::{
    HTTP_HOST, HTTP_METHOD, HTTP_SCHEME, HTTP_STATUS_CODE, HTTP_URL, HTTP_USER_AGENT, NET_HOST_PORT,
};

#[doc(hidden)]
pub mod rquest_otel_span_macro;
