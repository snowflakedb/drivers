use crate::config::rest_parameters::{LoginParameters, NativeOktaConfig};
use crate::config::retry::RetryPolicy;
use crate::http::retry::{HttpContext, HttpError, execute_with_retry};
use crate::rest::snowflake::auth::{AuthRequest, AuthRequestData};
use html_escape::decode_html_entities;
use reqwest::header;
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use snafu::{Location, ResultExt, Snafu};
use std::time::{Duration, Instant};
use url::Url;

const SF_AUTHENTICATOR_REQUEST_PATH: &str = "/session/authenticator-request";

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum NativeOktaError {
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
    #[snafu(display(
        "IdP URL safety validation failed: returned {returned} does not match configured Okta URL {configured}"
    ))]
    IdpUrlMismatch {
        configured: String,
        returned: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "SAML postback destination validation failed: postback {postback} does not match Snowflake server {server}"
    ))]
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
    if let Ok(parsed) = serde_json::from_str::<OktaTokenResponse>(body)
        && (parsed.error_code.is_some() || parsed.error_summary.is_some())
    {
        let code = parsed.error_code.unwrap_or_else(|| "unknown".to_string());
        let summary = parsed
            .error_summary
            .unwrap_or_else(|| "unknown".to_string());
        return format!("Okta errorCode={code}, errorSummary={summary}; rawBody={body}");
    }
    body.to_string()
}

fn url_origin_matches(a: &Url, b: &Url) -> bool {
    let a_port = a.port_or_known_default();
    let b_port = b.port_or_known_default();
    a.scheme() == b.scheme() && a.host_str() == b.host_str() && a_port == b_port
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
        // Truncated or malformed HTML — return None so that the caller
        // (fetch_saml_with_retries) can re-mint the token and retry.
        return None;
    }
    let raw = &html[start..end];
    Some(decode_html_entities(raw).into_owned())
}

async fn request_text_with_retry(
    build: impl Fn() -> reqwest::RequestBuilder,
    ctx: &HttpContext,
    policy: &RetryPolicy,
) -> Result<(StatusCode, String), HttpError> {
    execute_with_retry(build, ctx, policy, |resp| async move {
        let status = resp.status();
        let text = resp.text().await.map_err(|e| HttpError::Transport {
            source: e,
            location: Location::new(file!(), line!(), column!()),
        })?;
        Ok((status, text))
    })
    .await
}

fn remaining_policy(
    base: &RetryPolicy,
    start: Instant,
    budget: Duration,
) -> Result<RetryPolicy, NativeOktaError> {
    let elapsed = start.elapsed();
    if elapsed >= budget {
        return AuthenticationTimeoutSnafu { budget }.fail();
    }
    let mut p = base.clone();
    p.max_elapsed = budget - elapsed;
    Ok(p)
}

#[tracing::instrument(
    skip(client, login_parameters, base_policy, config),
    fields(authentication_timeout_secs = config.authentication_timeout_secs)
)]
pub(crate) async fn fetch_native_okta_saml(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    base_policy: &RetryPolicy,
    config: &NativeOktaConfig,
) -> Result<String, NativeOktaError> {
    tracing::info!("Starting native Okta authentication");
    let budget = Duration::from_secs(config.authentication_timeout_secs);
    let start = Instant::now();

    // Step 1: ask Snowflake for the IdP endpoints (tokenUrl + ssoUrl).
    let idp_data = request_authenticator_endpoints(
        client,
        login_parameters,
        base_policy,
        config,
        start,
        budget,
    )
    .await?;

    // Step 2: verify that Snowflake-returned IdP URLs match the configured Okta URL.
    let (token_url, sso_url) = validate_idp_urls(&config.okta_url, &idp_data, start)?;

    // Steps 3+4: mint one-time token, fetch SAML form, validate.
    // Re-mints the token and retries on transient failures.
    fetch_saml_with_retries(
        client,
        login_parameters,
        base_policy,
        config,
        &token_url,
        &sso_url,
        start,
    )
    .await
}

/// Step 1: POST /session/authenticator-request to get tokenUrl + ssoUrl.
async fn request_authenticator_endpoints(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    base_policy: &RetryPolicy,
    config: &NativeOktaConfig,
    start: Instant,
    budget: Duration,
) -> Result<AuthenticatorRequestData, NativeOktaError> {
    let policy = remaining_policy(base_policy, start, budget)?;
    let mut data: AuthRequestData = super::base_auth_request_data(login_parameters);
    data.login_name = Some(config.username.to_string());
    data.authenticator = Some(config.okta_url.as_str().to_string());
    let authn_req = AuthRequest { data };
    let authn_url = format!(
        "{}{}",
        login_parameters.server_url, SF_AUTHENTICATOR_REQUEST_PATH
    );

    let body_string = serde_json::to_string(&authn_req).context(JsonSerializeSnafu)?;
    let ctx = HttpContext::new(Method::POST, SF_AUTHENTICATOR_REQUEST_PATH).allow_post_retry();
    let (status, text) = request_text_with_retry(
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
        &ctx,
        &policy,
    )
    .await
    .context(RetryExhaustedSnafu)?;

    tracing::debug!(
        status = %status,
        elapsed_ms = start.elapsed().as_millis(),
        "Snowflake authenticator-request completed"
    );

    if !status.is_success() {
        tracing::error!(
            status = %status,
            elapsed_ms = start.elapsed().as_millis(),
            "Snowflake authenticator-request failed"
        );
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
        tracing::error!(
            message = %msg,
            elapsed_ms = start.elapsed().as_millis(),
            "Snowflake authenticator-request returned logical failure"
        );
        return HttpStatusSnafu {
            context: "Snowflake authenticator-request (logical failure)",
            status: StatusCode::BAD_REQUEST,
            body: msg,
        }
        .fail();
    }
    let data = idp.data.ok_or_else(|| NativeOktaError::MissingField {
        field: "data",
        location: Location::new(file!(), line!(), column!()),
    })?;
    tracing::debug!("Received IdP endpoints from Snowflake");
    Ok(data)
}

/// Step 2: validate that the returned tokenUrl and ssoUrl share the same origin
/// as the configured Okta URL.
fn validate_idp_urls(
    okta_url: &Url,
    idp_data: &AuthenticatorRequestData,
    start: Instant,
) -> Result<(Url, Url), NativeOktaError> {
    let token_url = Url::parse(&idp_data.token_url).context(UrlParseSnafu {
        url: idp_data.token_url.clone(),
    })?;
    let sso_url = Url::parse(&idp_data.sso_url).context(UrlParseSnafu {
        url: idp_data.sso_url.clone(),
    })?;
    if !url_origin_matches(okta_url, &token_url) {
        tracing::error!(
            configured = %okta_url,
            returned = %idp_data.token_url,
            elapsed_ms = start.elapsed().as_millis(),
            "IdP tokenUrl does not match configured Okta URL"
        );
        return IdpUrlMismatchSnafu {
            configured: okta_url.to_string(),
            returned: idp_data.token_url.clone(),
        }
        .fail();
    }
    if !url_origin_matches(okta_url, &sso_url) {
        tracing::error!(
            configured = %okta_url,
            returned = %idp_data.sso_url,
            elapsed_ms = start.elapsed().as_millis(),
            "IdP ssoUrl does not match configured Okta URL"
        );
        return IdpUrlMismatchSnafu {
            configured: okta_url.to_string(),
            returned: idp_data.sso_url.clone(),
        }
        .fail();
    }
    tracing::debug!("IdP URL safety validation passed");
    Ok((token_url, sso_url))
}

/// Steps 3+4: mint a one-time token from Okta, fetch the SAML HTML, and
/// validate the postback URL.  Re-mints the token on transient failures.
async fn fetch_saml_with_retries(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    base_policy: &RetryPolicy,
    config: &NativeOktaConfig,
    token_url: &Url,
    sso_url: &Url,
    start: Instant,
) -> Result<String, NativeOktaError> {
    let budget = Duration::from_secs(config.authentication_timeout_secs);
    let idp_login = config.okta_username.as_deref().unwrap_or(&config.username);
    tracing::debug!(
        budget_secs = config.authentication_timeout_secs,
        max_attempts = base_policy.max_attempts,
        "Starting SAML fetch with retries"
    );

    let mut saml_attempt: u32 = 0;
    loop {
        saml_attempt += 1;
        let policy = remaining_policy(base_policy, start, budget)?;

        // Step 3: request a one-time token from Okta.
        let (one_time, relay_state) = request_okta_token(
            client,
            token_url,
            idp_login,
            &config.password,
            &policy,
            start,
        )
        .await?;
        tracing::debug!(attempt = saml_attempt, "Obtained one-time token from Okta");

        // Step 4: fetch SAML HTML form.
        // Use a single-attempt policy so transient errors bubble up to this
        // outer loop, allowing us to re-mint the one-time token before retrying.
        let mut single_attempt_policy = remaining_policy(base_policy, start, budget)?;
        single_attempt_policy.max_attempts = 1;
        let saml_ctx = HttpContext::new(Method::GET, "okta:saml");
        let saml_result = request_text_with_retry(
            || {
                client.get(sso_url.clone()).query(&[
                    ("RelayState", relay_state.as_str()),
                    ("onetimetoken", one_time.as_str()),
                ])
            },
            &saml_ctx,
            &single_attempt_policy,
        )
        .await;

        let (saml_status, saml_html) = match saml_result {
            Ok(result) => result,
            Err(e) if saml_attempt < base_policy.max_attempts => {
                tracing::warn!(
                    error = %e,
                    attempt = saml_attempt,
                    elapsed_ms = start.elapsed().as_millis(),
                    "SAML fetch failed transiently, re-minting token and retrying"
                );
                continue;
            }
            Err(e) => return Err(e).context(RetryExhaustedSnafu),
        };

        if !saml_status.is_success() {
            if saml_status == StatusCode::UNAUTHORIZED || saml_status == StatusCode::FORBIDDEN {
                return HttpStatusSnafu {
                    context: "Okta SAML fetch (unauthorized)",
                    status: saml_status,
                    body: saml_html,
                }
                .fail();
            }
            if saml_attempt >= base_policy.max_attempts {
                return HttpStatusSnafu {
                    context: "Okta SAML fetch (retries exhausted)",
                    status: saml_status,
                    body: saml_html,
                }
                .fail();
            }
            tracing::warn!(
                status = %saml_status,
                attempt = saml_attempt,
                elapsed_ms = start.elapsed().as_millis(),
                "SAML fetch returned non-success status, re-minting token and retrying"
            );
            continue;
        }

        // Step 4b: destination/postback validation (unless disabled).
        let Some(postback) = extract_form_action(&saml_html) else {
            if saml_attempt < base_policy.max_attempts {
                tracing::warn!(
                    attempt = saml_attempt,
                    elapsed_ms = start.elapsed().as_millis(),
                    "SAML HTML missing form action, re-minting token and retrying"
                );
                continue;
            }
            tracing::error!(
                elapsed_ms = start.elapsed().as_millis(),
                "Failed to extract form action from SAML HTML after max attempts"
            );
            return MissingSamlPostbackSnafu.fail();
        };

        if !config.disable_saml_url_check {
            validate_saml_postback(login_parameters, &postback, start)?;
        }

        tracing::info!(
            elapsed_ms = start.elapsed().as_millis(),
            "Native Okta authentication completed successfully"
        );
        return Ok(saml_html);
    }
}

/// Step 3: POST to Okta's `/api/v1/authn` to get a one-time session token.
async fn request_okta_token(
    client: &reqwest::Client,
    token_url: &Url,
    username: &str,
    password: &str,
    policy: &RetryPolicy,
    start: Instant,
) -> Result<(String, String), NativeOktaError> {
    let token_ctx = HttpContext::new(Method::POST, "okta:token").allow_post_retry();
    let token_body = serde_json::json!({
        "username": username,
        "password": password,
    });
    let token_body_string = token_body.to_string();
    let (status, text) = request_text_with_retry(
        || {
            client
                .post(token_url.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "application/json")
                .body(token_body_string.clone())
        },
        &token_ctx,
        policy,
    )
    .await
    .context(RetryExhaustedSnafu)?;

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        tracing::error!(
            status = %status,
            elapsed_ms = start.elapsed().as_millis(),
            "Okta rejected credentials"
        );
        return BadCredentialsSnafu { status }.fail();
    }
    tracing::debug!(
        status = %status,
        elapsed_ms = start.elapsed().as_millis(),
        "Okta token endpoint responded"
    );

    if !status.is_success() {
        tracing::error!(
            status = %status,
            elapsed_ms = start.elapsed().as_millis(),
            "Okta token request failed"
        );
        return HttpStatusSnafu {
            context: "Okta token request",
            status,
            body: enrich_okta_error_body(&text),
        }
        .fail();
    }

    let resp: OktaTokenResponse = serde_json::from_str(&text).context(JsonParseSnafu)?;
    if resp.status.as_deref() == Some("MFA_REQUIRED") {
        tracing::error!(
            elapsed_ms = start.elapsed().as_millis(),
            "Okta returned MFA_REQUIRED - unsupported in native Okta flow"
        );
        return MfaRequiredSnafu.fail();
    }
    let one_time = resp.session_token.or(resp.cookie_token).ok_or_else(|| {
        NativeOktaError::MissingOneTimeToken {
            location: Location::new(file!(), line!(), column!()),
        }
    })?;
    let relay_state = resp.relay_state.unwrap_or_default();
    tracing::debug!(
        elapsed_ms = start.elapsed().as_millis(),
        has_relay_state = !relay_state.is_empty(),
        "Successfully obtained one-time token from Okta"
    );
    Ok((one_time, relay_state))
}

/// Step 4b: verify the SAML postback URL matches the Snowflake server origin.
fn validate_saml_postback(
    login_parameters: &LoginParameters,
    postback: &str,
    start: Instant,
) -> Result<(), NativeOktaError> {
    let server = Url::parse(&login_parameters.server_url).context(UrlParseSnafu {
        url: login_parameters.server_url.clone(),
    })?;
    let postback_url = Url::parse(postback).context(UrlParseSnafu {
        url: postback.to_string(),
    })?;
    if !url_origin_matches(&server, &postback_url) {
        tracing::error!(
            server = %login_parameters.server_url,
            postback = %postback,
            elapsed_ms = start.elapsed().as_millis(),
            "SAML postback destination does not match Snowflake server"
        );
        return SamlDestinationMismatchSnafu {
            server: login_parameters.server_url.clone(),
            postback: postback.to_string(),
        }
        .fail();
    }
    tracing::debug!("SAML postback destination validation passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
