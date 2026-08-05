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
        .region(Region::new(region))
        .load()
        .await;

    let sts_client = StsClient::new(&sdk_config);

    let credentials = if config.impersonation_path.is_empty() {
        None
    } else {
        Some(chain_assume_role(&sts_client, &config.impersonation_path).await?)
    };

    let final_sts_client = if let Some(creds) = credentials {
        let final_config = StsConfigBuilder::from(&sdk_config)
            .credentials_provider(SharedCredentialsProvider::new(creds))
            .build();
        StsClient::from_conf(final_config)
    } else {
        sts_client
    };

    let response = final_sts_client
        .get_web_identity_token()
        .audience(SNOWFLAKE_AUDIENCE)
        .signing_algorithm(AWS_WIF_SIGNING_ALGORITHM)
        .send()
        .await
        .boxed()
        .context(WebIdentityTokenSnafu)?;

    response
        .web_identity_token()
        .map(|t| t.to_string())
        .context(WebIdentityTokenEmptySnafu)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Resolve final credentials: load ambient creds and optionally walk an
/// impersonation chain via `sts:AssumeRole`.
async fn resolve_credentials(
    config: &WorkloadIdentityConfig,
    region: &str,
) -> Result<Credentials, AwsAttestationError> {
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region.to_string()))
        .load()
        .await;

    if config.impersonation_path.is_empty() {
        let provider = sdk_config
            .credentials_provider()
            .context(NoCredentialsProviderSnafu)?;
        provider
            .provide_credentials()
            .await
            .boxed()
            .context(CredentialsLoadSnafu)
    } else {
        let sts_client = StsClient::new(&sdk_config);
        chain_assume_role(&sts_client, &config.impersonation_path).await
    }
}

/// Walk an impersonation chain via `sts:AssumeRole`, returning the credentials
/// obtained after assuming all roles.
async fn chain_assume_role(
    initial_client: &StsClient,
    role_arns: &[String],
) -> Result<Credentials, AwsAttestationError> {
    let mut current_client = initial_client.clone();
    let mut current_credentials: Option<Credentials> = None;

    for role_arn in role_arns {
        let client_to_use = if let Some(ref creds) = current_credentials {
            let sdk_config = aws_config::defaults(BehaviorVersion::latest())
                .credentials_provider(SharedCredentialsProvider::new(creds.clone()))
                .load()
                .await;
            StsClient::new(&sdk_config)
        } else {
            current_client.clone()
        };

        let session_name = format!("snowflake-wif-{}", std::process::id());
        let resp = client_to_use
            .assume_role()
            .role_arn(role_arn)
            .role_session_name(&session_name)
            .send()
            .await
            .boxed()
            .context(AssumeRoleSnafu {
                role_arn: role_arn.clone(),
            })?;

        let raw_creds = resp
            .credentials()
            .context(AssumeRoleMissingCredentialsSnafu {
                role_arn: role_arn.clone(),
            })?;

        current_credentials = Some(Credentials::new(
            raw_creds.access_key_id(),
            raw_creds.secret_access_key(),
            Some(raw_creds.session_token().to_string()),
            None,
            "snowflake-wif-assume-role",
        ));
        current_client = client_to_use;
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

    let token_resp = client
        .put(format!("{imds_base_url}/latest/api/token"))
        .header("X-aws-ec2-metadata-token-ttl-seconds", "60")
        .send()
        .await
        .ok()?;
    let token = token_resp.text().await.ok()?;

    let region_resp = client
        .get(format!("{imds_base_url}/latest/meta-data/placement/region"))
        .header("X-aws-ec2-metadata-token", &token)
        .send()
        .await
        .ok()?;

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
