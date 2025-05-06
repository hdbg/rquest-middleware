use bytes::Bytes;
use http::Extensions;
use rquest::Url;

#[derive(Debug, Clone)]
pub struct Response {
    response_body: Bytes,
    parts: http::response::Parts,
    url: rquest::Url,
}

impl Response {
    pub(crate) async fn from_original(inner: rquest::Response) -> crate::Result<Self> {
        let response = http::response::Response::new(());
        let (mut parts, ()) = response.into_parts();

        parts.status = inner.status();
        parts.headers = inner.headers().clone();
        parts.version = inner.version();
        parts.extensions = Extensions::default();

        let url = inner.url().clone();

        let preloaded_bytes = inner
            .bytes()
            .await
            .map_err(|e| crate::error::Error::Rquest(e.into()))?;

        Ok(Self {
            response_body: preloaded_bytes,
            parts,
            url,
        })
    }

    /// Get the `StatusCode` of this `Response`.
    #[inline]
    pub fn status(&self) -> rquest::StatusCode {
        self.parts.status
    }

    /// Get the HTTP `Version` of this `Response`.
    #[inline]
    pub fn version(&self) -> rquest::Version {
        self.parts.version
    }

    /// Get the `Headers` of this `Response`.
    #[inline]
    pub fn headers(&self) -> &rquest::header::HeaderMap {
        &self.parts.headers
    }

    /// Get a mutable reference to the `Headers` of this `Response`.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut rquest::header::HeaderMap {
        &mut self.parts.headers
    }

    /// Get the content length of the response, if it is known.
    ///
    /// This value does not directly represents the value of the `Content-Length`
    /// header, but rather the size of the response's body. To read the header's
    /// value, please use the [`Response::headers`] method instead.
    ///
    /// Reasons it may not be known:
    ///
    /// - The response does not include a body (e.g. it responds to a `HEAD`
    ///   request).
    /// - The response is gzipped and automatically decoded (thus changing the
    ///   actual decoded length).
    pub fn content_length(&self) -> Option<u64> {
        self.headers()
            .get(rquest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
    }

    /// Get the final `Url` of this `Response`.
    #[inline]
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Returns a reference to the associated extensions.
    pub fn extensions(&self) -> &http::Extensions {
        &self.parts.extensions
    }

    /// Returns a mutable reference to the associated extensions.
    pub fn extensions_mut(&mut self) -> &mut http::Extensions {
        &mut self.parts.extensions
    }

    // body methods

    /// Get the full response text.
    ///
    /// This method decodes the response body with BOM sniffing
    /// and with malformed sequences replaced with the [`char::REPLACEMENT_CHARACTER`].
    /// Encoding is determined from the `charset` parameter of `Content-Type` header,
    /// and defaults to `utf-8` if not presented.
    ///
    /// Note that the BOM is stripped from the returned String.
    ///
    /// # Note
    ///
    /// If the `charset` feature is disabled the method will only attempt to decode the
    /// response as UTF-8, regardless of the given `Content-Type`
    ///
    /// # Example
    ///
    /// ```
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let content = rquest::Client::new()
    ///     .get("http://httpbin.org/range/26")
    ///     .send()
    ///     .await?
    ///     .text()
    ///     .await?;
    ///
    /// println!("text: {content:?}");
    /// # Ok(())
    /// # }
    /// ```
    pub fn text(self) -> crate::Result<String> {
        String::from_utf8(self.response_body.to_vec())
            .map_err(|e| crate::error::Error::Middleware(anyhow::Error::from(e)))
    }

    /// Get the full response text given a specific encoding.
    ///
    /// This method decodes the response body with BOM sniffing
    /// and with malformed sequences replaced with the
    /// [`char::REPLACEMENT_CHARACTER`].
    /// You can provide a default encoding for decoding the raw message, while the
    /// `charset` parameter of `Content-Type` header is still prioritized. For more information
    /// about the possible encoding name, please go to [`encoding_rs`] docs.
    ///
    /// Note that the BOM is stripped from the returned String.
    ///
    /// [`encoding_rs`]: https://docs.rs/encoding_rs/0.8/encoding_rs/#relationship-with-windows-code-pages
    ///
    /// # Optional
    ///
    /// This requires the optional `encoding_rs` feature enabled.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let content = rquest::Client::new()
    ///     .get("http://httpbin.org/range/26")
    ///     .send()
    ///     .await?
    ///     .text_with_charset("utf-8")
    ///     .await?;
    ///
    /// println!("text: {content:?}");
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "charset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "charset")))]
    pub async fn text_with_charset(self, default_encoding: &str) -> crate::Result<String> {
        let content_type = self
            .headers()
            .get(crate::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok());
        let encoding_name = content_type
            .as_ref()
            .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
            .unwrap_or(default_encoding);
        let encoding = Encoding::for_label(encoding_name.as_bytes()).unwrap_or(UTF_8);

        let full = self.bytes().await?;

        let (text, _, _) = encoding.decode(&full);
        Ok(text.into_owned())
    }

    /// Try to deserialize the response body as JSON.
    ///
    /// # Optional
    ///
    /// This requires the optional `json` feature enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate rquest;
    /// # extern crate serde;
    /// #
    /// # use rquest::Error;
    /// # use serde::Deserialize;
    /// #
    /// // This `derive` requires the `serde` dependency.
    /// #[derive(Deserialize)]
    /// struct Ip {
    ///     origin: String,
    /// }
    ///
    /// # async fn run() -> Result<(), Error> {
    /// let ip = rquest::Client::new()
    ///     .get("http://httpbin.org/ip")
    ///     .send()
    ///     .await?
    ///     .json::<Ip>()
    ///     .await?;
    ///
    /// println!("ip: {}", ip.origin);
    /// # Ok(())
    /// # }
    /// #
    /// # fn main() { }
    /// ```
    ///
    /// # Errors
    ///
    /// This method fails whenever the response body is not in JSON format
    /// or it cannot be properly deserialized to target type `T`. For more
    /// details please see [`serde_json::from_reader`].
    ///
    /// [`serde_json::from_reader`]: https://docs.serde.rs/serde_json/fn.from_reader.html
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub fn json<T: serde::de::DeserializeOwned>(self) -> crate::Result<T> {
        serde_json::from_slice(self.preloaded_bytes.as_slice())
            .map_err(|e| crate::error::Error::Rquest(e.into()))
    }

    /// Get the full response body as `Bytes`.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let bytes = rquest::Client::new()
    ///     .get("http://httpbin.org/ip")
    ///     .send()
    ///     .await?
    ///     .bytes()
    ///     .await?;
    ///
    /// println!("bytes: {bytes:?}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bytes(self) -> crate::Result<Bytes> {
        Ok(self.response_body)
    }

    // util methods

    /// Turn a response into an error if the server returned an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use rquest::Response;
    /// fn on_response(res: Response) {
    ///     match res.error_for_status() {
    ///         Ok(_res) => (),
    ///         Err(err) => {
    ///             // asserting a 400 as an example
    ///             // it could be any status between 400...599
    ///             assert_eq!(
    ///                 err.status(),
    ///                 Some(rquest::StatusCode::BAD_REQUEST)
    ///             );
    ///         }
    ///     }
    /// }
    /// # fn main() {}
    /// ```
    pub fn error_for_status(self) -> crate::Result<Self> {
        if self.status().is_client_error() || self.status().is_server_error() {
            Err(crate::Error::Middleware(anyhow::Error::msg(format!(
                "Response error: {}",
                self.status()
            ))))
        } else {
            Ok(self)
        }
    }
}
