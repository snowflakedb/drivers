use crate::config::rest_parameters::LoginParameters;
use crate::config::retry::RetryPolicy;
use crate::http::retry::{HttpContext, HttpError, execute_with_retry};
use crate::rest::snowflake::auth::{AuthRequest, AuthRequestData};
use reqwest::header;
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use snafu::{Location, ResultExt, Snafu};
use std::time::{Duration, Instant};
use url::Url;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum OktaError {
    #[snafu(display("Authentication timeout exceeded (budget {budget:?})"))]
    AuthenticationTimeout {
        budget: Duration,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse URL: {url}"))]
    UrlParse {
        url: String,
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("IdP URL safety validation failed: returned {returned} does not match configured Okta URL {configured}"))]
    IdpUrlMismatch {
        configured: String,
        returned: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("SAML postback destination validation failed: postback {postback} does not match Snowflake server {server}"))]
    SamlDestinationMismatch {
        server: String,
        postback: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Okta token endpoint rejected credentials (HTTP {status})"))]
    BadCredentials {
        status: StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Okta MFA required for this flow"))]
    MfaRequired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Okta token response missing one-time token"))]
    MissingOneTimeToken {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Okta token response missing relay state"))]
    MissingRelayState {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to extract SAML postback (form action) from HTML"))]
    MissingSamlPostback {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{context} failed with HTTP {status}"))]
    HttpStatus {
        context: &'static str,
        status: StatusCode,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing required field: {field}"))]
    MissingField {
        field: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("HTTP retry budget exhausted during Okta flow"))]
    RetryExhausted {
        source: HttpError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse JSON response"))]
    JsonParse {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Deserialize)]
struct AuthenticatorRequestResponse {
    success: bool,
    message: Option<String>,
    #[serde(rename = "code")]
    _code: Option<String>,
    data: Option<AuthenticatorRequestData>,
}

#[derive(Debug, Deserialize)]
struct AuthenticatorRequestData {
    #[serde(rename = "tokenUrl")]
    token_url: String,
    #[serde(rename = "ssoUrl")]
    sso_url: String,
}

#[derive(Debug, Deserialize)]
struct OktaTokenResponse {
    #[serde(rename = "sessionToken")]
    session_token: Option<String>,
    #[serde(rename = "cookieToken")]
    cookie_token: Option<String>,
    #[serde(rename = "relayState")]
    relay_state: Option<String>,
    status: Option<String>,
    #[serde(rename = "errorCode")]
    error_code: Option<String>,
    #[serde(rename = "errorSummary")]
    error_summary: Option<String>,
}

fn enrich_okta_error_body(body: &str) -> String {
    // Okta often returns a JSON error object with `errorCode` + `errorSummary`.
    // Use them to provide a clearer error message (and to avoid dead_code warnings).
    if let Ok(parsed) = serde_json::from_str::<OktaTokenResponse>(body) {
        if parsed.error_code.is_some() || parsed.error_summary.is_some() {
            let code = parsed.error_code.unwrap_or_else(|| "unknown".to_string());
            let summary = parsed
                .error_summary
                .unwrap_or_else(|| "unknown".to_string());
            return format!("Okta errorCode={code}, errorSummary={summary}; rawBody={body}");
        }
    }
    body.to_string()
}

fn url_origin_matches(a: &Url, b: &Url) -> bool {
    let a_port = a.port_or_known_default();
    let b_port = b.port_or_known_default();
    a.scheme() == b.scheme() && a.host_str() == b.host_str() && a_port == b_port
}

fn decode_html_entities_minimal(input: &str) -> String {
    // Minimal decoding sufficient for common SAML form action encodings we see in drivers.
    // Supports: &amp; &quot; &apos; &lt; &gt; and numeric entities (&#...; and &#x...;).
    fn starts_with_at(haystack: &[u8], i: usize, needle: &[u8]) -> bool {
        haystack.get(i..i + needle.len()) == Some(needle)
    }

    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }

        // named entities
        if starts_with_at(bytes, i, b"&amp;") {
            out.push('&');
            i += 5;
            continue;
        }
        if starts_with_at(bytes, i, b"&quot;") {
            out.push('"');
            i += 6;
            continue;
        }
        if starts_with_at(bytes, i, b"&apos;") {
            out.push('\'');
            i += 6;
            continue;
        }
        if starts_with_at(bytes, i, b"&lt;") {
            out.push('<');
            i += 4;
            continue;
        }
        if starts_with_at(bytes, i, b"&gt;") {
            out.push('>');
            i += 4;
            continue;
        }

        // numeric entity
        if i + 3 < bytes.len() && bytes[i + 1] == b'#' {
            let mut j = i + 2;
            let hex = j < bytes.len() && (bytes[j] == b'x' || bytes[j] == b'X');
            if hex {
                j += 1;
            }
            let start = j;
            while j < bytes.len() && bytes[j] != b';' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b';' {
                let num_str = &input[start..j];
                let parsed = if hex {
                    u32::from_str_radix(num_str, 16).ok()
                } else {
                    num_str.parse::<u32>().ok()
                };
                if let Some(cp) = parsed.and_then(char::from_u32) {
                    out.push(cp);
                    i = j + 1;
                    continue;
                }
            }
        }

        // fallback: keep raw '&'
        out.push('&');
        i += 1;
    }
    out
}

fn extract_form_action(html: &str) -> Option<String> {
    // Fast, non-validating extraction similar to other drivers: locate the first action="...".
    let lower = html.to_ascii_lowercase();
    let (needle, quote) = if let Some(idx) = lower.find("action=\"") {
        (idx, b'"')
    } else if let Some(idx) = lower.find("action='") {
        (idx, b'\'')
    } else {
        return None;
    };

    let start = needle + "action=".len() + 1; // + opening quote
    if start >= html.len() {
        return None;
    }
    let mut end = start;
    while end < html.len() && html.as_bytes()[end] != quote {
        end += 1;
    }
    if end >= html.len() {
        return None;
    }
    let raw = &html[start..end];
    Some(decode_html_entities_minimal(raw))
}

async fn request_text_with_retry(
    build: impl Fn() -> reqwest::RequestBuilder,
    ctx: &HttpContext,
    policy: &RetryPolicy,
) -> Result<(StatusCode, String), HttpError> {
    execute_with_retry(build, ctx, policy, |resp| async move {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| HttpError::Transport {
                source: e,
                location: Location::new(file!(), line!(), column!()),
            })?;
        Ok((status, text))
    })
    .await
}

fn remaining_policy(base: &RetryPolicy, start: Instant, budget: Duration) -> Result<RetryPolicy, OktaError> {
    let elapsed = start.elapsed();
    if elapsed >= budget {
        return AuthenticationTimeoutSnafu { budget }.fail();
    }
    let mut p = base.clone();
    p.max_elapsed = budget - elapsed;
    Ok(p)
}

pub(crate) async fn fetch_okta_saml_html(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    base_policy: &RetryPolicy,
    okta_url: &str,
    username: &str,
    password: &str,
    disable_saml_url_check: bool,
    authentication_timeout_secs: u64,
) -> Result<String, OktaError> {
    let budget = Duration::from_secs(authentication_timeout_secs);
    let start = Instant::now();

    // Step 1: authenticator-request (get tokenUrl + ssoUrl)
    let policy = remaining_policy(base_policy, start, budget)?;
    let mut data: AuthRequestData = super::base_auth_request_data(login_parameters);
    data.login_name = Some(username.to_string());
    data.authenticator = Some(okta_url.to_string());
    let authn_req = AuthRequest { data };
    let authn_url = format!("{}/session/authenticator-request", login_parameters.server_url);

    let body_string = serde_json::to_string(&authn_req).context(JsonParseSnafu)?;
    let ctx = HttpContext::new(Method::POST, "/session/authenticator-request").allow_post_retry();
    let (status, text) = request_text_with_retry(
        || {
            client
                .post(&authn_url)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "application/json")
                .header("User-Agent", super::user_agent(&login_parameters.client_info))
                .body(body_string.clone())
        },
        &ctx,
        &policy,
    )
    .await
    .context(RetryExhaustedSnafu)?;

    if !status.is_success() {
        return HttpStatusSnafu {
            context: "Snowflake authenticator-request",
            status,
            body: text,
        }
        .fail();
    }

    let idp: AuthenticatorRequestResponse = serde_json::from_str(&text).context(JsonParseSnafu)?;
    if !idp.success {
        let msg = idp.message.unwrap_or_else(|| "Unknown error".to_string());
        return HttpStatusSnafu {
            context: "Snowflake authenticator-request (logical failure)",
            status: StatusCode::BAD_REQUEST,
            body: msg,
        }
        .fail();
    }
    let idp_data = idp
        .data
        .ok_or_else(|| OktaError::MissingField {
            field: "data",
            location: Location::new(file!(), line!(), column!()),
        })?;

    // Step 2: IdP URL safety validation
    let configured = Url::parse(okta_url).context(UrlParseSnafu { url: okta_url })?;
    let token_url = Url::parse(&idp_data.token_url).context(UrlParseSnafu {
        url: idp_data.token_url.clone(),
    })?;
    let sso_url = Url::parse(&idp_data.sso_url).context(UrlParseSnafu {
        url: idp_data.sso_url.clone(),
    })?;
    if !url_origin_matches(&configured, &token_url) {
        return IdpUrlMismatchSnafu {
            configured: okta_url.to_string(),
            returned: idp_data.token_url,
        }
        .fail();
    }
    if !url_origin_matches(&configured, &sso_url) {
        return IdpUrlMismatchSnafu {
            configured: okta_url.to_string(),
            returned: idp_data.sso_url,
        }
        .fail();
    }

    // Step 3+4: mint one-time token, fetch SAML form. If SAML fetch fails transiently, re-mint token and retry.
    let mut saml_attempt: u32 = 0;
    loop {
        saml_attempt += 1;
        let policy = remaining_policy(base_policy, start, budget)?;

        // Step 3: token
        let token_ctx = HttpContext::new(Method::POST, "okta:token").allow_post_retry();
        let token_body = serde_json::json!({
            "username": username,
            "password": password,
        });
        let token_body_string = token_body.to_string();
        let (token_status, token_text) = request_text_with_retry(
            || {
                client
                    .post(token_url.clone())
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::ACCEPT, "application/json")
                    .body(token_body_string.clone())
            },
            &token_ctx,
            &policy,
        )
        .await
        .context(RetryExhaustedSnafu)?;

        if token_status == StatusCode::UNAUTHORIZED || token_status == StatusCode::FORBIDDEN {
            return BadCredentialsSnafu { status: token_status }.fail();
        }
        if !token_status.is_success() {
            return HttpStatusSnafu {
                context: "Okta token request",
                status: token_status,
                body: enrich_okta_error_body(&token_text),
            }
            .fail();
        }

        let token_resp: OktaTokenResponse =
            serde_json::from_str(&token_text).context(JsonParseSnafu)?;
        if token_resp.status.as_deref() == Some("MFA_REQUIRED") {
            return MfaRequiredSnafu.fail();
        }
        let one_time = token_resp
            .session_token
            .or(token_resp.cookie_token)
            .ok_or_else(|| OktaError::MissingOneTimeToken {
                location: Location::new(file!(), line!(), column!()),
            })?;
        // relayState is optional - Okta doesn't always return it
        let relay_state = token_resp.relay_state.unwrap_or_default();

        // Step 4: fetch SAML HTML form
        let policy = remaining_policy(base_policy, start, budget)?;
        let saml_ctx = HttpContext::new(Method::GET, "okta:saml");
        let (saml_status, saml_html) = request_text_with_retry(
            || {
                client
                    .get(sso_url.clone())
                    .query(&[("RelayState", relay_state.as_str()), ("onetimetoken", one_time.as_str())])
            },
            &saml_ctx,
            &policy,
        )
        .await
        .context(RetryExhaustedSnafu)?;

        if !saml_status.is_success() {
            if saml_status == StatusCode::UNAUTHORIZED || saml_status == StatusCode::FORBIDDEN {
                return HttpStatusSnafu {
                    context: "Okta SAML fetch (unauthorized)",
                    status: saml_status,
                    body: saml_html,
                }
                .fail();
            }
            // On non-success statuses (e.g., 429/5xx), shared retry policy already applied; treat as transient and retry by re-minting token.
            if saml_attempt >= base_policy.max_attempts {
                return HttpStatusSnafu {
                    context: "Okta SAML fetch",
                    status: saml_status,
                    body: saml_html,
                }
                .fail();
            }
            continue;
        }

        // Step 4b: destination/postback validation (unless disabled)
        let Some(postback) = extract_form_action(&saml_html) else {
            // Some drivers treat “postback not found” as retryable by re-minting the token.
            if saml_attempt < base_policy.max_attempts {
                continue;
            }
            return MissingSamlPostbackSnafu.fail();
        };

        if !disable_saml_url_check {
            let server = Url::parse(&login_parameters.server_url).context(UrlParseSnafu {
                url: login_parameters.server_url.clone(),
            })?;
            let postback_url = Url::parse(&postback).context(UrlParseSnafu { url: postback.clone() })?;
            if !url_origin_matches(&server, &postback_url) {
                return SamlDestinationMismatchSnafu {
                    server: login_parameters.server_url.clone(),
                    postback,
                }
                .fail();
            }
        }

        return Ok(saml_html);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // HTML Entity Decoding Tests
    // =========================================================================

    #[test]
    fn should_decode_html_entities_in_form_action_url() {
        // Given HTML containing form action with encoded entities
        // When Extracting form action from HTML
        // Then Form action URL is correctly decoded
        assert_eq!(
            decode_html_entities_minimal("https&#x3a;&#x2f;&#x2f;example.com&#x2f;f"),
            "https://example.com/f"
        );
    }

    #[test]
    fn test_decode_html_entities_named_entities() {
        assert_eq!(decode_html_entities_minimal("a&amp;b"), "a&b");
        assert_eq!(decode_html_entities_minimal("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode_html_entities_minimal("&quot;quoted&quot;"), "\"quoted\"");
        assert_eq!(decode_html_entities_minimal("it&apos;s"), "it's");
    }

    #[test]
    fn test_decode_html_entities_numeric_decimal() {
        assert_eq!(decode_html_entities_minimal("&#58;"), ":");
        assert_eq!(decode_html_entities_minimal("&#47;"), "/");
        assert_eq!(decode_html_entities_minimal("&#65;"), "A");
    }

    #[test]
    fn test_decode_html_entities_passthrough_invalid() {
        // Invalid/incomplete entities should be preserved
        assert_eq!(decode_html_entities_minimal("&unknown;"), "&unknown;");
        assert_eq!(decode_html_entities_minimal("&#;"), "&#;");
        assert_eq!(decode_html_entities_minimal("&#xZZZ;"), "&#xZZZ;");
    }

    // =========================================================================
    // Form Action Extraction Tests
    // =========================================================================

    #[test]
    fn should_extract_form_action_with_double_quotes() {
        // Given HTML containing form with action in double quotes
        // When Extracting form action from HTML
        // Then Form action URL is extracted correctly
        let html = r#"<html><form method="post" action="https&#x3a;&#x2f;&#x2f;acct.snowflakecomputing.com/fed"></form></html>"#;
        let action = extract_form_action(html).unwrap();
        assert_eq!(action, "https://acct.snowflakecomputing.com/fed");
    }

    #[test]
    fn should_extract_form_action_with_single_quotes() {
        // Given HTML containing form with action in single quotes
        // When Extracting form action from HTML
        // Then Form action URL is extracted correctly
        let html = r#"<html><form method='post' action='https://acct.snowflakecomputing.com/fed'></form></html>"#;
        let action = extract_form_action(html).unwrap();
        assert_eq!(action, "https://acct.snowflakecomputing.com/fed");
    }

    #[test]
    fn test_extract_form_action_case_insensitive() {
        let html = r#"<html><form METHOD="post" ACTION="https://example.com/fed"></form></html>"#;
        let action = extract_form_action(html).unwrap();
        assert_eq!(action, "https://example.com/fed");
    }

    #[test]
    fn test_extract_form_action_returns_none_when_missing() {
        let html = r#"<html><form method="post"><input name="test"/></form></html>"#;
        assert!(extract_form_action(html).is_none());
    }

    #[test]
    fn test_extract_form_action_with_complex_html() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>SAML</title></head>
            <body>
                <form method="post" action="https&#x3a;&#x2f;&#x2f;account.snowflakecomputing.com&#x2f;fed&#x2f;login">
                    <input type="hidden" name="SAMLResponse" value="PHNhbWw..." />
                    <input type="submit" value="Submit" />
                </form>
            </body>
            </html>
        "#;
        let action = extract_form_action(html).unwrap();
        assert_eq!(action, "https://account.snowflakecomputing.com/fed/login");
    }

    // =========================================================================
    // Error Enrichment Tests
    // =========================================================================

    #[test]
    fn should_enrich_okta_error_body_with_error_code_and_summary() {
        // Given Okta JSON error response body
        // When Enriching error body
        // Then Enriched message contains errorCode and errorSummary
        let body = r#"{"errorCode":"E0000004","errorSummary":"Authentication failed"}"#;
        let enriched = enrich_okta_error_body(body);
        assert!(enriched.contains("E0000004"));
        assert!(enriched.contains("Authentication failed"));
    }

    #[test]
    fn test_enrich_okta_error_body_passthrough_non_json() {
        let body = "Not a JSON response";
        let enriched = enrich_okta_error_body(body);
        assert_eq!(enriched, body);
    }

    #[test]
    fn test_enrich_okta_error_body_passthrough_json_without_error_fields() {
        let body = r#"{"sessionToken":"abc123"}"#;
        let enriched = enrich_okta_error_body(body);
        assert_eq!(enriched, body);
    }

    // =========================================================================
    // URL Origin Matching Tests
    // =========================================================================

    #[test]
    fn test_url_origin_matches_same_origin() {
        let a = Url::parse("https://example.okta.com/api/v1/authn").unwrap();
        let b = Url::parse("https://example.okta.com/app/sso/saml").unwrap();
        assert!(url_origin_matches(&a, &b));
    }

    #[test]
    fn test_url_origin_matches_different_host() {
        let a = Url::parse("https://example.okta.com/api").unwrap();
        let b = Url::parse("https://attacker.evil.com/api").unwrap();
        assert!(!url_origin_matches(&a, &b));
    }

    #[test]
    fn test_url_origin_matches_different_scheme() {
        let a = Url::parse("https://example.okta.com/api").unwrap();
        let b = Url::parse("http://example.okta.com/api").unwrap();
        assert!(!url_origin_matches(&a, &b));
    }

    #[test]
    fn test_url_origin_matches_different_port() {
        let a = Url::parse("https://example.okta.com:443/api").unwrap();
        let b = Url::parse("https://example.okta.com:8443/api").unwrap();
        assert!(!url_origin_matches(&a, &b));
    }

    #[test]
    fn test_url_origin_matches_default_port() {
        // Should match when one uses explicit default port and one doesn't
        let a = Url::parse("https://example.okta.com/api").unwrap();
        let b = Url::parse("https://example.okta.com:443/api").unwrap();
        assert!(url_origin_matches(&a, &b));
    }
}

