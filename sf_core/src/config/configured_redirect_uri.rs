use snafu::{Location, ResultExt, Snafu};
use url::Url;

/// The user-configured `oauth_redirect_uri`, validated at construction via
/// [`url::Url`] but retaining the caller's **exact original spelling** for
/// transmission to the IdP.
///
/// OAuth IdPs that perform RFC 6749 §3.1.2.3 simple-string matching reject
/// a canonicalized form (e.g. `http://localhost:12346` → `http://localhost:12346/`).
/// This type validates the input while keeping the verbatim string intact.
///
/// Scope: redirect URIs only. Endpoint URLs are called, not string-matched,
/// so plain [`url::Url`] normalization is fine there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfiguredRedirectUri {
    parsed: Url,
    raw: String,
}

impl ConfiguredRedirectUri {
    /// Validate `raw` as an absolute URL while preserving its exact spelling.
    /// Fails with [`ConfiguredRedirectUriError`] on malformed input.
    pub fn parse(raw: impl Into<String>) -> Result<Self, ConfiguredRedirectUriError> {
        let raw = raw.into();
        let parsed = Url::parse(&raw).context(UrlParseSnafu { input: raw.clone() })?;
        Ok(Self { parsed, raw })
    }

    /// Construct from an already-validated [`Url`] and its advertised string
    /// without re-parsing. `raw` is the string that will be forwarded to the IdP.
    pub(crate) fn from_parts(parsed: Url, raw: String) -> Self {
        Self { parsed, raw }
    }

    /// Normalized structural view used for host/port/path inspection. Never
    /// serialized to the IdP.
    pub fn parsed(&self) -> &Url {
        &self.parsed
    }

    /// The string forwarded to the IdP (handed to `oauth2::RedirectUrl::new`).
    pub fn as_configured(&self) -> &str {
        &self.raw
    }
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum ConfiguredRedirectUriError {
    /// The configured `oauth_redirect_uri` is not a parseable absolute URL.
    #[snafu(display("invalid OAuth redirect URI {input:?}"))]
    UrlParse {
        input: String,
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_url_round_trips_as_configured_verbatim() {
        let uri = ConfiguredRedirectUri::parse("http://localhost:12346").unwrap();
        assert_eq!(uri.as_configured(), "http://localhost:12346");
    }

    #[test]
    fn parse_url_with_trailing_slash_round_trips_verbatim() {
        let uri = ConfiguredRedirectUri::parse("http://localhost:12346/").unwrap();
        assert_eq!(uri.as_configured(), "http://localhost:12346/");
    }

    #[test]
    fn parse_url_with_path_round_trips_verbatim() {
        let uri = ConfiguredRedirectUri::parse("http://localhost:12346/cb").unwrap();
        assert_eq!(uri.as_configured(), "http://localhost:12346/cb");
    }

    #[test]
    fn parsed_exposes_host_port_path() {
        let uri = ConfiguredRedirectUri::parse("http://localhost:12346/cb").unwrap();
        let p = uri.parsed();
        assert_eq!(p.host_str(), Some("localhost"));
        assert_eq!(p.port(), Some(12346));
        assert_eq!(p.path(), "/cb");
    }

    #[test]
    fn parse_returns_error_on_malformed_input() {
        let err = ConfiguredRedirectUri::parse("not a url").unwrap_err();
        match err {
            ConfiguredRedirectUriError::UrlParse { input, .. } => {
                assert_eq!(input, "not a url");
            }
        }
    }

    #[test]
    fn parse_returns_error_on_relative_url() {
        let err = ConfiguredRedirectUri::parse("/relative/path").unwrap_err();
        match err {
            ConfiguredRedirectUriError::UrlParse { input, .. } => {
                assert_eq!(input, "/relative/path");
            }
        }
    }

    #[test]
    fn as_configured_does_not_normalize_trailing_slash() {
        // Core regression: url::Url adds a trailing slash when the path
        // is empty; as_configured() must return the verbatim input.
        let without_slash = ConfiguredRedirectUri::parse("http://localhost:12346").unwrap();
        let with_slash = ConfiguredRedirectUri::parse("http://localhost:12346/").unwrap();
        assert_ne!(without_slash.as_configured(), with_slash.as_configured());
        assert_eq!(without_slash.as_configured(), "http://localhost:12346");
        assert_eq!(with_slash.as_configured(), "http://localhost:12346/");
    }
}
