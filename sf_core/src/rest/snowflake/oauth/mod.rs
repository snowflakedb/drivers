//! OAuth 2.x flow engine for Snowflake login.
//!
//! Houses the Authorization Code (with PKCE), Client Credentials, and
//! pre-acquired access-token paths plus the supporting primitives (PKCE,
//! loopback HTTP server, DPoP, token cache I/O, and the OAuth-specific
//! error type). CSRF state, browser launch, and the token-endpoint HTTP
//! exchange are handled by the `oauth2`, `webbrowser`, and `axum`
//! crates respectively.
//!
//! The re-exports below pin the surface that `auth_request_data`
//! consumes when wiring `LoginMethod::OAuth*` (login-request payload
//! mapping) and that `snowflake_login_with_client` uses for the
//! refresh-on-failure retry path (390303/390318 eviction + replay).

mod authorization_code;
mod client_credentials;
pub(crate) mod dpop;
mod error;
mod http_client;
mod loopback_server;
// Standalone PKCE helper kept as scaffolding; the active flow uses
// `oauth2::PkceCodeChallenge` directly (PKCE always-on).
#[allow(dead_code)]
mod pkce;
mod token;

/// Browser-launcher closure type used by [`OAuthAuthorizationCodeConfig`]
/// to carry a per-connection factory. Re-exported so
/// `crate::config::rest_parameters` can name the field type without
/// reaching into `oauth::authorization_code` directly.
pub(crate) use authorization_code::BrowserLaunchFn;
pub(crate) use authorization_code::derive_idp_url;
pub(crate) use authorization_code::run_oauth_authorization_code;
pub(crate) use client_credentials::acquire_client_credentials;
pub use error::OAuthError;
pub(crate) use token::{remove_oauth_access_token, remove_oauth_dpop_bundled};

/// Re-exported as `pub` under `cfg(any(test, feature = "test-utils"))` so
/// integration / e2e tests can still derive a cache-key host from a token
/// URL without re-implementing the Python-style
/// `urlparse(token_request_url).hostname` fallback chain. Production flows
/// build [`CacheKey`] directly via the token helpers and no longer need this.
#[cfg(any(test, feature = "test-utils"))]
pub use token::host_from_token_url;
