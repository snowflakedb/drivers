//! OAuth 2.x flow engine for Snowflake login.
//!
//! Houses the Authorization Code (with PKCE), Client Credentials, and
//! pre-acquired access-token paths plus the supporting primitives (PKCE,
//! state, loopback HTTP server, browser launcher, DPoP, token cache I/O,
//! and the OAuth-specific error type). Cross-driver behavior, parameter
//! names, redaction expectations, and gotchas are catalogued in
//! `analysis_feature_oauth.md` (especially §2–§9 and §14).
//!
//! This file currently exposes only the empty submodule tree introduced in
//! step 2.1 of the OAuth feature. Production logic lands in step 2.2;
//! `LoginMethod` parsing and authenticator-string wiring land in step 2.3.
//!
// TODO(SNOW-OAUTH): wire `pub(crate) use` re-exports for the flow entry
// points (authorization_code::run, client_credentials::run, token::cache I/O)
// in step 2.2 once the production code lands.

mod authorization_code;
mod browser;
mod client_credentials;
mod dpop;
mod error;
mod loopback_server;
mod pkce;
mod state;
mod token;
