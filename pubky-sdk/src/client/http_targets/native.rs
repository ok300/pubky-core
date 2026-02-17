use crate::errors::RequestError;
use crate::{PubkyHttpClient, PublicKey, Result, cross_log};
use reqwest::{IntoUrl, Method, RequestBuilder};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostKind {
    ResolvedPubky,
    Icann,
    Pubky,
}

fn classify_host(host: &str) -> HostKind {
    if let Some(pk_host) = host.strip_prefix("_pubky.") {
        if PublicKey::is_pubky_prefixed(pk_host) {
            return HostKind::Icann;
        }
        if PublicKey::try_from_z32(pk_host).is_ok() {
            return HostKind::ResolvedPubky;
        }
    } else if PublicKey::is_pubky_prefixed(host) || PublicKey::try_from_z32(host).is_err() {
        return HostKind::Icann;
    }
    HostKind::Pubky
}

impl PubkyHttpClient {
    /// Constructs a [`reqwest::RequestBuilder`] for the given HTTP `method` and `url`,
    /// routing through the client’s unified request path.
    ///
    /// This method ensures that special Pubky and pkarr hosts are resolved according to
    /// platform-specific rules (native or WASM), including:
    /// - Detecting `_pubky.<public-key>` hosts and applying the correct TLS handling.
    /// - Routing standard ICANN domains through the `icann_http` client on native builds.
    ///
    /// On native targets, this is effectively a thin wrapper around [`PubkyHttpClient::request`],
    /// while on WASM it also performs host transformation and may add the `pubky-host` header.
    ///
    /// Returns a [`Result`] containing the prepared `RequestBuilder`, or a URL/transport
    /// parsing error if the supplied `url` is invalid.
    ///
    /// [`PubkyHttpClient::request`]: crate::PubkyHttpClient::request
    #[allow(
        clippy::unused_async,
        reason = "native implementation stays async to share the same signature as the WASM backend"
    )]
    pub(crate) async fn cross_request(
        &self,
        method: Method,
        mut url: Url,
    ) -> Result<RequestBuilder> {
        let _ = self.prepare_request(&mut url).await?;
        Ok(self.request(method, &url))
    }

    /// Prepare a request for callers that need the WASM-style preflight.
    ///
    /// Native builds do not rewrite URLs; we only detect pubky hosts and return the
    /// `pubky-host` value when applicable.
    ///
    /// # Errors
    /// - Returns [`crate::errors::RequestError::Validation`] if the host uses a `pubky` prefix.
    #[allow(
        clippy::unused_async,
        reason = "keep async signature aligned with WASM build"
    )]
    pub async fn prepare_request(&self, url: &mut Url) -> Result<Option<String>> {
        let host = url.host_str().unwrap_or("");

        if let Some(stripped) = host.strip_prefix("_pubky.") {
            if PublicKey::is_pubky_prefixed(stripped) {
                return Err(RequestError::Validation {
                    message: "pubky prefix is not allowed in transport hosts; use raw z32"
                        .to_string(),
                }
                .into());
            }
            if PublicKey::try_from_z32(stripped).is_ok() {
                return Ok(Some(stripped.to_string()));
            }
        } else {
            if PublicKey::is_pubky_prefixed(host) {
                return Err(RequestError::Validation {
                    message: "pubky prefix is not allowed in transport hosts; use raw z32"
                        .to_string(),
                }
                .into());
            }
            if PublicKey::try_from_z32(host).is_ok() {
                return Ok(Some(host.to_string()));
            }
        }

        Ok(None)
    }

    /// Start building a `Request` with the `Method` and `Url` (native-only)
    ///
    /// Returns a `RequestBuilder`, which will allow setting headers and
    /// the request body before sending.
    ///
    /// Differs from [`reqwest::Client::request`], in that it can make requests to:
    /// 1. HTTPS URLs with a [`crate::PublicKey`] as top-level domain, by resolving
    ///    corresponding endpoints, and verifying TLS certificates accordingly.
    ///    (example: `https://o4dksfbqk85ogzdb5osziw6befigbuxmuxkuxq8434q89uj56uyy`)
    /// 2. `_pubky.<public-key>` URLs like `https://_pubky.o4dksfbqk85ogzdb5osziw6befigbuxmuxkuxq8434q89uj56uyy`
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn request<U: IntoUrl>(&self, method: Method, url: &U) -> RequestBuilder {
        let url_str = url.as_str();

        let host = Url::parse(url_str)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned));

        if let Some(ref host) = host {
            match classify_host(host) {
                HostKind::ResolvedPubky => {
                    cross_log!(
                        debug,
                        "Routing request for resolved _pubky host {} via Pubky TLS",
                        host
                    );
                    return self.http.request(method, url_str);
                }
                HostKind::Icann => {
                    // TODO: remove icann_http when we can control reqwest connection
                    // and or create a tls config per connection.
                    cross_log!(
                        debug,
                        "Routing request for ICANN host {} via standard TLS",
                        host
                    );
                    return self.icann_http.request(method, url_str);
                }
                HostKind::Pubky => {
                    cross_log!(
                        debug,
                        "Routing request for pubky host {} via PubkyTLS",
                        host
                    );
                }
            }
        }

        self.http.request(method, url_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Keypair;

    #[test]
    fn classify_host_resolved_pubky() {
        // Valid z32 key after _pubky. prefix
        let kp = Keypair::random();
        let z32 = kp.public_key().z32();
        let host = format!("_pubky.{}", z32);
        assert_eq!(classify_host(&host), HostKind::ResolvedPubky);
    }

    #[test]
    fn classify_host_icann_pubky_prefixed_after_underscore() {
        // pubky-prefixed key after _pubky. prefix should be ICANN
        let kp = Keypair::random();
        let pubky_prefixed = kp.public_key().to_string(); // Returns "pubky<z32>"
        let host = format!("_pubky.{}", pubky_prefixed);
        assert_eq!(classify_host(&host), HostKind::Icann);
    }

    #[test]
    fn classify_host_pubky_fallback_invalid_after_prefix() {
        // When _pubky. prefix is present but the key is invalid (neither pubky-prefixed nor valid z32),
        // the function falls through to the default Pubky classification.
        // This is the intended fallback behavior of classify_host.
        assert_eq!(classify_host("_pubky.invalid"), HostKind::Pubky);
        assert_eq!(classify_host("_pubky.example.com"), HostKind::Pubky);
    }

    #[test]
    fn classify_host_pubky_raw_z32() {
        // Raw z32 key without any prefix
        let kp = Keypair::random();
        let z32 = kp.public_key().z32();
        assert_eq!(classify_host(&z32), HostKind::Pubky);
    }

    #[test]
    fn classify_host_icann_pubky_prefixed() {
        // pubky-prefixed key without _pubky. prefix should be ICANN
        let kp = Keypair::random();
        let pubky_prefixed = kp.public_key().to_string(); // Returns "pubky<z32>"
        assert_eq!(classify_host(&pubky_prefixed), HostKind::Icann);
    }

    #[test]
    fn classify_host_icann_regular_domain() {
        // Regular domain names should be ICANN
        assert_eq!(classify_host("example.com"), HostKind::Icann);
        assert_eq!(classify_host("www.example.com"), HostKind::Icann);
        assert_eq!(classify_host("subdomain.example.org"), HostKind::Icann);
    }

    #[test]
    fn classify_host_icann_invalid_z32() {
        // Invalid z32 keys (wrong length, invalid chars) should be ICANN
        assert_eq!(classify_host("invalid"), HostKind::Icann);
        assert_eq!(classify_host("tooshort"), HostKind::Icann);
        assert_eq!(
            classify_host("thisiswaytoolongtobeavalidz32keyitshouldbefiftytwocharacters"),
            HostKind::Icann
        );
    }

    #[test]
    fn classify_host_empty_string() {
        // Empty string: try_from_z32("") fails, so the else branch returns ICANN
        assert_eq!(classify_host(""), HostKind::Icann);
    }

    #[test]
    fn classify_host_prefix_without_key() {
        // When only _pubky. prefix is present without a key, the ResolvedPubky checks fail
        // and the function falls through to the default Pubky classification.
        // This is consistent with the fallback behavior for malformed _pubky. hosts.
        assert_eq!(classify_host("_pubky."), HostKind::Pubky);
    }

    #[test]
    fn classify_host_localhost_and_ips() {
        // localhost and IP addresses: try_from_z32 will fail, so ICANN
        assert_eq!(classify_host("localhost"), HostKind::Icann);
        assert_eq!(classify_host("127.0.0.1"), HostKind::Icann);
    }
}
