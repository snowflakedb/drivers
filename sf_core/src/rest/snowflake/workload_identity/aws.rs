//! AWS attestation for Workload Identity Federation.
//!
//! **Default** — pre-signed STS `GetCallerIdentity` request encoded as base64
//! JSON `{"url":…,"method":"POST","headers":{…}}`.  Snowflake GS replays this
//! request server-side to verify the caller's identity.  No outbound STS call
//! is made by the driver.
//!
//! **Opt-in** — when `SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN=true` the driver
//! calls `sts:GetWebIdentityToken` directly and forwards the resulting JWT.
//!
//! **Impersonation** — in both modes the driver first walks
//! `workload_identity_impersonation_path` via `sts:AssumeRole`, using the
//! resulting temporary credentials for the final step.
//!
//! The AWS region is resolved from (in priority order):
//! 1. `AWS_REGION` environment variable.
//! 2. `AWS_DEFAULT_REGION` environment variable.
//! 3. EC2 instance metadata (IMDS) `placement/region`.
//! 4. Falls back to `us-east-1` if none of the above are set.

use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_credential_types::provider::ProvideCredentials as _;
use aws_sdk_sts::config::SharedCredentialsProvider;
use aws_sdk_sts::{Client as StsClient, config::Builder as StsConfigBuilder};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use futures::FutureExt as _;
use futures::future::BoxFuture;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::collections::BTreeMap;

use crate::config::rest_parameters::WorkloadIdentityConfig;

use super::AttestationEndpoints;

const SNOWFLAKE_AUDIENCE: &str = "snowflakecomputing.com";
const AWS_WIF_SIGNING_ALGORITHM: &str = "ES384";
const EMPTY_BODY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

type HmacSha256 = Hmac<Sha256>;

/// Errors raised while building an AWS Workload Identity attestation.
///
/// Heterogeneous AWS SDK errors are erased to `Box<dyn Error>` via
/// `.boxed()` before being attached as a Snafu `source`.
#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum AwsAttestationError {
    #[snafu(display("No AWS credentials provider configured"))]
    NoCredentialsProvider {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to load AWS credentials"))]
    CredentialsLoad {
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("AWS AssumeRole for '{role_arn}' failed"))]
    AssumeRole {
        role_arn: String,
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("AWS AssumeRole for '{role_arn}' returned no credentials"))]
    AssumeRoleMissingCredentials {
        role_arn: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("AWS impersonation chain was empty (internal error)"))]
    ImpersonationChainEmpty {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("AWS STS GetWebIdentityToken failed"))]
    WebIdentityToken {
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("AWS STS GetWebIdentityToken returned an empty token"))]
    WebIdentityTokenEmpty {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize GetCallerIdentity token"))]
    TokenSerialize {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to create HMAC-SHA256 instance"))]
    HmacInit {
        source: hmac::digest::InvalidLength,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Primary AWS WIF entry point.
///
/// Dispatches to the pre-signed `GetCallerIdentity` path (default) or the
/// outbound `GetWebIdentityToken` path when
/// `SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN=true`.
pub(super) async fn get_attestation_token(
    client: &reqwest::Client,
    config: &WorkloadIdentityConfig,
    endpoints: &AttestationEndpoints,
) -> Result<String, AwsAttestationError> {
    if enable_outbound_token() {
        get_web_identity_token(client, config, endpoints).await
    } else {
        get_caller_identity_token(config, endpoints).await
    }
}

/// Returns `true` when the caller explicitly opts into the outbound-token path.
fn enable_outbound_token() -> bool {
    std::env::var("SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Pre-signed GetCallerIdentity (default)
// ---------------------------------------------------------------------------

/// Build a pre-signed STS `GetCallerIdentity` request and return it
/// base64-encoded as `{"url":…,"method":"POST","headers":{…}}`.
async fn get_caller_identity_token(
    config: &WorkloadIdentityConfig,
    endpoints: &AttestationEndpoints,
) -> Result<String, AwsAttestationError> {
    let region = resolve_region(endpoints).await;
    let credentials = resolve_credentials(config, &region).await?;

    let now = chrono::Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();

    let request = build_signed_caller_identity_request(
        credentials.access_key_id(),
        credentials.secret_access_key(),
        credentials.session_token(),
        &region,
        &amz_date,
        &date_stamp,
    )?;

    let json = serde_json::to_string(&request).context(TokenSerializeSnafu)?;
    Ok(BASE64.encode(json.as_bytes()))
}

#[derive(serde::Serialize)]
struct SignedCallerIdentityRequest {
    url: String,
    method: &'static str,
    headers: BTreeMap<String, String>,
}

fn build_signed_caller_identity_request(
    access_key_id: &str,
    secret_access_key: &str,
    session_token: Option<&str>,
    region: &str,
    amz_date: &str,
    date_stamp: &str,
) -> Result<SignedCallerIdentityRequest, AwsAttestationError> {
    let host = sts_hostname(region);
    let url = format!("https://{host}/?Action=GetCallerIdentity&Version=2011-06-15");

    // Canonical headers: alphabetical, lowercase, each followed by \n.
    let mut canonical_headers = format!("host:{host}\nx-amz-date:{amz_date}\n");
    let mut signed_header_names = vec!["host", "x-amz-date"];

    if let Some(token) = session_token {
        canonical_headers.push_str(&format!("x-amz-security-token:{token}\n"));
        signed_header_names.push("x-amz-security-token");
    }
    canonical_headers.push_str(&format!("x-snowflake-audience:{SNOWFLAKE_AUDIENCE}\n"));
    signed_header_names.push("x-snowflake-audience");

    let signed_headers = signed_header_names.join(";");

    // Canonical request (Task 1)
    let canonical_request = format!(
        "POST\n/\nAction=GetCallerIdentity&Version=2011-06-15\n{canonical_headers}\n{signed_headers}\n{EMPTY_BODY_HASH}"
    );

    // String to sign (Task 2)
    let credential_scope = format!("{date_stamp}/{region}/sts/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );

    // Signing key (Task 3)
    let signing_key = derive_signing_key(secret_access_key, date_stamp, region, "sts")?;

    // Signature (Task 4)
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes())?);

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={access_key_id}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    let mut headers = BTreeMap::new();
    headers.insert("Authorization".to_string(), authorization);
    headers.insert("Host".to_string(), host);
    headers.insert("X-Amz-Date".to_string(), amz_date.to_string());
    if let Some(token) = session_token {
        headers.insert("X-Amz-Security-Token".to_string(), token.to_string());
    }
    headers.insert(
        "X-Snowflake-Audience".to_string(),
        SNOWFLAKE_AUDIENCE.to_string(),
    );

    Ok(SignedCallerIdentityRequest {
        url,
        method: "POST",
        headers,
    })
}

// ---------------------------------------------------------------------------
// Outbound GetWebIdentityToken (opt-in via SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN)
// ---------------------------------------------------------------------------

/// Acquire an AWS `GetWebIdentityToken` JWT for Workload Identity Federation.
///
/// Only called when `SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN=true`.
async fn get_web_identity_token(
    _client: &reqwest::Client,
    config: &WorkloadIdentityConfig,
    endpoints: &AttestationEndpoints,
) -> Result<String, AwsAttestationError> {
    let region = resolve_region(endpoints).await;
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region.clone()))
        .load()
        .await;

    let sts_client = StsClient::new(&sdk_config);

    let credentials = if config.impersonation_path.is_empty() {
        None
    } else {
        Some(chain_assume_role(&region, &config.impersonation_path).await?)
    };

    let final_sts_client = if let Some(creds) = credentials {
        let final_config = StsConfigBuilder::from(&sdk_config)
            .credentials_provider(SharedCredentialsProvider::new(creds))
            .build();
        StsClient::from_conf(final_config)
    } else {
        sts_client
    };

    tracing::info!("STS GetWebIdentityToken request");
    let result = final_sts_client
        .get_web_identity_token()
        .audience(SNOWFLAKE_AUDIENCE)
        .signing_algorithm(AWS_WIF_SIGNING_ALGORITHM)
        .send()
        .await;
    if let Err(ref e) = result
        && let Some(status) = sts_sdk_error_status(e)
    {
        tracing::warn!(status, "STS GetWebIdentityToken failed");
    }
    let response = result.boxed().context(WebIdentityTokenSnafu)?;
    // These STS operations only reach the success branch on HTTP 200; the
    // AWS SDK output type carries no status object to read it from directly.
    tracing::info!(status = 200u16, "STS GetWebIdentityToken response");

    response
        .web_identity_token()
        .map(|t| t.to_string())
        .context(WebIdentityTokenEmptySnafu)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Extract the raw HTTP status from an AWS SDK service error, for logging per
/// `ud-log-every-http-call-at-info`. Returns `None` for non-service-level
/// failures (construction, timeout, dispatch) that never reached the server
/// and therefore have no HTTP status to report.
fn sts_sdk_error_status<E>(err: &aws_sdk_sts::error::SdkError<E>) -> Option<u16> {
    match err {
        aws_sdk_sts::error::SdkError::ServiceError(e) => Some(e.raw().status().as_u16()),
        _ => None,
    }
}

/// Resolve final credentials: load ambient creds and optionally walk an
/// impersonation chain via `sts:AssumeRole`.
async fn resolve_credentials(
    config: &WorkloadIdentityConfig,
    region: &str,
) -> Result<Credentials, AwsAttestationError> {
    if config.impersonation_path.is_empty() {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .load()
            .await;
        let provider = sdk_config
            .credentials_provider()
            .context(NoCredentialsProviderSnafu)?;
        provider
            .provide_credentials()
            .await
            .boxed()
            .context(CredentialsLoadSnafu)
    } else {
        chain_assume_role(region, &config.impersonation_path).await
    }
}

/// Seam for a single `sts:AssumeRole` call. `aws_sdk_sts::Client` speaks
/// AWS's Query/XML protocol, which isn't practical to wiremock directly, so
/// production drives a real call ([`StsAssumeRoleProvider`]) while tests
/// drive a fake that returns canned credentials.
trait AssumeRoleProvider: Send + Sync {
    /// Assume `role_arn`. `credentials`, when present, is the previous
    /// role's temporary credentials, used instead of the ambient credential
    /// chain to authorize this call.
    fn assume_role<'a>(
        &'a self,
        role_arn: &'a str,
        credentials: Option<&'a Credentials>,
    ) -> BoxFuture<'a, Result<Credentials, AwsAttestationError>>;
}

/// Production [`AssumeRoleProvider`]: issues a real `sts:AssumeRole` call
/// via `aws_sdk_sts::Client`.
struct StsAssumeRoleProvider {
    region: String,
}

impl AssumeRoleProvider for StsAssumeRoleProvider {
    fn assume_role<'a>(
        &'a self,
        role_arn: &'a str,
        credentials: Option<&'a Credentials>,
    ) -> BoxFuture<'a, Result<Credentials, AwsAttestationError>> {
        async move {
            let mut loader = aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(self.region.clone()));
            if let Some(creds) = credentials {
                loader = loader.credentials_provider(SharedCredentialsProvider::new(creds.clone()));
            }
            let sdk_config = loader.load().await;
            let client = StsClient::new(&sdk_config);

            let session_name = format!("snowflake-wif-{}", std::process::id());
            tracing::info!(role_arn = %role_arn, "STS AssumeRole request");
            let result = client
                .assume_role()
                .role_arn(role_arn)
                .role_session_name(&session_name)
                .send()
                .await;
            if let Err(ref e) = result
                && let Some(status) = sts_sdk_error_status(e)
            {
                tracing::warn!(role_arn = %role_arn, status, "STS AssumeRole failed");
            }
            let resp = result.boxed().context(AssumeRoleSnafu {
                role_arn: role_arn.to_string(),
            })?;
            // This STS operation only reaches the success branch on HTTP 200;
            // the AWS SDK output type carries no status object to read it from
            // directly. `chain_assume_role_via` calls this once per hop in the
            // impersonation path, so every hop gets its own request/response
            // log pair for free.
            tracing::info!(role_arn = %role_arn, status = 200u16, "STS AssumeRole response");

            let raw_creds = resp
                .credentials()
                .context(AssumeRoleMissingCredentialsSnafu {
                    role_arn: role_arn.to_string(),
                })?;

            Ok(Credentials::new(
                raw_creds.access_key_id(),
                raw_creds.secret_access_key(),
                Some(raw_creds.session_token().to_string()),
                None,
                "snowflake-wif-assume-role",
            ))
        }
        .boxed()
    }
}

/// Walk an impersonation chain via `sts:AssumeRole`, returning the
/// credentials obtained after assuming all roles in `region`.
async fn chain_assume_role(
    region: &str,
    role_arns: &[String],
) -> Result<Credentials, AwsAttestationError> {
    let provider = StsAssumeRoleProvider {
        region: region.to_string(),
    };
    chain_assume_role_via(&provider, role_arns).await
}

/// Provider-agnostic core of [`chain_assume_role`]: calls `sts:AssumeRole`
/// once per entry in `role_arns`, in order, threading each hop's returned
/// credentials into the next hop's call. The first hop uses the ambient
/// credential chain (`credentials: None`).
async fn chain_assume_role_via(
    provider: &dyn AssumeRoleProvider,
    role_arns: &[String],
) -> Result<Credentials, AwsAttestationError> {
    let mut current_credentials: Option<Credentials> = None;

    for role_arn in role_arns {
        let creds = provider
            .assume_role(role_arn, current_credentials.as_ref())
            .await?;
        current_credentials = Some(creds);
    }

    current_credentials.context(ImpersonationChainEmptySnafu)
}

/// Resolve the AWS region.
///
/// Priority: `AWS_REGION` → `AWS_DEFAULT_REGION` → IMDS → `us-east-1`.
async fn resolve_region(endpoints: &AttestationEndpoints) -> String {
    if let Ok(r) = std::env::var("AWS_REGION")
        && !r.is_empty()
    {
        return r;
    }
    if let Ok(r) = std::env::var("AWS_DEFAULT_REGION")
        && !r.is_empty()
    {
        return r;
    }
    if let Some(r) = try_imds_region(&endpoints.aws_imds_base_url).await {
        return r;
    }
    "us-east-1".to_string()
}

async fn try_imds_region(imds_base_url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()?;

    let token_url = format!("{imds_base_url}/latest/api/token");
    let token_parsed = reqwest::Url::parse(&token_url).ok();
    tracing::info!(
        method = "PUT",
        host = token_parsed
            .as_ref()
            .and_then(|u| u.host_str())
            .unwrap_or("<none>"),
        path = token_parsed.as_ref().map_or("", |u| u.path()),
        "outbound HTTP call"
    );
    let token_resp = client
        .put(token_url)
        .header("X-aws-ec2-metadata-token-ttl-seconds", "60")
        .send()
        .await
        .ok()?;
    tracing::info!(status = token_resp.status().as_u16(), "HTTP response");
    let token = token_resp.text().await.ok()?;

    let region_url = format!("{imds_base_url}/latest/meta-data/placement/region");
    let region_parsed = reqwest::Url::parse(&region_url).ok();
    tracing::info!(
        method = "GET",
        host = region_parsed
            .as_ref()
            .and_then(|u| u.host_str())
            .unwrap_or("<none>"),
        path = region_parsed.as_ref().map_or("", |u| u.path()),
        "outbound HTTP call"
    );
    let region_resp = client
        .get(region_url)
        .header("X-aws-ec2-metadata-token", &token)
        .send()
        .await
        .ok()?;
    tracing::info!(status = region_resp.status().as_u16(), "HTTP response");

    if region_resp.status().is_success() {
        region_resp.text().await.ok()
    } else {
        None
    }
}

fn sts_hostname(region: &str) -> String {
    if region.starts_with("cn-") {
        format!("sts.{region}.amazonaws.com.cn")
    } else {
        format!("sts.{region}.amazonaws.com")
    }
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, AwsAttestationError> {
    let mut mac = HmacSha256::new_from_slice(key).context(HmacInitSnafu)?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn derive_signing_key(
    secret_access_key: &str,
    date_stamp: &str,
    region: &str,
    service: &str,
) -> Result<Vec<u8>, AwsAttestationError> {
    let k_secret = format!("AWS4{secret_access_key}");
    let k_date = hmac_sha256(k_secret.as_bytes(), date_stamp.as_bytes())?;
    let k_region = hmac_sha256(&k_date, region.as_bytes())?;
    let k_service = hmac_sha256(&k_region, service.as_bytes())?;
    hmac_sha256(&k_service, b"aws4_request")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::WifProvider;
    use std::sync::Mutex;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// `resolve_region` returns the region from the EC2 IMDS response.
    #[tokio::test]
    async fn resolve_region_reads_region_from_imds() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/latest/api/token"))
            .and(header("X-aws-ec2-metadata-token-ttl-seconds", "60"))
            .respond_with(ResponseTemplate::new(200).set_body_string("imds-token"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/latest/meta-data/placement/region"))
            .and(header("X-aws-ec2-metadata-token", "imds-token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("us-west-2"))
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            aws_imds_base_url: server.uri(),
            ..Default::default()
        };

        temp_env::async_with_vars(
            [
                ("AWS_REGION", None::<&str>),
                ("AWS_DEFAULT_REGION", None::<&str>),
            ],
            async {
                let region = resolve_region(&endpoints).await;
                assert_eq!(region, "us-west-2");
            },
        )
        .await;
    }

    /// Proves the two IMDS region-resolution calls are logged at INFO per
    /// `ud-log-every-http-call-at-info`: a dispatch log carrying method + path
    /// before each call and a response-status log after each.
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn imds_region_calls_are_logged_at_info() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/latest/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("imds-token"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/latest/meta-data/placement/region"))
            .respond_with(ResponseTemplate::new(200).set_body_string("us-west-2"))
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            aws_imds_base_url: server.uri(),
            ..Default::default()
        };

        temp_env::async_with_vars(
            [
                ("AWS_REGION", None::<&str>),
                ("AWS_DEFAULT_REGION", None::<&str>),
            ],
            async {
                let region = resolve_region(&endpoints).await;
                assert_eq!(region, "us-west-2");
            },
        )
        .await;

        assert!(logs_contain("outbound HTTP call"), "dispatch log missing");
        let expected_host = reqwest::Url::parse(&server.uri())
            .unwrap()
            .host_str()
            .unwrap()
            .to_owned();
        assert!(
            logs_contain(&expected_host),
            "host not logged on dispatch line"
        );
        assert!(logs_contain("/latest/api/token"), "token path not logged");
        assert!(
            logs_contain("/latest/meta-data/placement/region"),
            "region path not logged"
        );
        assert!(logs_contain("HTTP response"), "response log missing");
        assert!(logs_contain("status=200"), "response status not logged");
    }

    /// `AWS_REGION` must win without ever falling through to IMDS. IMDS is
    /// pointed at an address nothing listens on, so if `resolve_region`
    /// consulted it anyway this test would have to wait out a connection
    /// failure instead of resolving immediately.
    #[tokio::test]
    async fn resolve_region_prefers_aws_region_env_var_over_imds() {
        let endpoints = AttestationEndpoints {
            aws_imds_base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };

        temp_env::async_with_vars(
            [
                ("AWS_REGION", Some("eu-west-1")),
                ("AWS_DEFAULT_REGION", None::<&str>),
            ],
            async {
                let region = resolve_region(&endpoints).await;
                assert_eq!(region, "eu-west-1");
            },
        )
        .await;
    }

    /// `AWS_DEFAULT_REGION` is the second-priority env var: it must be used
    /// when `AWS_REGION` is unset, again without falling through to IMDS.
    #[tokio::test]
    async fn resolve_region_falls_back_to_aws_default_region_when_aws_region_unset() {
        let endpoints = AttestationEndpoints {
            aws_imds_base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };

        temp_env::async_with_vars(
            [
                ("AWS_REGION", None::<&str>),
                ("AWS_DEFAULT_REGION", Some("ap-southeast-2")),
            ],
            async {
                let region = resolve_region(&endpoints).await;
                assert_eq!(region, "ap-southeast-2");
            },
        )
        .await;
    }

    /// When both env vars are set, `AWS_REGION` takes precedence over
    /// `AWS_DEFAULT_REGION` -- this is part of the documented contract in
    /// this module's header comment, not just "does either one work".
    #[tokio::test]
    async fn resolve_region_prefers_aws_region_over_aws_default_region_when_both_set() {
        let endpoints = AttestationEndpoints {
            aws_imds_base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };

        temp_env::async_with_vars(
            [
                ("AWS_REGION", Some("us-west-2")),
                ("AWS_DEFAULT_REGION", Some("eu-central-1")),
            ],
            async {
                let region = resolve_region(&endpoints).await;
                assert_eq!(
                    region, "us-west-2",
                    "AWS_REGION must take precedence over AWS_DEFAULT_REGION"
                );
            },
        )
        .await;
    }

    /// Final fallback: when neither env var is set and IMDS is unreachable,
    /// `resolve_region` must still return a usable region (`us-east-1`)
    /// instead of propagating the IMDS connection failure.
    #[tokio::test]
    async fn resolve_region_falls_back_to_us_east_1_when_no_env_var_and_imds_unreachable() {
        let endpoints = AttestationEndpoints {
            aws_imds_base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };

        temp_env::async_with_vars(
            [
                ("AWS_REGION", None::<&str>),
                ("AWS_DEFAULT_REGION", None::<&str>),
            ],
            async {
                let region = resolve_region(&endpoints).await;
                assert_eq!(region, "us-east-1");
            },
        )
        .await;
    }

    /// Legacy: `test_explicit_aws_no_auth_raises_error` asserts
    /// `"No AWS credentials were found"` when nothing in boto3's credential
    /// chain resolves. UD's message text differs (`NoCredentialsProvider`
    /// says "No AWS credentials provider configured";
    /// `CredentialsLoad` wraps whatever the AWS SDK reports) — this test
    /// asserts on the error discriminant rather than message text, since
    /// message-text parity wasn't part of this change's scope and either
    /// variant is a legitimate "no credentials" outcome depending on
    /// exactly where the SDK's own provider chain gives up.
    ///
    /// Points every provider in the default chain (env vars, shared
    /// config/credentials files, container/web-identity-token providers,
    /// IMDS) at nothing, so this resolves deterministically offline instead
    /// of depending on the host's ambient AWS environment.
    #[tokio::test]
    async fn resolve_credentials_with_no_ambient_credentials_returns_clear_error() {
        let config = WorkloadIdentityConfig {
            provider: WifProvider::Aws,
            entra_resource: None,
            impersonation_path: vec![],
            oidc_token: None,
        };

        temp_env::async_with_vars(
            [
                ("AWS_ACCESS_KEY_ID", None::<&str>),
                ("AWS_SECRET_ACCESS_KEY", None::<&str>),
                ("AWS_SESSION_TOKEN", None::<&str>),
                ("AWS_PROFILE", None::<&str>),
                (
                    "AWS_SHARED_CREDENTIALS_FILE",
                    Some("/nonexistent/aws-credentials-for-tests"),
                ),
                ("AWS_CONFIG_FILE", Some("/nonexistent/aws-config-for-tests")),
                ("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI", None::<&str>),
                ("AWS_CONTAINER_CREDENTIALS_FULL_URI", None::<&str>),
                ("AWS_WEB_IDENTITY_TOKEN_FILE", None::<&str>),
                ("AWS_EC2_METADATA_DISABLED", Some("true")),
            ],
            async {
                let err = resolve_credentials(&config, "us-east-1")
                    .await
                    .expect_err("expected no ambient AWS credentials to be found");
                assert!(
                    matches!(
                        err,
                        AwsAttestationError::NoCredentialsProvider { .. }
                            | AwsAttestationError::CredentialsLoad { .. }
                    ),
                    "expected a credentials-not-found error, got {err:?}"
                );
            },
        )
        .await;
    }

    /// Legacy: `test_aws_token_format_based_on_env_variable` verifies AWS
    /// dispatches on `SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN`. UD's
    /// dispatch (`get_attestation_token`'s `if enable_outbound_token() {
    /// get_web_identity_token } else { get_caller_identity_token }`) delays
    /// to the AWS SDK once outbound is selected, so this tests the pure,
    /// directly-unit-testable dispatch decision rather than driving both
    /// downstream paths end-to-end.
    #[test]
    fn enable_outbound_token_defaults_to_false_when_unset() {
        temp_env::with_var(
            "SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN",
            None::<&str>,
            || {
                assert!(!enable_outbound_token());
            },
        );
    }

    #[test]
    fn enable_outbound_token_true_variants() {
        for value in ["true", "TRUE", "True", "1"] {
            temp_env::with_var(
                "SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN",
                Some(value),
                || {
                    assert!(
                        enable_outbound_token(),
                        "{value:?} should enable outbound token"
                    );
                },
            );
        }
    }

    #[test]
    fn enable_outbound_token_false_variants() {
        for value in ["false", "FALSE", "0", "yes", ""] {
            temp_env::with_var(
                "SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN",
                Some(value),
                || {
                    assert!(
                        !enable_outbound_token(),
                        "{value:?} should not enable outbound token"
                    );
                },
            );
        }
    }

    /// Records every `assume_role` call this fake receives -- role ARN and
    /// the access-key-id of whatever credentials it was called with (`None`
    /// for the ambient chain) -- and returns a distinct canned credential
    /// per call so tests can prove hop *N+1* used hop *N*'s output.
    #[derive(Default)]
    struct FakeAssumeRoleProvider {
        calls: Mutex<Vec<(String, Option<String>)>>,
    }

    impl AssumeRoleProvider for FakeAssumeRoleProvider {
        fn assume_role<'a>(
            &'a self,
            role_arn: &'a str,
            credentials: Option<&'a Credentials>,
        ) -> BoxFuture<'a, Result<Credentials, AwsAttestationError>> {
            let used_access_key_id = credentials.map(|c| c.access_key_id().to_string());
            async move {
                let call_index = {
                    let mut calls = self.calls.lock().unwrap();
                    calls.push((role_arn.to_string(), used_access_key_id));
                    calls.len()
                };
                Ok(Credentials::new(
                    format!("access-key-{call_index}"),
                    format!("secret-key-{call_index}"),
                    Some(format!("session-token-{call_index}")),
                    None,
                    "fake-assume-role",
                ))
            }
            .boxed()
        }
    }

    /// Legacy: `test_aws_impersonation_calls_correct_apis_for_each_role_in_impersonation_path`
    /// asserts `sts:AssumeRole` is called once per role via
    /// `assume_role_call_count`, in order (its `FakeAwsEnvironment.assume_role`
    /// only advances the counter when the `RoleArn` matches the expected next
    /// hop). This test additionally asserts *which* credentials each hop
    /// used, proving the chaining behavior the docstring on
    /// `chain_assume_role_via` claims: hop 1 uses the ambient chain, every
    /// later hop uses the previous hop's returned credentials.
    #[tokio::test]
    async fn chain_assume_role_calls_once_per_role_in_order_using_previous_credentials() {
        let provider = FakeAssumeRoleProvider::default();
        let role_arns = vec![
            "arn:aws:iam::123456789:role/role2".to_string(),
            "arn:aws:iam::123456789:role/role3".to_string(),
        ];

        let final_credentials = chain_assume_role_via(&provider, &role_arns).await.unwrap();

        let calls = provider.calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            2,
            "expected exactly one AssumeRole call per role in the impersonation path"
        );
        assert_eq!(calls[0].0, "arn:aws:iam::123456789:role/role2");
        assert_eq!(calls[1].0, "arn:aws:iam::123456789:role/role3");

        // First hop authorizes via the ambient credential chain.
        assert_eq!(calls[0].1, None);
        // Second hop must use the first hop's returned credentials.
        assert_eq!(calls[1].1, Some("access-key-1".to_string()));

        // The final result is whatever the last hop returned.
        assert_eq!(final_credentials.access_key_id(), "access-key-2");
    }

    #[tokio::test]
    async fn chain_assume_role_with_single_role_uses_ambient_credentials() {
        let provider = FakeAssumeRoleProvider::default();
        let role_arns = vec!["arn:aws:iam::123456789:role/only-role".to_string()];

        chain_assume_role_via(&provider, &role_arns).await.unwrap();

        let calls = provider.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, None);
    }

    #[tokio::test]
    async fn chain_assume_role_with_empty_path_returns_impersonation_chain_empty_error() {
        let provider = FakeAssumeRoleProvider::default();
        let err = chain_assume_role_via(&provider, &[]).await.unwrap_err();
        assert!(
            matches!(err, AwsAttestationError::ImpersonationChainEmpty { .. }),
            "expected ImpersonationChainEmpty, got: {err:?}"
        );
    }

    /// Which [`AwsAttestationError`] variant [`FailingAssumeRoleProvider`]
    /// synthesizes once it reaches its configured failing hop.
    #[derive(Clone, Copy)]
    enum FakeAssumeRoleFailure {
        /// The `sts:AssumeRole` call itself fails.
        Call,
        /// The call succeeds but STS returns no credentials.
        MissingCredentials,
    }

    /// Like [`FakeAssumeRoleProvider`], but fails at a configurable
    /// 1-indexed call number instead of always succeeding. Proves
    /// `chain_assume_role_via` stops at the failing hop -- later hops are
    /// never attempted -- and propagates that hop's error variant
    /// unchanged rather than continuing on or rewrapping it.
    struct FailingAssumeRoleProvider {
        fail_at_call: usize,
        failure: FakeAssumeRoleFailure,
        calls: Mutex<Vec<(String, Option<String>)>>,
    }

    impl FailingAssumeRoleProvider {
        fn new(fail_at_call: usize, failure: FakeAssumeRoleFailure) -> Self {
            Self {
                fail_at_call,
                failure,
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl AssumeRoleProvider for FailingAssumeRoleProvider {
        fn assume_role<'a>(
            &'a self,
            role_arn: &'a str,
            credentials: Option<&'a Credentials>,
        ) -> BoxFuture<'a, Result<Credentials, AwsAttestationError>> {
            let used_access_key_id = credentials.map(|c| c.access_key_id().to_string());
            async move {
                let call_index = {
                    let mut calls = self.calls.lock().unwrap();
                    calls.push((role_arn.to_string(), used_access_key_id));
                    calls.len()
                };
                if call_index == self.fail_at_call {
                    return match self.failure {
                        FakeAssumeRoleFailure::Call => {
                            let synthetic: Result<Credentials, std::io::Error> =
                                Err(std::io::Error::other("simulated STS AssumeRole failure"));
                            synthetic.boxed().context(AssumeRoleSnafu {
                                role_arn: role_arn.to_string(),
                            })
                        }
                        FakeAssumeRoleFailure::MissingCredentials => {
                            AssumeRoleMissingCredentialsSnafu {
                                role_arn: role_arn.to_string(),
                            }
                            .fail()
                        }
                    };
                }
                Ok(Credentials::new(
                    format!("access-key-{call_index}"),
                    format!("secret-key-{call_index}"),
                    Some(format!("session-token-{call_index}")),
                    None,
                    "fake-assume-role",
                ))
            }
            .boxed()
        }
    }

    /// Proves that when a middle hop's `sts:AssumeRole` call itself fails,
    /// `chain_assume_role_via` stops immediately: the hop after the failing
    /// one is never attempted, and the error that comes back is the
    /// `AssumeRole` variant for the failing role, not a generic error or
    /// the wrong role's ARN.
    #[tokio::test]
    async fn chain_assume_role_via_stops_at_failing_hop_and_propagates_assume_role_error() {
        let provider = FailingAssumeRoleProvider::new(2, FakeAssumeRoleFailure::Call);
        let role_arns = vec![
            "arn:aws:iam::123456789:role/role1".to_string(),
            "arn:aws:iam::123456789:role/role2".to_string(),
            "arn:aws:iam::123456789:role/role3".to_string(),
        ];

        let err = chain_assume_role_via(&provider, &role_arns)
            .await
            .unwrap_err();

        match err {
            AwsAttestationError::AssumeRole { role_arn, .. } => {
                assert_eq!(role_arn, "arn:aws:iam::123456789:role/role2");
            }
            other => panic!("expected AssumeRole error, got {other:?}"),
        }

        let calls = provider.calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            2,
            "the third hop must never be attempted once the second hop fails"
        );
        assert_eq!(calls[0].0, "arn:aws:iam::123456789:role/role1");
        assert_eq!(calls[1].0, "arn:aws:iam::123456789:role/role2");
    }

    /// Same shape as the previous test, but for the other failure mode a
    /// hop can produce: STS accepts the call but returns no credentials.
    /// Proves that error propagates as `AssumeRoleMissingCredentials`
    /// (not `AssumeRole` or some other variant) and that the chain still
    /// stops at the failing hop.
    #[tokio::test]
    async fn chain_assume_role_via_stops_at_failing_hop_and_propagates_missing_credentials_error() {
        let provider = FailingAssumeRoleProvider::new(1, FakeAssumeRoleFailure::MissingCredentials);
        let role_arns = vec![
            "arn:aws:iam::123456789:role/role1".to_string(),
            "arn:aws:iam::123456789:role/role2".to_string(),
        ];

        let err = chain_assume_role_via(&provider, &role_arns)
            .await
            .unwrap_err();

        match err {
            AwsAttestationError::AssumeRoleMissingCredentials { role_arn, .. } => {
                assert_eq!(role_arn, "arn:aws:iam::123456789:role/role1");
            }
            other => panic!("expected AssumeRoleMissingCredentials error, got {other:?}"),
        }

        let calls = provider.calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            1,
            "the second hop must never be attempted once the first hop fails"
        );
    }

    #[test]
    fn sts_hostname_china() {
        assert_eq!(
            sts_hostname("cn-northwest-1"),
            "sts.cn-northwest-1.amazonaws.com.cn"
        );
    }

    #[test]
    fn sts_hostname_standard() {
        assert_eq!(sts_hostname("us-east-1"), "sts.us-east-1.amazonaws.com");
    }

    /// Expands hostname coverage beyond china + one standard region: legacy's
    /// `test_get_aws_sts_hostname_valid_inputs` (8-case matrix, parameterized
    /// on a `partition` argument that UD's `sts_hostname` doesn't have, since
    /// it derives the TLD purely from the region prefix). GovCloud regions in
    /// particular use the standard `.amazonaws.com` suffix, not a distinct
    /// partition-specific one, so this also proves `sts_hostname` handles
    /// them correctly with no `us-gov-` special case.
    #[test]
    fn sts_hostname_covers_govcloud_and_additional_standard_regions() {
        let cases = [
            ("us-gov-west-1", "sts.us-gov-west-1.amazonaws.com"),
            ("us-gov-east-1", "sts.us-gov-east-1.amazonaws.com"),
            ("af-south-1", "sts.af-south-1.amazonaws.com"),
            ("eu-central-1", "sts.eu-central-1.amazonaws.com"),
            ("ap-southeast-2", "sts.ap-southeast-2.amazonaws.com"),
            ("cn-north-1", "sts.cn-north-1.amazonaws.com.cn"),
        ];
        for (region, expected) in cases {
            assert_eq!(sts_hostname(region), expected, "region {region}");
        }
    }

    #[test]
    fn caller_identity_token_structure() {
        let request = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            Some("session-token"),
            "us-east-1",
            "20240101T120000Z",
            "20240101",
        )
        .unwrap();

        let json = serde_json::to_string(&request).unwrap();
        let decoded: serde_json::Value = serde_json::from_str(&json).unwrap();

        let url = decoded["url"].as_str().unwrap();
        assert!(url.starts_with("https://sts.us-east-1.amazonaws.com/"));
        assert!(url.contains("Action=GetCallerIdentity"));
        assert!(url.contains("Version=2011-06-15"));
        assert_eq!(decoded["method"].as_str().unwrap(), "POST");

        let headers = &decoded["headers"];
        assert_eq!(
            headers["Host"].as_str().unwrap(),
            "sts.us-east-1.amazonaws.com"
        );
        assert_eq!(
            headers["X-Snowflake-Audience"].as_str().unwrap(),
            "snowflakecomputing.com"
        );
        assert_eq!(headers["X-Amz-Date"].as_str().unwrap(), "20240101T120000Z");
        assert_eq!(
            headers["X-Amz-Security-Token"].as_str().unwrap(),
            "session-token"
        );
        assert!(
            headers["Authorization"]
                .as_str()
                .unwrap()
                .starts_with("AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/"),
            "authorization header should start with AWS4-HMAC-SHA256"
        );
    }

    #[test]
    fn caller_identity_token_without_session_token() {
        let request = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            None,
            "eu-west-1",
            "20240101T120000Z",
            "20240101",
        )
        .unwrap();

        let json = serde_json::to_string(&request).unwrap();
        let decoded: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(decoded["headers"].get("X-Amz-Security-Token").is_none());
        assert!(
            decoded["headers"]["Authorization"]
                .as_str()
                .unwrap()
                .contains("SignedHeaders=host;x-amz-date;x-snowflake-audience"),
        );
    }

    #[test]
    fn caller_identity_token_is_base64_encoded_json() {
        let request = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "secret",
            None,
            "us-west-2",
            "20240101T120000Z",
            "20240101",
        )
        .unwrap();
        let json = serde_json::to_string(&request).unwrap();
        let encoded = BASE64.encode(json.as_bytes());

        // Must decode back to valid JSON
        let decoded_bytes = BASE64.decode(&encoded).unwrap();
        let decoded: serde_json::Value = serde_json::from_slice(&decoded_bytes).unwrap();
        assert_eq!(decoded["method"].as_str().unwrap(), "POST");
    }
}
