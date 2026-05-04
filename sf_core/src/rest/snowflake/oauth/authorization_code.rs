//! OAuth 2.0 Authorization Code flow with PKCE (S256).
//!
//! Owns the end-to-end orchestration: PKCE verifier/challenge, state
//! parameter, browser launch, loopback HTTP redirect handling, token
//! exchange, and refresh-token rotation. See `analysis_feature_oauth.md`
//! §3 for the per-driver state machine and gotchas (notably §3.5 on
//! 127.0.0.1 binding and §14 #11 on rejecting Node's `0.0.0.0`).

// TODO(SNOW-OAUTH): implement in step 2.2
