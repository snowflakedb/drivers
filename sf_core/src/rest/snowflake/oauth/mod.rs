//! OAuth 2.x flow engine for Snowflake login.
//!
//! Houses the Authorization Code (with PKCE), Client Credentials, and
//! pre-acquired access-token paths plus the supporting primitives (PKCE,
//! state, loopback HTTP server, browser launcher, DPoP, token cache I/O,
//! and the OAuth-specific error type). Cross-driver behavior, parameter
//! names, redaction expectations, and gotchas are catalogued in
//! `analysis_feature_oauth.md` (especially §2–§9 and §14).
//!
//! Re-exports below pin the surface that step 2.3 (`auth_request_data`
//! wiring) will consume; everything else stays private to this module.
//!
//! `#![allow(dead_code)]` is intentional: until step 2.3 wires
//! `LoginMethod::OAuth*` into `auth_request_data`, no production caller
//! references these helpers — exclusively the unit tests below do. The
//! allow is removed by the wiring step.
#![allow(dead_code)]

// The submodules below are not yet wired into `auth_request_data` (that is
// step 2.3). Suppress dead-code warnings on the new surface until then so
// that `cargo clippy -D warnings` stays green for downstream consumers.
#[allow(dead_code)]
mod authorization_code;
#[allow(dead_code)]
mod browser;
#[allow(dead_code)]
mod client_credentials;
#[allow(dead_code)]
mod dpop;
#[allow(dead_code)]
mod error;
#[allow(dead_code)]
mod loopback_server;
#[allow(dead_code)]
mod pkce;
#[allow(dead_code)]
mod state;
#[allow(dead_code)]
mod token;

#[allow(unused_imports)]
pub(crate) use authorization_code::{AcquiredOAuthToken, acquire_authorization_code};
#[allow(unused_imports)]
pub(crate) use client_credentials::acquire_client_credentials;
#[allow(unused_imports)]
pub(crate) use error::OAuthError;
#[allow(unused_imports)]
pub(crate) use token::{
    host_from_token_url, remove_oauth_access_token, remove_oauth_dpop_bundled,
    remove_oauth_refresh_token, store_oauth_access_token, store_oauth_dpop_bundled,
    store_oauth_refresh_token, try_get_cached_oauth_access_token,
    try_get_cached_oauth_dpop_bundled, try_get_cached_oauth_refresh_token,
};
