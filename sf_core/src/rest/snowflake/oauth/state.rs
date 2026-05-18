//! `state` parameter generation and CSRF validation for the AC flow.
//!
//! Mismatch must surface the canonical "It might indicate an XSS attack"
//! message — multiple drivers ship that exact wording and SREs grep for
//! it (analysis_feature_oauth.md §3.4 and §14 #7).

// TODO(SNOW-OAUTH): implement in step 2.2
