///! External browser authentication
///!
///! Implements general external browser authentication flow
use super::oauth::{OAuthError, perform_oauth_browser_auth};
use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
pub enum ExternalBrowserError {
    #[snafu(display("OAuth error during external browser authentication"))]
    OAuth {
        source: OAuthError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("External browser authentication cancelled by user"))]
    Cancelled {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Authentication timeout"))]
    Timeout {
        #[snafu(implicit)]
        location: Location,
    },
}

/// Performs external browser authentication
///
/// Opens browser for user to authenticate, returns token
pub async fn perform_external_browser_auth(
    account: &str,
    authenticator_url: &str,
) -> Result<String, ExternalBrowserError> {
    tracing::info!("Starting external browser authentication for account: {account}");

    // Use OAuth flow for external browser
    let token = perform_oauth_browser_auth(account, authenticator_url)
        .await
        .map_err(|e| ExternalBrowserError::OAuth {
            source: e,
            location: snafu::location!(),
        })?;

    tracing::info!("External browser authentication successful");
    Ok(token)
}
