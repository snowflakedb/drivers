//! Errors raised by the OAuth flow engine.
//!
//! Skeleton enum introduced in step 2.1; concrete variants for state
//! mismatch, IdP failures, browser timeouts, etc. land in step 2.2 (see
//! `analysis_feature_oauth.md` §13 for the cross-driver error taxonomy).

// TODO(SNOW-OAUTH): implement in step 2.2

use snafu::{Location, Snafu};

/// Errors raised by the OAuth flow engine.
#[allow(dead_code)]
#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub(crate) enum OAuthError {
    #[snafu(display("OAuth flow not yet implemented"))]
    NotImplemented {
        #[snafu(implicit)]
        location: Location,
    },
}
