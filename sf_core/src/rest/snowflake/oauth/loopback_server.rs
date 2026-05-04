//! Loopback HTTP server that receives the IdP's authorization-code redirect.
//!
//! Must bind explicitly to `127.0.0.1` (and only `::1` when the user
//! supplies a literal IPv6 redirect URI). Do **not** replicate Node's
//! `0.0.0.0` bind — see `analysis_feature_oauth.md` §3.5 and §14 #11.

// TODO(SNOW-OAUTH): implement in step 2.2
