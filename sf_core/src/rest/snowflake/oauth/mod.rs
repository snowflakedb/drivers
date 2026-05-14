//! OAuth 2.x flow engine for Snowflake login.
//!
//! Houses the Authorization Code (with PKCE), Client Credentials, and
//! pre-acquired access-token paths plus the supporting primitives (PKCE,
//! loopback HTTP server, DPoP, token cache I/O, and the OAuth-specific
//! error type). CSRF state, browser launch, and the token-endpoint HTTP
//! exchange are handled by the `oauth2`, `webbrowser`, and `axum`
//! crates respectively. Cross-driver behavior, parameter names,
//! redaction expectations, and gotchas are catalogued in
//! `analysis_feature_oauth.md` (especially §2–§9 and §14).
//!
//! The re-exports below pin the surface that `auth_request_data`
//! consumes when wiring `LoginMethod::OAuth*` (analysis §6 / §10.1)
//! and that `snowflake_login_with_client` uses for the
//! refresh-on-failure retry path (analysis §8 / §14 #9).

mod authorization_code;
mod client_credentials;
pub(crate) mod dpop;
mod error;
mod http_client;
mod loopback_server;
// Standalone PKCE helper kept as scaffolding; the active flow uses
// `oauth2::PkceCodeChallenge` directly (analysis §9: PKCE always-on).
#[allow(dead_code)]
mod pkce;
mod token;

pub(crate) use authorization_code::acquire_authorization_code;
pub(crate) use client_credentials::acquire_client_credentials;
pub(crate) use error::EndpointUrlParseSnafu;
pub use error::OAuthError;
pub(crate) use token::{host_from_token_url, remove_oauth_access_token, remove_oauth_dpop_bundled};
