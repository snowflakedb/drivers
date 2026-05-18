//! OAuth 2.0 Client Credentials flow (external IdP only).
//!
//! Snowflake-as-IdP does not currently issue tokens for
//! `grant_type=client_credentials` (analysis_feature_oauth.md §4), so this
//! module always requires an explicit `token_url`. Tokens obtained here are
//! intentionally not persisted to the OS token cache (analysis §14 #12).

// TODO(SNOW-OAUTH): implement in step 2.2
