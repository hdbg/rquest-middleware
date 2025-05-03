# rquest-middleware

A crate implementing a wrapper around [rquest](https://crates.io/crates/rquest)
to allow for client middleware chains.

[![Crates.io](https://img.shields.io/crates/v/rquest-middleware.svg)](https://crates.io/crates/rquest-middleware)
[![Docs.rs](https://docs.rs/rquest-middleware/badge.svg)](https://docs.rs/rquest-middleware)
[![CI](https://github.com/TrueLayer/rquest-middleware/workflows/CI/badge.svg)](https://github.com/TrueLayer/rquest-middleware/actions)
[![Coverage Status](https://coveralls.io/repos/github/TrueLayer/rquest-middleware/badge.svg?branch=main&t=YKhONc)](https://coveralls.io/github/TrueLayer/rquest-middleware?branch=main)

This crate provides functionality for building and running middleware but no middleware
implementations. This repository also contains a couple of useful concrete middleware crates:

* [`rquest-retry`](https://crates.io/crates/rquest-retry): retry failed requests.
* [`rquest-tracing`](https://crates.io/crates/rquest-tracing):
  [`tracing`](https://crates.io/crates/tracing) integration, optional opentelemetry support.

Note about browser support: automated tests targeting wasm are disabled. The crate may work with
wasm but wasm support is unmaintained. PRs improving wasm are still welcome but you'd need to
reintroduce the tests and get them passing before we'd merge it (see
https://github.com/TrueLayer/rquest-middleware/pull/105).

## Overview

The `rquest-middleware` client exposes the same interface as a plain `rquest` client, but
`ClientBuilder` exposes functionality to attach middleware:

```toml
# Cargo.toml
# ...
[dependencies]
rquest = { version = "0.12", features = ["rustls-tls"] }
rquest-middleware = "0.4"
rquest-retry = "0.7"
rquest-tracing = "0.5"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

```rust
use rquest_middleware::{ClientBuilder, ClientWithMiddleware};
use rquest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use rquest_tracing::TracingMiddleware;

#[tokio::main]
async fn main() {
    // Retry up to 3 times with increasing intervals between attempts.
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let client = ClientBuilder::new(rquest::Client::new())
        // Trace HTTP requests. See the tracing crate to make use of these traces.
        .with(TracingMiddleware::default())
        // Retry failed requests.
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();
    run(client).await;
}

async fn run(client: ClientWithMiddleware) {
    client
        .get("https://truelayer.com")
        .header("foo", "bar")
        .send()
        .await
        .unwrap();
}
```

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
</sub>

## Third-party middleware

The following third-party middleware use `rquest-middleware`:

- [`rquest-conditional-middleware`](https://github.com/oxidecomputer/rquest-conditional-middleware) - Per-request basis middleware
- [`http-cache`](https://github.com/06chaynes/http-cache) - HTTP caching rules
- [`rquest-cache`](https://gitlab.com/famedly/company/backend/libraries/rquest-cache) - HTTP caching
- [`aliri_rquest`](https://github.com/neoeinstein/aliri/tree/main/aliri_rquest) - Background token management and renewal
- [`http-signature-normalization-rquest`](https://crates.io/crates/http-signature-normalization-rquest) (not free software) - HTTP Signatures
- [`rquest-chain`](https://github.com/tommilligan/rquest-chain) - Apply custom criteria to any rquest response, deciding when and how to retry.
