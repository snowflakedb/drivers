//! DPoP (RFC 9449) proof-of-possession helpers.
//!
//! ES256 P-256 keypair, proof JWT with `jti`/`htm`/`htu`/`iat` (and
//! optional `nonce` on `use_dpop_nonce` retry), `dpop_jkt` thumbprint on
//! the `/authorize` request, and a bundled access-token cache row. Only
//! JDBC has parity today (analysis_feature_oauth.md §5).

// TODO(SNOW-OAUTH): implement in step 2.2
