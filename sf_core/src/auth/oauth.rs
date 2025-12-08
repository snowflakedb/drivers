///! OAuth authentication implementation
///!
///! Implements OAuth external browser flow for Snowflake authentication
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::io::{Read, Write};
use std::net::TcpListener;
use url::Url;

const OAUTH_REDIRECT_PORT: u16 = 8080;
const OAUTH_LOCALHOST_URL: &str = "http://localhost:8080";

#[derive(Debug, Snafu)]
pub enum OAuthError {
    #[snafu(display("Failed to start OAuth callback server"))]
    ServerStart {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to open browser for OAuth"))]
    BrowserOpen {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("OAuth callback timeout"))]
    Timeout {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse OAuth callback URL"))]
    UrlParse {
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Missing OAuth authorization code"))]
    MissingCode {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to exchange OAuth code for token"))]
    TokenExchange {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Token exchange failed: status={status}, body={body}"))]
    TokenExchangeFailed {
        status: reqwest::StatusCode,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse token response"))]
    TokenParse {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Missing access_token in OAuth response"))]
    MissingAccessToken {
        #[snafu(implicit)]
        location: Location,
    },
}

/// Performs OAuth external browser authentication
///
/// 1. Starts local callback server on port 8080
/// 2. Opens browser to Snowflake OAuth URL
/// 3. Waits for callback with authorization code
/// 4. Returns the authorization code
pub async fn perform_oauth_browser_auth(
    account: &str,
    oauth_url: &str,
) -> Result<String, OAuthError> {
    // Start local callback server
    let listener =
        TcpListener::bind(format!("127.0.0.1:{OAUTH_REDIRECT_PORT}")).context(ServerStartSnafu)?;

    tracing::info!("OAuth callback server started on port {OAUTH_REDIRECT_PORT}");

    // Construct full OAuth URL with redirect
    let auth_url = format!(
        "{}?redirect_uri={}&response_type=code&client_id={}",
        oauth_url, OAUTH_LOCALHOST_URL, account
    );

    // Open browser to OAuth URL
    tracing::info!("Opening browser for OAuth authentication: {auth_url}");
    open_browser(&auth_url)?;

    // Wait for callback
    tracing::info!("Waiting for OAuth callback...");
    let authorization_code = wait_for_oauth_callback(listener)?;

    tracing::info!("OAuth authorization code received");
    Ok(authorization_code)
}

/// Opens the system default browser to the given URL
fn open_browser(url: &str) -> Result<(), OAuthError> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context(BrowserOpenSnafu)?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context(BrowserOpenSnafu)?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .context(BrowserOpenSnafu)?;
    }

    Ok(())
}

/// Waits for OAuth callback and extracts authorization code
fn wait_for_oauth_callback(listener: TcpListener) -> Result<String, OAuthError> {
    // Set timeout for waiting
    listener.set_nonblocking(false).context(ServerStartSnafu)?;

    // Accept one connection
    let (mut stream, _) = listener.accept().context(ServerStartSnafu)?;

    // Read HTTP request
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer).context(ServerStartSnafu)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    tracing::debug!("Received OAuth callback request: {request}");

    // Parse request line to get URL
    let first_line = request.lines().next().context(MissingCodeSnafu)?;
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 {
        return MissingCodeSnafu.fail();
    }

    let path = parts[1];
    let full_url = format!("http://localhost{path}");
    let url = Url::parse(&full_url).context(UrlParseSnafu)?;

    // Extract authorization code from query parameters
    let authorization_code = url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .context(MissingCodeSnafu)?;

    // Send success response to browser
    let response = "HTTP/1.1 200 OK\r\n\r\n\
        <html><body><h1>Authentication Successful!</h1>\
        <p>You can close this window and return to your application.</p>\
        </body></html>";

    stream
        .write_all(response.as_bytes())
        .context(ServerStartSnafu)?;
    stream.flush().context(ServerStartSnafu)?;

    Ok(authorization_code)
}

/// Exchanges OAuth authorization code for access token
pub async fn exchange_oauth_code_for_token(
    account: &str,
    code: &str,
    token_endpoint: &str,
) -> Result<String, OAuthError> {
    tracing::info!("Exchanging OAuth code for access token: account={account}");

    // Build token exchange request
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", OAUTH_LOCALHOST_URL),
        ("client_id", account),
    ];

    // Send token exchange request
    let response = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .await
        .context(TokenExchangeSnafu)?;

    // Check response status
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::error!("Token exchange failed: status={status}, body={body}");
        return TokenExchangeFailedSnafu { status, body }.fail();
    }

    // Parse response JSON
    let token_response: serde_json::Value = response.json().await.context(TokenParseSnafu)?;

    // Extract access token
    let access_token = token_response
        .get("access_token")
        .and_then(|v| v.as_str())
        .context(MissingAccessTokenSnafu)?
        .to_string();

    tracing::info!("Successfully exchanged OAuth code for access token");
    Ok(access_token)
}
