use crate::config::rest_parameters::LoginParameters;
use crate::config::retry::RetryPolicy;
use crate::env_vars;
use crate::http::retry::{HttpContext, HttpError};
use crate::rest::snowflake::auth::{AuthRequest, AuthRequestData};
use crate::rest::snowflake::parse_gs_code_or_unavailable;
use crate::sensitive::SensitiveString;
use reqwest::{Method, StatusCode, header};
use serde::Deserialize;
use snafu::{Location, ResultExt, Snafu, ensure};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

// ─── Constants ───────────────────────────────────────────────────────────────

const SF_AUTHENTICATOR_REQUEST_PATH: &str = "/session/authenticator-request";

const SUCCESS_HTML: &str = "\
<!DOCTYPE html>\
<html><head><title>Authentication Successful</title></head>\
<body><h1>Your identity was confirmed</h1>\
<p>You can close this browser tab.</p>\
</body></html>";

// ─── Public types ────────────────────────────────────────────────────────────

/// Result of the full external browser authentication flow.
#[derive(Debug)]
pub(crate) struct ExternalBrowserAuthResult {
    pub token: SensitiveString,
    pub proof_key: SensitiveString,
    /// Whether the IdP consented to caching the ID token.
    /// `None` when the callback was a plain GET redirect (no consent info).
    /// TODO: Read when ID token caching is implemented (follow-up PR).
    #[allow(dead_code)]
    pub consent_cache_id_token: Option<bool>,
}

/// Opens the SSO / authorize URL. Allows injecting a custom callback.
pub(crate) trait BrowserOpener: Send + Sync {
    fn open(&self, url: &str) -> Result<(), String>;
}

pub(crate) type BrowserOpenFn = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

/// Default implementation that opens the system browser, unless the
/// `SF_TEST_BROWSER_OPENER` env var is set to `noop` (for headless CI).
pub(crate) struct DefaultBrowserOpener;

pub(crate) struct FnBrowserOpener(pub(crate) BrowserOpenFn);

impl BrowserOpener for FnBrowserOpener {
    fn open(&self, url: &str) -> Result<(), String> {
        (self.0)(url)
    }
}

impl BrowserOpener for DefaultBrowserOpener {
    fn open(&self, url: &str) -> Result<(), String> {
        if std::env::var(env_vars::SF_TEST_BROWSER_OPENER).as_deref() == Ok("noop") {
            tracing::info!(
                url,
                "Browser open suppressed by SF_TEST_BROWSER_OPENER=noop"
            );
            return Ok(());
        }
        // Route through the WSL-safe launcher (SNOW-3649282); off WSL this
        // defers to the `webbrowser` crate.
        if let Err(e) = super::browser::open_url(url) {
            eprintln!(
                "Could not open browser. Open the following URL in your browser manually:\n{url}"
            );
            tracing::warn!(
                error = %e,
                "Could not open browser — waiting for callback anyway"
            );
        }
        Ok(())
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum ExternalBrowserError {
    #[snafu(display("Authentication timeout exceeded (budget {budget:?})"))]
    AuthenticationTimeout {
        budget: Duration,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to bind local listener on 127.0.0.1"))]
    ListenerBind {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to open browser for SSO login: {reason}"))]
    BrowserOpen {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{reason}"))]
    BrowserCallback {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("HTTP {status} from Snowflake during {context}: {body}"))]
    HttpStatus {
        context: &'static str,
        status: StatusCode,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Snowflake authenticator-request (logical failure): {message}, code: {code}"))]
    AuthenticatorRequestRejected {
        message: String,
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("HTTP retry budget exhausted during external browser flow"))]
    RetryExhausted {
        source: HttpError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize JSON request body"))]
    JsonSerialize {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse JSON response"))]
    JsonParse {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Authenticator response missing field: {field}"))]
    MissingField {
        field: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read from callback connection"))]
    CallbackIo {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Browser callback body exceeds the maximum allowed size of {max_bytes} bytes"
    ))]
    CallbackBodyTooLarge {
        max_bytes: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Browser callback headers exceed the maximum allowed size of {max_bytes} bytes"
    ))]
    CallbackHeadersTooLarge {
        max_bytes: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

// ─── Main entry point ────────────────────────────────────────────────────────

/// Run the full external browser authentication flow:
///
/// 1. Bind a local TCP listener on an OS-assigned port.
/// 2. Ask Snowflake for the SSO URL and proof key.
/// 3. Open the SSO URL in the user's default browser.
/// 4. Wait for the IdP to redirect back with a token (or time out).
#[tracing::instrument(
    skip(client, login_parameters, browser_opener),
    fields(authentication_timeout_secs)
)]
pub(crate) async fn external_browser_authenticate(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    username: &str,
    authentication_timeout_secs: u64,
    browser_opener: Arc<dyn BrowserOpener>,
    retry_policy: &RetryPolicy,
) -> Result<ExternalBrowserAuthResult, ExternalBrowserError> {
    let budget = Duration::from_secs(authentication_timeout_secs);
    let start = Instant::now();
    tracing::info!("Starting external browser authentication");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context(ListenerBindSnafu)?;
    let local_port = listener.local_addr().context(ListenerBindSnafu)?.port();
    tracing::debug!(port = local_port, "Local callback listener bound");

    let idp_data =
        request_authenticator(client, login_parameters, username, local_port, retry_policy).await?;
    let proof_key = idp_data.proof_key;
    tracing::debug!("Received SSO URL and proof key from Snowflake");

    // Validate the SSO URL before handing it to the system browser: reject
    // non-https URLs and URLs carrying characters unsafe to pass to a
    // launcher. See SNOW-3649282.
    if let Err(reason) = super::browser::validate_browser_url(&idp_data.sso_url) {
        return BrowserOpenSnafu { reason }.fail();
    }

    let remaining = budget.saturating_sub(start.elapsed());
    let sso_url = idp_data.sso_url.clone();
    let open_fut = async move {
        tokio::task::spawn_blocking(move || browser_opener.open(&sso_url))
            .await
            .unwrap_or_else(|e| Err(e.to_string()))
    };
    tokio::pin!(open_fut);
    let accept_fut = tokio::time::timeout(remaining, accept_token_from_callback(&listener));
    tokio::pin!(accept_fut);
    let timeout_err = || ExternalBrowserError::AuthenticationTimeout {
        budget,
        location: Location::new(file!(), line!(), column!()),
    };
    let callback = tokio::select! {
        biased;
        accept = &mut accept_fut => {
            accept.map_err(|_| timeout_err())??
        }
        result = &mut open_fut => {
            match result {
                Err(reason) => return BrowserCallbackSnafu { reason }.fail(),
                Ok(()) => {
                    eprintln!(
                        "Initiating login request with your identity provider. A browser window \
                         should have opened for you to complete the login. If you can't see it, \
                         check existing browser windows, or your OS settings. Alternatively, \
                         open the following URL in your browser:\n{}",
                        idp_data.sso_url
                    );
                    accept_fut.await.map_err(|_| timeout_err())??
                }
            }
        }
    };

    tracing::info!(
        elapsed_ms = start.elapsed().as_millis(),
        consent_cache_id_token = ?callback.consent_cache_id_token,
        "External browser authentication completed successfully"
    );
    Ok(ExternalBrowserAuthResult {
        token: callback.token,
        proof_key: proof_key.into(),
        consent_cache_id_token: callback.consent_cache_id_token,
    })
}

// ─── Snowflake authenticator-request ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AuthenticatorRequestResponse {
    success: bool,
    code: Option<String>,
    message: Option<String>,
    data: Option<AuthenticatorRequestData>,
}

#[derive(Debug, Deserialize)]
struct AuthenticatorRequestData {
    #[serde(rename = "ssoUrl")]
    sso_url: String,
    #[serde(rename = "proofKey")]
    proof_key: String,
}

/// Send `POST /session/authenticator-request` to Snowflake with
/// `AUTHENTICATOR=EXTERNALBROWSER` and the local listener port.
async fn request_authenticator(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    username: &str,
    redirect_port: u16,
    retry_policy: &RetryPolicy,
) -> Result<AuthenticatorRequestData, ExternalBrowserError> {
    let mut data: AuthRequestData = super::base_auth_request_data(login_parameters);
    data.login_name = Some(username.to_string());
    data.authenticator = Some("EXTERNALBROWSER".to_string());
    data.browser_mode_redirect_port = Some(redirect_port.to_string());
    let authn_req = AuthRequest { data };
    let authn_url = format!(
        "{}{}",
        login_parameters.server_url, SF_AUTHENTICATOR_REQUEST_PATH
    );

    let body_string = serde_json::to_string(&authn_req).context(JsonSerializeSnafu)?;
    let http_ctx = HttpContext::new(Method::POST, SF_AUTHENTICATOR_REQUEST_PATH).allow_post_retry();
    let (status, text) = super::request_text_with_retry(
        || {
            client
                .post(&authn_url)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "application/json")
                .header(
                    "User-Agent",
                    super::user_agent(&login_parameters.client_info),
                )
                .body(body_string.clone())
        },
        &http_ctx,
        retry_policy,
    )
    .await
    .context(RetryExhaustedSnafu)?;

    if !status.is_success() {
        tracing::error!(
            status = %status,
            "Snowflake authenticator-request failed for external browser"
        );
        return HttpStatusSnafu {
            context: "Snowflake authenticator-request",
            status,
            body: text,
        }
        .fail();
    }

    let resp: AuthenticatorRequestResponse = serde_json::from_str(&text).context(JsonParseSnafu)?;
    if !resp.success {
        let code = parse_gs_code_or_unavailable(resp.code.as_deref());
        let message = resp.message.unwrap_or_else(|| "Unknown error".to_string());
        tracing::error!(
            code,
            message = %message,
            "Snowflake authenticator-request returned logical failure"
        );
        return AuthenticatorRequestRejectedSnafu { message, code }.fail();
    }

    resp.data.ok_or_else(|| ExternalBrowserError::MissingField {
        field: "data",
        location: Location::new(file!(), line!(), column!()),
    })
}

// ─── Callback listener ───────────────────────────────────────────────────────

#[derive(Debug)]
struct TokenFromCallback {
    token: SensitiveString,
    consent_cache_id_token: Option<bool>,
}

/// Accept HTTP requests on the listener until a token-bearing request arrives.
///
/// Handles:
/// - **OPTIONS** (CORS preflight): responds with permissive CORS headers and loops.
/// - **GET** `/?token=...`: extracts token from query string.
/// - **POST** with JSON body `{"token":"...","consent":true}` or form-encoded `token=...`.
/// - Non-token requests (e.g. `/favicon.ico`): silently closed, loops.
async fn accept_token_from_callback(
    listener: &TcpListener,
) -> Result<TokenFromCallback, ExternalBrowserError> {
    loop {
        let (mut stream, _addr) = listener.accept().await.context(CallbackIoSnafu)?;
        let request = read_http_request(&mut stream).await?;

        let first_line = request.lines().next().unwrap_or("");
        let method = first_line.split_whitespace().next().unwrap_or("");

        if method.eq_ignore_ascii_case("OPTIONS") {
            let origin = extract_header(&request, "Origin").unwrap_or_default();
            let resp = cors_preflight_response(&origin);
            send_response(&mut stream, &resp).await;
            tracing::debug!("Handled OPTIONS preflight, waiting for token request");
            continue;
        }

        let result = if method.eq_ignore_ascii_case("POST") {
            extract_token_from_post(&request)
        } else {
            extract_token_from_get(first_line)
        };

        let Some(callback) = result else {
            let _ = stream.shutdown().await;
            tracing::debug!(
                path = first_line.split_whitespace().nth(1).unwrap_or("?"),
                "Ignoring non-token request, waiting for IdP callback"
            );
            continue;
        };

        let origin = extract_header(&request, "Origin");
        let resp = success_response(origin.as_deref(), callback.consent_cache_id_token);
        send_response(&mut stream, &resp).await;

        return Ok(callback);
    }
}

// ─── HTTP request reading ────────────────────────────────────────────────────

/// Maximum total size of the request header section (all lines up to and
/// including the blank-line separator). Header parsing is bounded so a callback
/// connection uses a fixed amount of memory; the request body is separately
/// bounded by `MAX_CALLBACK_BODY_BYTES`.
const MAX_CALLBACK_HEADER_BYTES: usize = 64 * 1024;
/// Maximum size of a single header line. Bounds memory even when a line arrives
/// without its newline terminator.
const MAX_CALLBACK_HEADER_LINE_BYTES: usize = 8 * 1024;

/// Read one `\n`-terminated line from `reader`, appending at most `max_line`
/// bytes. Returns `Ok(None)` at end of input with no pending bytes. Returns
/// [`ExternalBrowserError::CallbackHeadersTooLarge`] if a line reaches
/// `max_line` bytes before a newline, so a single unterminated line cannot grow
/// memory without bound.
async fn read_header_line(
    reader: &mut (impl tokio::io::AsyncBufRead + Unpin),
    max_line: usize,
) -> Result<Option<String>, ExternalBrowserError> {
    let mut line: Vec<u8> = Vec::new();
    loop {
        let available = reader.fill_buf().await.context(CallbackIoSnafu)?;
        if available.is_empty() {
            if line.is_empty() {
                return Ok(None);
            }
            break;
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(pos) => {
                let take = pos + 1;
                ensure!(
                    line.len() + take <= max_line,
                    CallbackHeadersTooLargeSnafu {
                        max_bytes: MAX_CALLBACK_HEADER_BYTES,
                    }
                );
                line.extend_from_slice(&available[..take]);
                reader.consume(take);
                break;
            }
            None => {
                ensure!(
                    line.len() + available.len() <= max_line,
                    CallbackHeadersTooLargeSnafu {
                        max_bytes: MAX_CALLBACK_HEADER_BYTES,
                    }
                );
                let consumed = available.len();
                line.extend_from_slice(available);
                reader.consume(consumed);
            }
        }
    }
    Ok(Some(String::from_utf8_lossy(&line).into_owned()))
}

/// Read a full HTTP request: headers + body (using Content-Length).
///
/// Reads line-by-line until the blank line separator, then reads exactly
/// Content-Length bytes for the body. Both the header section and the body are
/// size-bounded so a callback connection uses a fixed amount of memory.
async fn read_http_request(
    stream: &mut (impl tokio::io::AsyncRead + Unpin),
) -> Result<String, ExternalBrowserError> {
    let mut reader = BufReader::new(stream);
    let mut raw_request = String::new();
    let mut content_length: usize = 0;
    let mut header_bytes: usize = 0;

    loop {
        let line = match read_header_line(&mut reader, MAX_CALLBACK_HEADER_LINE_BYTES).await? {
            Some(line) => line,
            None => break,
        };
        // Cap the cumulative header bytes across all lines.
        header_bytes = header_bytes.saturating_add(line.len());
        ensure!(
            header_bytes <= MAX_CALLBACK_HEADER_BYTES,
            CallbackHeadersTooLargeSnafu {
                max_bytes: MAX_CALLBACK_HEADER_BYTES,
            }
        );
        if line.trim().is_empty() {
            raw_request.push_str(&line);
            break;
        }
        if let Some((key, value)) = line.split_once(':')
            && key.trim().eq_ignore_ascii_case("Content-Length")
        {
            content_length = value.trim().parse().unwrap_or(0);
        }
        raw_request.push_str(&line);
    }

    if content_length > 0 {
        const MAX_CALLBACK_BODY_BYTES: usize = 20 * 1024 * 1024;
        ensure!(
            content_length <= MAX_CALLBACK_BODY_BYTES,
            CallbackBodyTooLargeSnafu {
                max_bytes: MAX_CALLBACK_BODY_BYTES,
            }
        );
        let mut body_buf = vec![0u8; content_length];
        reader
            .read_exact(&mut body_buf)
            .await
            .context(CallbackIoSnafu)?;
        raw_request.push_str(&String::from_utf8_lossy(&body_buf));
    }

    Ok(raw_request)
}

// ─── HTTP response helpers ───────────────────────────────────────────────────

async fn send_response(stream: &mut (impl AsyncWriteExt + Unpin), response: &str) {
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

fn http_response(status: &str, headers: &[(&str, &str)], body: &str) -> String {
    let mut resp = format!("HTTP/1.1 {status}\r\n");
    for (key, value) in headers {
        resp.push_str(&format!("{key}: {value}\r\n"));
    }
    resp.push_str(&format!("Content-Length: {}\r\n", body.len()));
    resp.push_str("Connection: close\r\n\r\n");
    resp.push_str(body);
    resp
}

fn cors_preflight_response(origin: &str) -> String {
    http_response(
        "200 OK",
        &[
            ("Access-Control-Allow-Origin", origin),
            ("Access-Control-Allow-Methods", "POST, GET, OPTIONS"),
            ("Access-Control-Allow-Headers", "Content-Type"),
            ("Access-Control-Max-Age", "86400"),
        ],
        "",
    )
}

fn success_response(origin: Option<&str>, consent: Option<bool>) -> String {
    match origin {
        Some(origin) => {
            let body = match consent {
                Some(v) => format!(r#"{{"consent":{v}}}"#),
                None => "{}".to_string(),
            };
            http_response(
                "200 OK",
                &[
                    ("Content-Type", "application/json"),
                    ("Access-Control-Allow-Origin", origin),
                    ("Vary", "Accept-Encoding, Origin"),
                ],
                &body,
            )
        }
        None => http_response("200 OK", &[("Content-Type", "text/html")], SUCCESS_HTML),
    }
}

// ─── Token extraction ────────────────────────────────────────────────────────

/// Extract a non-empty `token` from a GET request's query string.
fn extract_token_from_get(first_line: &str) -> Option<TokenFromCallback> {
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split_once('?').map(|(_, q)| q)?;
    let token = extract_nonempty_query_param(query, "token")?;
    Some(TokenFromCallback {
        token: SensitiveString::from(token),
        consent_cache_id_token: None,
    })
}

/// Extract `token` (and optionally `consent`) from a POST request body.
///
/// Tries JSON first (`{"token":"...","consent":true}`), then falls back
/// to form-encoded (`token=...`).
fn extract_token_from_post(request: &str) -> Option<TokenFromCallback> {
    let body = extract_http_body(request)?;
    if let Some(result) = extract_token_from_json(body) {
        return Some(result);
    }
    let token = extract_nonempty_query_param(body, "token")?;
    Some(TokenFromCallback {
        token: SensitiveString::from(token),
        consent_cache_id_token: None,
    })
}

fn extract_token_from_json(body: &str) -> Option<TokenFromCallback> {
    #[derive(Deserialize)]
    struct PostPayload {
        token: String,
        consent: Option<bool>,
    }
    let payload: PostPayload = serde_json::from_str(body).ok()?;
    if payload.token.is_empty() {
        return None;
    }
    Some(TokenFromCallback {
        token: SensitiveString::from(payload.token),
        consent_cache_id_token: payload.consent,
    })
}

/// Extract a named query parameter, returning `None` for missing or empty values.
fn extract_nonempty_query_param(query: &str, name: &str) -> Option<String> {
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=')
            && key == name
        {
            let decoded = urlencoding::decode(value).ok()?.into_owned();
            if decoded.is_empty() {
                return None;
            }
            return Some(decoded);
        }
    }
    None
}

// ─── Raw HTTP parsing helpers ────────────────────────────────────────────────

/// Extract the body from a raw HTTP request (everything after `\r\n\r\n`).
fn extract_http_body(request: &str) -> Option<&str> {
    let (_, body) = request.split_once("\r\n\r\n")?;
    let body = body.trim();
    if body.is_empty() { None } else { Some(body) }
}

/// Extract a header value by name (case-insensitive) from a raw HTTP request.
fn extract_header(request: &str, name: &str) -> Option<String> {
    for line in request.lines() {
        if let Some((key, value)) = line.split_once(':')
            && key.trim().eq_ignore_ascii_case(name)
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[test]
    fn authenticator_request_rejection_renders_the_snowflake_code_and_message() {
        let rendered = AuthenticatorRequestRejectedSnafu {
            message: "SSO URL generation failed in External browser's SAML Request flow",
            code: 390511,
        }
        .build()
        .to_string();

        assert!(rendered.contains("390511"), "{rendered}");
        assert!(rendered.contains("logical failure"), "{rendered}");
        assert!(
            rendered.contains("SSO URL generation failed in External browser's SAML Request flow"),
            "{rendered}"
        );
    }

    // ─── GET token extraction ────────────────────────────────────────────

    #[test]
    fn extract_token_get_basic() {
        let cb = extract_token_from_get("GET /?token=abc123 HTTP/1.1").unwrap();
        assert_eq!(cb.token.reveal(), "abc123");
    }

    #[test]
    fn extract_token_get_url_encoded() {
        let cb = extract_token_from_get("GET /?token=abc%20123%3D HTTP/1.1").unwrap();
        assert_eq!(cb.token.reveal(), "abc 123=");
    }

    #[test]
    fn extract_token_get_with_extra_params() {
        let cb = extract_token_from_get("GET /?foo=bar&token=mytoken&baz=qux HTTP/1.1").unwrap();
        assert_eq!(cb.token.reveal(), "mytoken");
        assert!(cb.consent_cache_id_token.is_none());
    }

    #[test]
    fn extract_token_get_missing() {
        assert!(extract_token_from_get("GET /?foo=bar HTTP/1.1").is_none());
    }

    #[test]
    fn extract_token_get_no_query_string() {
        assert!(extract_token_from_get("GET / HTTP/1.1").is_none());
    }

    #[test]
    fn extract_token_get_empty_value_is_rejected() {
        assert!(
            extract_token_from_get("GET /?token= HTTP/1.1").is_none(),
            "Empty token should be rejected"
        );
    }

    // ─── POST token extraction (JSON) ────────────────────────────────────

    #[test]
    fn extract_token_post_json_with_consent() {
        let request = "POST / HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"token\":\"json_token\",\"consent\":true}";
        let cb = extract_token_from_post(request).unwrap();
        assert_eq!(cb.token.reveal(), "json_token");
        assert_eq!(cb.consent_cache_id_token, Some(true));
    }

    #[test]
    fn extract_token_post_json_consent_false() {
        let request = "POST / HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"token\":\"tk\",\"consent\":false}";
        let cb = extract_token_from_post(request).unwrap();
        assert_eq!(cb.token.reveal(), "tk");
        assert_eq!(cb.consent_cache_id_token, Some(false));
    }

    #[test]
    fn extract_token_post_json_no_consent_field() {
        let request =
            "POST / HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"token\":\"tk2\"}";
        let cb = extract_token_from_post(request).unwrap();
        assert_eq!(cb.token.reveal(), "tk2");
        assert_eq!(cb.consent_cache_id_token, None);
    }

    #[test]
    fn extract_token_post_json_empty_token_rejected() {
        let request = "POST / HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"token\":\"\"}";
        assert!(
            extract_token_from_post(request).is_none(),
            "Empty JSON token should be rejected"
        );
    }

    // ─── POST token extraction (form-encoded) ────────────────────────────

    #[test]
    fn extract_token_post_form_encoded() {
        let request = "POST / HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\n\r\ntoken=form_token&extra=val";
        let cb = extract_token_from_post(request).unwrap();
        assert_eq!(cb.token.reveal(), "form_token");
        assert!(cb.consent_cache_id_token.is_none());
    }

    #[test]
    fn extract_token_post_no_body() {
        let request = "POST / HTTP/1.1\r\nContent-Type: application/json\r\n\r\n";
        assert!(extract_token_from_post(request).is_none());
    }

    // ─── Header extraction ───────────────────────────────────────────────

    #[test]
    fn extract_header_case_insensitive() {
        let request = "GET / HTTP/1.1\r\norigin: https://example.com\r\n\r\n";
        assert_eq!(
            extract_header(request, "Origin"),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn extract_header_missing() {
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert!(extract_header(request, "Origin").is_none());
    }

    // ─── Listener integration tests ──────────────────────────────────────

    #[tokio::test]
    async fn listener_accepts_get_token() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = tokio::spawn(async move { accept_token_from_callback(&listener).await });

        let mut client = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        client
            .write_all(b"GET /?token=test_token_value HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();

        let cb = server.await.unwrap().unwrap();
        assert_eq!(cb.token.reveal(), "test_token_value");
        assert!(cb.consent_cache_id_token.is_none());
        assert!(String::from_utf8_lossy(&response).contains("Your identity was confirmed"));
    }

    #[tokio::test]
    async fn listener_accepts_post_json_token() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = tokio::spawn(async move { accept_token_from_callback(&listener).await });

        let mut client = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        let body = r#"{"token":"post_json_token","consent":false}"#;
        let request = format!(
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        client.write_all(request.as_bytes()).await.unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();

        let cb = server.await.unwrap().unwrap();
        assert_eq!(cb.token.reveal(), "post_json_token");
        assert_eq!(cb.consent_cache_id_token, Some(false));
    }

    #[tokio::test]
    async fn listener_skips_favicon_then_accepts_token() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = tokio::spawn(async move { accept_token_from_callback(&listener).await });

        let mut favicon = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        favicon
            .write_all(b"GET /favicon.ico HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut fav_resp = Vec::new();
        favicon.read_to_end(&mut fav_resp).await.unwrap();

        let mut client = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        client
            .write_all(b"GET /?token=real_token HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();

        let cb = server.await.unwrap().unwrap();
        assert_eq!(cb.token.reveal(), "real_token");
    }

    #[tokio::test]
    async fn listener_handles_options_then_post() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = tokio::spawn(async move { accept_token_from_callback(&listener).await });

        let mut options_client = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        options_client
            .write_all(
                b"OPTIONS / HTTP/1.1\r\nHost: localhost\r\nOrigin: https://idp.example.com\r\nAccess-Control-Request-Method: POST\r\n\r\n",
            )
            .await
            .unwrap();
        let mut options_response = Vec::new();
        options_client
            .read_to_end(&mut options_response)
            .await
            .unwrap();
        assert!(
            String::from_utf8_lossy(&options_response).contains("Access-Control-Allow-Methods")
        );

        let mut post_client = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        let body = r#"{"token":"after_options","consent":true}"#;
        let request = format!(
            "POST / HTTP/1.1\r\nHost: localhost\r\nOrigin: https://idp.example.com\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        post_client.write_all(request.as_bytes()).await.unwrap();

        let mut post_response = Vec::new();
        post_client.read_to_end(&mut post_response).await.unwrap();
        let post_str = String::from_utf8_lossy(&post_response);
        assert!(post_str.contains("Access-Control-Allow-Origin"));
        assert!(post_str.contains(r#""consent":true"#));

        let cb = server.await.unwrap().unwrap();
        assert_eq!(cb.token.reveal(), "after_options");
        assert_eq!(cb.consent_cache_id_token, Some(true));
    }

    #[tokio::test]
    async fn listener_timeout_when_no_callback() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

        let result = tokio::time::timeout(
            Duration::from_millis(50),
            accept_token_from_callback(&listener),
        )
        .await;

        assert!(result.is_err(), "Should time out when no callback arrives");
    }

    #[tokio::test]
    async fn should_reject_callback_body_exceeding_size_cap() {
        let oversized_length = 20 * 1024 * 1024 + 1usize;
        let header = format!("POST / HTTP/1.1\r\nContent-Length: {oversized_length}\r\n\r\n");
        let mut stream = std::io::Cursor::new(header.into_bytes());
        let result = read_http_request(&mut stream).await;
        assert!(
            matches!(
                result,
                Err(ExternalBrowserError::CallbackBodyTooLarge { .. })
            ),
            "expected CallbackBodyTooLarge, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn should_reject_callback_headers_exceeding_total_size_cap() {
        // Many small header lines that never terminate with a blank line,
        // together exceeding the 64 KiB header cap.
        let mut request = String::from("POST / HTTP/1.1\r\n");
        let filler = format!("X-Filler: {}\r\n", "a".repeat(80));
        while request.len() <= 64 * 1024 {
            request.push_str(&filler);
        }
        let mut stream = std::io::Cursor::new(request.into_bytes());
        let result = read_http_request(&mut stream).await;
        assert!(
            matches!(
                result,
                Err(ExternalBrowserError::CallbackHeadersTooLarge { .. })
            ),
            "expected CallbackHeadersTooLarge, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn should_reject_callback_header_line_exceeding_line_cap() {
        // A single header line longer than the 8 KiB per-line limit.
        let long_value = "a".repeat(8 * 1024 + 1);
        let request = format!("POST / HTTP/1.1\r\nX-Long: {long_value}\r\n\r\n");
        let mut stream = std::io::Cursor::new(request.into_bytes());
        let result = read_http_request(&mut stream).await;
        assert!(
            matches!(
                result,
                Err(ExternalBrowserError::CallbackHeadersTooLarge { .. })
            ),
            "expected CallbackHeadersTooLarge, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn should_parse_request_with_headers_and_body_within_caps() {
        // Headers and body both within their caps parse successfully.
        let body = r#"{"token":"abc123"}"#;
        let request = format!(
            "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let mut stream = std::io::Cursor::new(request.into_bytes());
        let parsed = read_http_request(&mut stream)
            .await
            .expect("request within caps must parse");
        assert!(
            parsed.contains("abc123"),
            "body should be included, got: {parsed:?}"
        );
    }

    #[tokio::test]
    async fn injected_opener_receives_sso_url_and_completes_callback() {
        use std::io::{Read, Write};
        use std::sync::{Arc, Mutex};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, Request, ResponseTemplate};

        struct SsoRespond;
        impl wiremock::Respond for SsoRespond {
            fn respond(&self, request: &Request) -> ResponseTemplate {
                let body: serde_json::Value =
                    serde_json::from_slice(&request.body).expect("json body");
                let port = body["data"]["BROWSER_MODE_REDIRECT_PORT"]
                    .as_str()
                    .expect("BROWSER_MODE_REDIRECT_PORT");
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "success": true,
                    "data": {
                        "ssoUrl": format!(
                            "https://idp.snowflake.com/sso?browser_mode_redirect_port={port}"
                        ),
                        "proofKey": "proof"
                    }
                }))
            }
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/session/authenticator-request"))
            .respond_with(SsoRespond)
            .mount(&server)
            .await;

        let seen = Arc::new(Mutex::new(None::<String>));
        let seen_cb = Arc::clone(&seen);
        let opener = FnBrowserOpener(Arc::new(move |url: &str| {
            *seen_cb.lock().expect("lock") = Some(url.to_string());
            let port: u16 = url::Url::parse(url)
                .expect("sso url")
                .query_pairs()
                .find(|(k, _)| k == "browser_mode_redirect_port")
                .expect("port query")
                .1
                .parse()
                .expect("port");
            let mut stream =
                std::net::TcpStream::connect(("127.0.0.1", port)).expect("loopback connect");
            stream
                .write_all(b"GET /?token=sso-token HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("write token");
            let mut buf = Vec::new();
            let _ = stream.read_to_end(&mut buf);
            Ok(())
        }));

        let login_parameters = crate::config::rest_parameters::LoginParameters {
            account_name: "testaccount".to_string(),
            login_method: crate::config::rest_parameters::LoginMethod::ExternalBrowser {
                username: "alice".to_string(),
                authentication_timeout_secs: 30,
                client_store_temporary_credential: false,
            },
            server_url: server.uri(),
            database: None,
            schema: None,
            warehouse: None,
            role: None,
            secondary_roles: None,
            client_info: crate::config::rest_parameters::test_fixtures::test_client_info(),
            session_parameters: None,
            spcs_token: None,
            disable_parallel_user_prompt: true,
            validate_session_token: true,
            browser_opener: None,
        };

        let result = external_browser_authenticate(
            &reqwest::Client::new(),
            &login_parameters,
            "alice",
            30,
            Arc::new(opener),
            &crate::config::retry::RetryPolicy::default(),
        )
        .await
        .expect("sso flow");

        assert_eq!(result.token.reveal(), "sso-token");
        assert_eq!(result.proof_key.reveal(), "proof");
        let seen_url = seen.lock().expect("lock").clone().expect("opener called");
        assert!(
            seen_url.starts_with("https://idp.snowflake.com/sso?"),
            "opener must receive the IdP URL, got {seen_url}"
        );
    }

    #[tokio::test]
    async fn injected_opener_error_fails_without_waiting_for_callback() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, Request, ResponseTemplate};

        struct SsoRespond;
        impl wiremock::Respond for SsoRespond {
            fn respond(&self, request: &Request) -> ResponseTemplate {
                let body: serde_json::Value =
                    serde_json::from_slice(&request.body).expect("json body");
                let port = body["data"]["BROWSER_MODE_REDIRECT_PORT"]
                    .as_str()
                    .expect("BROWSER_MODE_REDIRECT_PORT");
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "success": true,
                    "data": {
                        "ssoUrl": format!(
                            "https://idp.snowflake.com/sso?browser_mode_redirect_port={port}"
                        ),
                        "proofKey": "proof"
                    }
                }))
            }
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/session/authenticator-request"))
            .respond_with(SsoRespond)
            .mount(&server)
            .await;

        let opener = FnBrowserOpener(Arc::new(
            |_url: &str| Err("you don't have a browser".into()),
        ));

        let login_parameters = crate::config::rest_parameters::LoginParameters {
            account_name: "testaccount".to_string(),
            login_method: crate::config::rest_parameters::LoginMethod::ExternalBrowser {
                username: "alice".to_string(),
                authentication_timeout_secs: 30,
                client_store_temporary_credential: false,
            },
            server_url: server.uri(),
            database: None,
            schema: None,
            warehouse: None,
            role: None,
            secondary_roles: None,
            client_info: crate::config::rest_parameters::test_fixtures::test_client_info(),
            session_parameters: None,
            spcs_token: None,
            disable_parallel_user_prompt: true,
            validate_session_token: true,
            browser_opener: None,
        };

        let started = Instant::now();
        let err = external_browser_authenticate(
            &reqwest::Client::new(),
            &login_parameters,
            "alice",
            30,
            Arc::new(opener),
            &crate::config::retry::RetryPolicy::default(),
        )
        .await
        .expect_err("custom opener Err must fail the flow");

        assert!(
            started.elapsed() < Duration::from_secs(5),
            "opener Err must fail without waiting out the authentication timeout"
        );
        assert!(
            matches!(
                err,
                ExternalBrowserError::BrowserCallback { ref reason, .. }
                    if reason == "you don't have a browser"
            ),
            "expected BrowserCallback with the opener message, got: {err}"
        );
    }
}
