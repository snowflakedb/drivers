//! Errors raised by the OAuth flow engine.
//!
//! The variant set is derived from the cross-driver error taxonomy in
//! `analysis_feature_oauth.md` §13. Marked `pub(crate)` because the
//! wiring layer (step 2.3) translates these into the driver-facing
//! `RestError` / `AuthError` taxonomy. Crate-internal callers should
//! match on variants to make eviction / refresh decisions (analysis §8:
//! e.g. `RefreshTokenExchange` should drop the cached refresh token and
//! replay the full flow).

use crate::token_cache::TokenCacheError;
use snafu::{Location, Snafu};

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum OAuthError {
    /// `state` mismatch on the loopback redirect — analysis §14 #7.
    /// Equality is enforced via `oauth2::CsrfToken`'s timing-safe
    /// `PartialEq` (the `timing-resistant-secret-traits` feature
    /// derives a SHA-256-based comparator).
    #[snafu(display(
        "OAuth callback state parameter did not match the value issued by this client"
    ))]
    StateMismatch {
        #[snafu(implicit)]
        location: Location,
    },

    /// Loopback redirect arrived without a `code=` query parameter and
    /// without an `error=` parameter (defensive — should not happen in
    /// well-behaved IdPs).
    #[snafu(display("Authorization redirect did not include an authorization code"))]
    MissingAuthorizationCode {
        #[snafu(implicit)]
        location: Location,
    },

    /// IdP returned `error=...&error_description=...` either on the
    /// authorize redirect or in the token-endpoint response.
    #[snafu(display("Identity Provider responded with error: {error}: {description}"))]
    IdpError {
        error: String,
        description: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Loopback `accept()` did not see a request before the configured
    /// browser-response timeout elapsed (cross-driver: JDBC/Python/Go/.NET
    /// 120s default, analysis §3.5).
    #[snafu(display("OAuth browser authorization timed out"))]
    BrowserTimeout {
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not bind a TCP listener on `127.0.0.1:<port>` for the
    /// loopback redirect (e.g. port already in use, EACCES).
    #[snafu(display("Failed to bind loopback listener for OAuth redirect"))]
    PortBind {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    /// Loopback / redirect URI parse error (typically when the user
    /// supplied a malformed `oauth_redirect_uri`).
    #[snafu(display("Failed to parse OAuth redirect URI"))]
    RedirectUriParse {
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },

    /// IdP endpoint URL (`oauth_authorization_url`, `oauth_token_url`)
    /// failed to parse, either as supplied or as constructed from the
    /// Snowflake server URL via the `https://{host}/oauth/...` defaults.
    #[snafu(display("Failed to parse OAuth endpoint URL: {url}"))]
    EndpointUrlParse {
        url: String,
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not deserialize the token endpoint JSON response (used by
    /// the `RequestTokenError::Parse` mapping in the AC/CC flows).
    #[snafu(display("Failed to decode OAuth token response"))]
    TokenResponseDecode {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },

    /// Refresh-token exchange failed. Caller should evict the refresh
    /// token from cache and replay the full flow (analysis §7.4 / §8).
    #[snafu(display("OAuth refresh-token exchange failed"))]
    RefreshTokenExchange {
        #[snafu(implicit)]
        location: Location,
    },

    /// Token endpoint returned 2xx but no `access_token` field. Treated
    /// as an error to avoid passing an empty Bearer token downstream
    /// (analysis §13: ODBC's `idp_auth_missing_access_token` fixture).
    #[snafu(display("OAuth token response did not include an access_token"))]
    MissingAccessToken {
        #[snafu(implicit)]
        location: Location,
    },

    /// DPoP proof JWT could not be constructed because of an underlying
    /// openssl primitive (key generation, coordinate extraction).
    #[snafu(display("Failed to generate DPoP proof JWT"))]
    DPoPProofGeneration {
        source: openssl::error::ErrorStack,
        #[snafu(implicit)]
        location: Location,
    },

    /// DPoP proof JWT signing failed inside the `jwt` crate (e.g.
    /// header/claims serialization or DER → JOSE conversion).
    #[snafu(display("Failed to sign DPoP proof JWT"))]
    DPoPProofSigning {
        source: jwt::Error,
        #[snafu(implicit)]
        location: Location,
    },

    /// DPoP JWK could not be parsed (e.g. unsupported `kty`/`crv` or
    /// a missing required field on a cached bundled JWK).
    #[snafu(display("DPoP JWK could not be parsed: {reason}"))]
    DPoPJwkParse {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Sentinel: token endpoint requested a DPoP nonce. Internal callers
    /// catch this once and retry with the supplied nonce embedded in the
    /// proof JWT. Never bubbled to user code (analysis §5.1).
    #[snafu(display("OAuth token endpoint requested a DPoP nonce; retrying"))]
    DPoPNonceRequired {
        #[snafu(implicit)]
        location: Location,
    },

    /// Token cache I/O failure (read, write or delete). Non-fatal at the
    /// flow level — callers WARN and fall through to a fresh exchange.
    #[snafu(display("OAuth token cache operation failed"))]
    Cache {
        source: TokenCacheError,
        #[snafu(implicit)]
        location: Location,
    },

    /// Underlying HTTP transport error from `reqwest` while talking to
    /// the IdP token endpoint.
    #[snafu(display("OAuth HTTP transport error"))]
    Transport {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
}
