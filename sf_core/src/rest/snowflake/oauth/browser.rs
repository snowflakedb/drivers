//! OS browser launcher for the AC flow's `/authorize` redirect.
//!
//! Falls back to printing the URL when the platform helper is unavailable
//! (Python's manual-paste pattern, analysis_feature_oauth.md §3.6).
//!
//! We never log the full authorize URL, only its authority and path —
//! the query string carries `state` and `dpop_jkt`, which are not strictly
//! secret but should not be persisted in driver logs (see analysis §11).

use std::process::Command;

use snafu::ResultExt;
use url::Url;

use super::error::{BrowserLaunchSnafu, OAuthError};

/// Spawn the OS browser to navigate to `url`.
///
/// Linux uses `xdg-open`. macOS uses `/usr/bin/open`. Windows uses
/// `cmd /C start "" <url>`. Failures are wrapped into
/// [`OAuthError::BrowserLaunch`] so callers can fall back to
/// [`print_paste_instructions`].
pub(crate) fn open(url: &Url) -> Result<(), OAuthError> {
    tracing::debug!(
        authority = %url.authority(),
        path = %url.path(),
        "Opening system browser for OAuth authorization"
    );

    #[cfg(target_os = "macos")]
    {
        Command::new("/usr/bin/open")
            .arg(url.as_str())
            .spawn()
            .context(BrowserLaunchSnafu)?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url.as_str()])
            .spawn()
            .context(BrowserLaunchSnafu)?;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url.as_str())
            .spawn()
            .context(BrowserLaunchSnafu)?;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", unix)))]
    {
        let _ = url;
        Err(OAuthError::BrowserLaunch {
            source: std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "no browser launcher available on this platform",
            ),
            location: snafu::Location::new(file!(), line!(), column!()),
        })
    }
}

/// Headless / no-browser fallback: print the URL so the user can copy
/// and paste it into a browser. Mirrors Python's
/// `_ask_authorization_callback_from_user` flow (analysis §3.6).
pub(crate) fn print_paste_instructions(url: &Url) {
    eprintln!("Open this URL in your browser to continue: {url}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_paste_instructions_does_not_panic() {
        let url = Url::parse("https://idp.example.com/oauth/authorize?state=AAA").unwrap();
        print_paste_instructions(&url);
    }

    #[test]
    fn open_smoke_test_or_skipped_in_ci() {
        // Skip in CI to avoid actually launching a browser. Otherwise this
        // is a smoke test that the `Command::spawn` call itself does not
        // immediately panic for a well-formed URL.
        if std::env::var_os("CI").is_some() {
            return;
        }
        let url = Url::parse("about:blank").unwrap();
        let _ = open(&url);
    }
}
