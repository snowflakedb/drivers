//! OAuth token cache I/O and refresh-token rotation.
//!
//! Cache key derivation follows JDBC/Python: `{HOST_FROM_TOKEN_REQUEST_URL}:
//! {USER}:{TYPE}` uppercased, SHA-256 hashed on Linux file backends
//! (analysis_feature_oauth.md §7). Eviction on Snowflake error codes
//! `390303` / `390318` is required across all drivers (§8).

// TODO(SNOW-OAUTH): implement in step 2.2
