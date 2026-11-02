//! AWS attestation for Workload Identity Federation.
//!
//! **Default** — pre-signed STS `GetCallerIdentity` request encoded as base64
//! JSON `{"url":…,"method":"POST","headers":{…}}`.  Snowflake GS replays this
//! request server-side to verify the caller's identity.  No outbound STS call
//! is made by the driver.
//!
//! **Opt-in** — when `workload_identity_aws_use_outbound_token=true` or
//! `SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN=true` the driver
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
use aws_lc_rs::{digest, hmac};
use aws_sdk_sts::config::SharedCredentialsProvider;
use aws_sdk_sts::config::endpoint::{
    DefaultResolver, Params as StsEndpointParams, ResolveEndpoint,
};
use aws_sdk_sts::{Client as StsClient, config::Builder as StsConfigBuilder};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use futures::FutureExt as _;
use futures::future::BoxFuture;
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::collections::BTreeMap;

use crate::config::rest_parameters::WorkloadIdentityConfig;
use crate::tls::aws_http_client::AwsSdkReqwestClient;

use super::AttestationEndpoints;

const SNOWFLAKE_AUDIENCE: &str = "snowflakecomputing.com";
const AWS_WIF_SIGNING_ALGORITHM: &str = "ES384";
const EMPTY_BODY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

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
    #[snafu(display("Failed to build the AWS SDK HTTP client"))]
    SdkHttpClient {
        #[snafu(source(from(crate::tls::error::TlsError, Box::new)))]
        source: Box<crate::tls::error::TlsError>,
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
    #[snafu(display("invalid wif_host '{host}'"))]
    InvalidWifHost {
        host: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("could not resolve an STS endpoint for region '{region}'"))]
    StsEndpointResolve {
        region: String,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Primary AWS WIF entry point.
///
/// Dispatches to the pre-signed `GetCallerIdentity` path (default) or the
/// outbound `GetWebIdentityToken` path when
/// `workload_identity_aws_use_outbound_token=true` or
/// `SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN=true`.
pub(super) async fn get_attestation_token(
    sdk_http: &AwsSdkReqwestClient,
    config: &WorkloadIdentityConfig,
    endpoints: &AttestationEndpoints,
) -> Result<String, AwsAttestationError> {
    if enable_outbound_token(config) {
        get_web_identity_token(sdk_http, config, endpoints).await
    } else {
        get_caller_identity_token(sdk_http, config, endpoints).await
    }
}

/// Returns `true` when the caller explicitly opts into the outbound-token path.
fn enable_outbound_token(config: &WorkloadIdentityConfig) -> bool {
    if config.aws_use_outbound_token {
        return true;
    }
    crate::utils::env_flag("SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN")
}

// ---------------------------------------------------------------------------
// Pre-signed GetCallerIdentity (default)
// ---------------------------------------------------------------------------

/// Build a pre-signed STS `GetCallerIdentity` request and return it
/// base64-encoded as `{"url":…,"method":"POST","headers":{…}}`.
async fn get_caller_identity_token(
    sdk_http: &AwsSdkReqwestClient,
    config: &WorkloadIdentityConfig,
    endpoints: &AttestationEndpoints,
) -> Result<String, AwsAttestationError> {
    let region = resolve_region(endpoints).await;
    let credentials = resolve_credentials(sdk_http, config, &region).await?;

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
        config.wif_host.as_deref(),
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
    wif_host: Option<&str>,
) -> Result<SignedCallerIdentityRequest, AwsAttestationError> {
    let endpoint = aws_sts_endpoint_for(region, wif_host)?;
    let host = endpoint.authority;
    let (scheme, after_scheme) = endpoint
        .base_url
        .split_once("://")
        .context(StsEndpointResolveSnafu { region })?;
    let canonical_uri = after_scheme
        .strip_prefix(&host)
        .filter(|path| !path.is_empty())
        .unwrap_or("/");
    let url =
        format!("{scheme}://{host}{canonical_uri}?Action=GetCallerIdentity&Version=2011-06-15");

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

    let canonical_request = format!(
        "POST\n{canonical_uri}\nAction=GetCallerIdentity&Version=2011-06-15\n{canonical_headers}\n{signed_headers}\n{EMPTY_BODY_HASH}"
    );

    let credential_scope = format!("{date_stamp}/{region}/sts/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );

    let signing_key = derive_signing_key(secret_access_key, date_stamp, region, "sts");

    let signature =
        hex::encode(hmac_sha256(signing_key.as_ref(), string_to_sign.as_bytes()).as_ref());

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
    sdk_http: &AwsSdkReqwestClient,
    config: &WorkloadIdentityConfig,
    endpoints: &AttestationEndpoints,
) -> Result<String, AwsAttestationError> {
    let region = resolve_region(endpoints).await;
    let sts_endpoint = aws_sts_endpoint_for(&region, config.wif_host.as_deref())?;
    let sdk_config =
        sdk_config_with_shared_transport(sdk_http, &region, None, Some(&sts_endpoint)).await;

    let sts_client = StsClient::new(&sdk_config);

    let credentials = if config.impersonation_path.is_empty() {
        None
    } else {
        Some(chain_assume_role(sdk_http, &region, &config.impersonation_path, &sts_endpoint).await?)
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

/// Builds an AWS SDK config whose HTTP client is the driver's shared reqwest
/// transport, the same adapter S3 transfers use.
///
/// The AWS SDK's default HTTP client is disabled at compile time, so SDK calls
/// cannot be built without an explicit transport. Supplying the shared
/// transport keeps WIF on the connection's TLS and proxy policy and the same
/// crypto backend as the rest of the driver.
///
/// Takes [`AwsSdkReqwestClient`] rather than a bare `reqwest::Client` because
/// these are SigV4-signed SDK calls: the transport must not follow redirects,
/// auto-decompress, or negotiate HTTP/2 (see `tls::aws_http_client`), and the
/// newtype is what guarantees a general-purpose client cannot end up here.
///
/// When `sts_endpoint` is an override, it is pinned as `endpoint_url` and
/// FIPS/dualstack selection is cleared so they cannot reject the custom host.
/// A regional default is left to the SDK so ambient FIPS/dualstack still apply.
async fn sdk_config_with_shared_transport(
    sdk_http: &AwsSdkReqwestClient,
    region: &str,
    credentials: Option<&Credentials>,
    sts_endpoint: Option<&AwsStsEndpoint>,
) -> aws_config::SdkConfig {
    let mut loader = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region.to_string()))
        .http_client(crate::tls::aws_http_client::reqwest_aws_http_client(
            sdk_http.clone(),
        ));
    if let Some(creds) = credentials {
        loader = loader.credentials_provider(SharedCredentialsProvider::new(creds.clone()));
    }
    if let Some(endpoint) = sts_endpoint
        && endpoint.overridden
    {
        loader = loader
            .endpoint_url(&endpoint.base_url)
            .use_fips(false)
            .use_dual_stack(false);
    }
    loader.load().await
}

/// Resolve final credentials: load ambient creds and optionally walk an
/// impersonation chain via `sts:AssumeRole`.
async fn resolve_credentials(
    sdk_http: &AwsSdkReqwestClient,
    config: &WorkloadIdentityConfig,
    region: &str,
) -> Result<Credentials, AwsAttestationError> {
    if config.impersonation_path.is_empty() {
        let sdk_config = sdk_config_with_shared_transport(sdk_http, region, None, None).await;
        let provider = sdk_config
            .credentials_provider()
            .context(NoCredentialsProviderSnafu)?;
        provider
            .provide_credentials()
            .await
            .boxed()
            .context(CredentialsLoadSnafu)
    } else {
        let sts_endpoint = aws_sts_endpoint_for(region, config.wif_host.as_deref())?;
        chain_assume_role(sdk_http, region, &config.impersonation_path, &sts_endpoint).await
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
    sdk_http: AwsSdkReqwestClient,
    region: String,
    sts_endpoint: AwsStsEndpoint,
}

impl AssumeRoleProvider for StsAssumeRoleProvider {
    fn assume_role<'a>(
        &'a self,
        role_arn: &'a str,
        credentials: Option<&'a Credentials>,
    ) -> BoxFuture<'a, Result<Credentials, AwsAttestationError>> {
        async move {
            let sdk_config = sdk_config_with_shared_transport(
                &self.sdk_http,
                &self.region,
                credentials,
                Some(&self.sts_endpoint),
            )
            .await;
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
    sdk_http: &AwsSdkReqwestClient,
    region: &str,
    role_arns: &[String],
    sts_endpoint: &AwsStsEndpoint,
) -> Result<Credentials, AwsAttestationError> {
    let provider = StsAssumeRoleProvider {
        sdk_http: sdk_http.clone(),
        region: region.to_string(),
        sts_endpoint: sts_endpoint.clone(),
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
    crate::tls::ensure_crypto_provider();
    let client = reqwest::Client::builder()
        .tls_built_in_root_certs(false)
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct AwsStsEndpoint {
    authority: String,
    base_url: String,
    overridden: bool,
}

/// A blank or missing `wif_host` uses the regional SDK STS endpoint. Any other
/// value is parsed as an override and rejected when it is not a valid http(s) host.
fn aws_sts_endpoint_for(
    region: &str,
    wif_host: Option<&str>,
) -> Result<AwsStsEndpoint, AwsAttestationError> {
    if let Some(host) = wif_host {
        let host = host.trim();
        if !host.is_empty() {
            return parse_workload_identity_host(host);
        }
    }
    default_sts_endpoint(region)
}

fn parse_workload_identity_host(host: &str) -> Result<AwsStsEndpoint, AwsAttestationError> {
    let with_scheme = if host.contains("://") {
        host.to_string()
    } else {
        format!("https://{host}")
    };
    let parsed = url::Url::parse(&with_scheme)
        .ok()
        .context(InvalidWifHostSnafu { host: "" })?;
    let authority = authority_of(&parsed).unwrap_or_default();
    let path = parsed.path().trim_end_matches('/');
    let host = if parsed.cannot_be_a_base() || path.is_empty() {
        authority.clone()
    } else {
        format!("{authority}{path}")
    };
    if !wif_host_url_allowed(&parsed) || authority.is_empty() {
        return InvalidWifHostSnafu { host }.fail();
    }
    Ok(AwsStsEndpoint {
        base_url: base_url_of(&parsed, &authority),
        authority,
        overridden: true,
    })
}

// Taken from gosnowflake `parseWorkloadIdentityHost`.
fn wif_host_url_allowed(parsed: &url::Url) -> bool {
    matches!(parsed.scheme(), "http" | "https")
        && parsed.host().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none()
}

fn default_sts_endpoint(region: &str) -> Result<AwsStsEndpoint, AwsAttestationError> {
    let params = StsEndpointParams::builder()
        .region(region)
        .use_fips(false)
        .use_dual_stack(false)
        .use_global_endpoint(false)
        .build()
        .ok()
        .context(StsEndpointResolveSnafu { region })?;
    let endpoint = DefaultResolver::new()
        .resolve_endpoint(&params)
        .now_or_never()
        .and_then(Result::ok)
        .context(StsEndpointResolveSnafu { region })?;
    endpoint_from_url(endpoint.url(), region, false)
}

fn endpoint_from_url(
    url: &str,
    region: &str,
    overridden: bool,
) -> Result<AwsStsEndpoint, AwsAttestationError> {
    let parsed = url::Url::parse(url)
        .ok()
        .context(StsEndpointResolveSnafu { region })?;
    let authority = authority_of(&parsed).context(StsEndpointResolveSnafu { region })?;
    Ok(AwsStsEndpoint {
        base_url: base_url_of(&parsed, &authority),
        authority,
        overridden,
    })
}

fn authority_of(parsed: &url::Url) -> Option<String> {
    let host = match parsed.host()? {
        url::Host::Domain(domain) => domain.to_ascii_lowercase(),
        url::Host::Ipv4(ip) => ip.to_string(),
        url::Host::Ipv6(ip) => format!("[{ip}]"),
    };
    Some(match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    })
}

fn base_url_of(parsed: &url::Url, authority: &str) -> String {
    let mut base = format!("{}://{authority}", parsed.scheme());
    let path = parsed.path().trim_end_matches('/');
    if !path.is_empty() {
        base.push_str(path);
    }
    base
}

// Keep local WIF signing on AWS-LC so fips uses its FIPS module for both
// the canonical request hash and HMAC; SDK-managed SigV4 is a separate path.
fn sha256_hex(data: &[u8]) -> String {
    hex::encode(digest::digest(&digest::SHA256, data).as_ref())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> hmac::Tag {
    hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, key), data)
}

fn derive_signing_key(
    secret_access_key: &str,
    date_stamp: &str,
    region: &str,
    service: &str,
) -> hmac::Tag {
    let k_secret = format!("AWS4{secret_access_key}");
    let k_date = hmac_sha256(k_secret.as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(k_date.as_ref(), region.as_bytes());
    let k_service = hmac_sha256(k_region.as_ref(), service.as_bytes());
    hmac_sha256(k_service.as_ref(), b"aws4_request")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::WifProvider;
    use std::sync::Mutex;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn proxy_content_length(head: &[u8]) -> usize {
        std::str::from_utf8(head)
            .unwrap_or("")
            .lines()
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().ok())
                        .flatten()
                })
            })
            .unwrap_or(0)
    }

    async fn start_recording_proxy(
        sts_request_prefix: &[u8],
    ) -> (
        std::net::SocketAddr,
        tokio::sync::oneshot::Receiver<Vec<u8>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind proxy listener");
        let addr = listener.local_addr().expect("proxy address");
        let sts_request_prefix = sts_request_prefix.to_vec();
        let (request_tx, request_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let mut request_tx = Some(request_tx);
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let mut head = Vec::with_capacity(512);
                let mut byte = [0u8; 1];
                loop {
                    match socket.read(&mut byte).await {
                        Ok(0) | Err(_) => return,
                        Ok(_) => {}
                    }
                    head.push(byte[0]);
                    if head.ends_with(b"\r\n\r\n") || head.len() > 8192 {
                        break;
                    }
                }
                let body_len = proxy_content_length(&head);
                let mut recorded = head.clone();
                if body_len > 0 {
                    let mut body = vec![0u8; body_len];
                    if socket.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    recorded.extend_from_slice(&body);
                }
                let is_sts = recorded.starts_with(&sts_request_prefix);
                let body = b"<ErrorResponse><Error><Code>InvalidAction</Code><Message>test</Message></Error><RequestId>test</RequestId></ErrorResponse>";
                let response = format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(body).await;
                let _ = socket.flush().await;
                let _ = socket.shutdown().await;
                if is_sts {
                    let _ = request_tx.take().map(|tx| tx.send(recorded));
                    return;
                }
            }
        });
        (addr, request_rx)
    }

    async fn sdk_http_through_recording_proxy(
        sts_request_prefix: &[u8],
    ) -> (AwsSdkReqwestClient, tokio::sync::oneshot::Receiver<Vec<u8>>) {
        let (proxy_addr, request_rx) = start_recording_proxy(sts_request_prefix).await;
        let proxy = crate::tls::ProxyConfig {
            host: Some(proxy_addr.ip().to_string()),
            port: Some(i64::from(proxy_addr.port())),
            ..Default::default()
        };
        let sdk_http = AwsSdkReqwestClient::build(
            &crate::tls::TlsConfig::default(),
            Some(&proxy),
            crate::crl::CrlManager::new().connection_crl(),
            crate::tls::RootStoreProvider::new(),
        )
        .expect("SDK HTTP client must build");
        (sdk_http, request_rx)
    }

    fn aws_offline_signed_sts_env() -> [(&'static str, Option<&'static str>); 12] {
        [
            ("AWS_REGION", Some("us-east-1")),
            ("AWS_DEFAULT_REGION", None),
            ("AWS_ACCESS_KEY_ID", Some("test-key")),
            ("AWS_SECRET_ACCESS_KEY", Some("test-secret")),
            ("AWS_SESSION_TOKEN", None),
            ("AWS_PROFILE", None),
            (
                "AWS_SHARED_CREDENTIALS_FILE",
                Some("/nonexistent/aws-credentials-for-tests"),
            ),
            ("AWS_CONFIG_FILE", Some("/nonexistent/aws-config-for-tests")),
            ("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI", None),
            ("AWS_CONTAINER_CREDENTIALS_FULL_URI", None),
            ("AWS_WEB_IDENTITY_TOKEN_FILE", None),
            ("AWS_EC2_METADATA_DISABLED", Some("true")),
        ]
    }

    async fn sts_request_head_via_recording_proxy(
        sts_endpoint: Option<&AwsStsEndpoint>,
        client_endpoint_url: Option<&str>,
    ) -> String {
        let (sdk_http, request_rx) =
            sdk_http_through_recording_proxy(b"POST http://sts-target.invalid/").await;
        let credentials = Credentials::new("test-key", "test-secret", None, None, "test");
        let sdk_config = sdk_config_with_shared_transport(
            &sdk_http,
            "us-east-1",
            Some(&credentials),
            sts_endpoint,
        )
        .await;
        // Retries are off and the timeouts are explicit so a transport
        // regression surfaces as the assertion below rather than as a hang:
        // under the SDK's default standard retry mode a first attempt that
        // looks like a connection error is replayed, and the replay races the
        // proxy's accept loop.
        let mut sts_config = StsConfigBuilder::from(&sdk_config)
            .retry_config(aws_config::retry::RetryConfig::disabled())
            .timeout_config(
                aws_config::timeout::TimeoutConfig::builder()
                    .operation_timeout(std::time::Duration::from_secs(2))
                    .operation_attempt_timeout(std::time::Duration::from_secs(2))
                    .build(),
            );
        if let Some(url) = client_endpoint_url {
            sts_config = sts_config.endpoint_url(url);
        }
        let sts_config = sts_config.build();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            StsClient::from_conf(sts_config)
                .get_caller_identity()
                .send(),
        )
        .await
        .expect("STS call must not hang");
        let request = tokio::time::timeout(std::time::Duration::from_secs(5), request_rx)
            .await
            .expect("proxy observation must not hang")
            .expect("proxy must receive the STS request");
        assert!(result.is_err(), "the test proxy returns an STS error");
        String::from_utf8(request).expect("request head must be utf-8")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sts_sdk_uses_the_supplied_reqwest_transport() {
        let request =
            sts_request_head_via_recording_proxy(None, Some("http://sts-target.invalid")).await;
        assert!(
            request.starts_with("POST http://sts-target.invalid/ HTTP/1.1\r\n"),
            "proxy received unexpected request head: {request}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sts_sdk_pins_overridden_wif_host_as_endpoint() {
        let endpoint = parse_workload_identity_host("http://sts-target.invalid").expect("wif_host");
        let request = sts_request_head_via_recording_proxy(Some(&endpoint), None).await;
        assert!(
            request.starts_with("POST http://sts-target.invalid/ HTTP/1.1\r\n"),
            "proxy received unexpected request head: {request}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_web_identity_token_routes_to_wif_host() {
        let (sdk_http, request_rx) =
            sdk_http_through_recording_proxy(b"POST http://sts.example.snowflake.com/").await;
        let config = WorkloadIdentityConfig {
            provider: WifProvider::Aws,
            entra_resource: None,
            azure_client_id: None,
            impersonation_path: vec![],
            aws_use_outbound_token: true,
            oidc_token: None,
            wif_host: Some("http://sts.example.snowflake.com".to_string()),
        };
        let endpoints = AttestationEndpoints {
            aws_imds_base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };

        temp_env::async_with_vars(aws_offline_signed_sts_env(), async {
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                get_web_identity_token(&sdk_http, &config, &endpoints),
            )
            .await;
        })
        .await;

        let request = tokio::time::timeout(std::time::Duration::from_secs(5), request_rx)
            .await
            .expect("proxy observation must not hang")
            .expect("proxy must receive the STS request");
        let request = String::from_utf8(request).expect("request must be utf-8");
        assert!(
            request.starts_with("POST http://sts.example.snowflake.com/ HTTP/1.1\r\n"),
            "proxy received unexpected request: {request}"
        );
        assert!(
            request.contains("Action=GetWebIdentityToken"),
            "expected GetWebIdentityToken in the STS body, got: {request}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn assume_role_routes_to_wif_host() {
        let (sdk_http, request_rx) =
            sdk_http_through_recording_proxy(b"POST http://sts.example.snowflake.com/").await;
        let sts_endpoint =
            parse_workload_identity_host("http://sts.example.snowflake.com").expect("wif_host");
        let role_arns = vec!["arn:aws:iam::123456789012:role/test-role".to_string()];

        temp_env::async_with_vars(aws_offline_signed_sts_env(), async {
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                chain_assume_role(&sdk_http, "us-east-1", &role_arns, &sts_endpoint),
            )
            .await;
        })
        .await;

        let request = tokio::time::timeout(std::time::Duration::from_secs(5), request_rx)
            .await
            .expect("proxy observation must not hang")
            .expect("proxy must receive the STS request");
        let request = String::from_utf8(request).expect("request must be utf-8");
        assert!(
            request.starts_with("POST http://sts.example.snowflake.com/ HTTP/1.1\r\n"),
            "proxy received unexpected request: {request}"
        );
        assert!(
            request.contains("Action=AssumeRole"),
            "expected AssumeRole in the STS body, got: {request}"
        );
    }

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
            azure_client_id: None,
            impersonation_path: vec![],
            aws_use_outbound_token: false,
            oidc_token: None,
            wif_host: None,
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
                let sdk_http =
                    AwsSdkReqwestClient::with_default_tls(crate::tls::RootStoreProvider::new())
                        .await
                        .expect("default-TLS SDK client must build");
                let err = resolve_credentials(&sdk_http, &config, "us-east-1")
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

    fn aws_config(aws_use_outbound_token: bool) -> WorkloadIdentityConfig {
        WorkloadIdentityConfig {
            provider: WifProvider::Aws,
            entra_resource: None,
            azure_client_id: None,
            impersonation_path: vec![],
            aws_use_outbound_token,
            oidc_token: None,
            wif_host: None,
        }
    }

    /// Legacy `test_aws_token_format_based_on_connection_option` matrix:
    /// `(True, false-env) → outbound`, `(False, true-env) → outbound`.
    /// Dispatch (`if enable_outbound_token(config) { get_web_identity_token }
    /// else { get_caller_identity_token }`) delays to the AWS SDK once
    /// outbound is selected, so these tests cover the boolean decision.
    #[test]
    fn enable_outbound_token_defaults_to_false_when_unset() {
        temp_env::with_var(
            "SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN",
            None::<&str>,
            || {
                assert!(!enable_outbound_token(&aws_config(false)));
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
                        enable_outbound_token(&aws_config(false)),
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
                        !enable_outbound_token(&aws_config(false)),
                        "{value:?} should not enable outbound token"
                    );
                },
            );
        }
    }

    #[test]
    fn enable_outbound_token_connection_param_enables_without_env() {
        temp_env::with_var(
            "SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN",
            None::<&str>,
            || {
                assert!(enable_outbound_token(&aws_config(true)));
            },
        );
    }

    #[test]
    fn enable_outbound_token_connection_param_takes_precedence_over_env() {
        temp_env::with_var(
            "SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN",
            Some("false"),
            || {
                assert!(enable_outbound_token(&aws_config(true)));
            },
        );
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

    fn sts_authority(region: &str, wif_host: Option<&str>) -> String {
        aws_sts_endpoint_for(region, wif_host)
            .expect("STS endpoint")
            .authority
    }

    #[test]
    fn sts_hostname_china() {
        assert_eq!(
            sts_authority("cn-northwest-1", None),
            "sts.cn-northwest-1.amazonaws.com.cn"
        );
    }

    #[test]
    fn sts_hostname_standard() {
        assert_eq!(
            sts_authority("us-east-1", None),
            "sts.us-east-1.amazonaws.com"
        );
    }

    #[test]
    fn sts_hostname_keeps_regional_default_when_wif_host_is_absent_or_empty() {
        assert_eq!(
            sts_authority("us-east-1", None),
            "sts.us-east-1.amazonaws.com"
        );
        assert_eq!(
            sts_authority("cn-north-1", Some("")),
            "sts.cn-north-1.amazonaws.com.cn"
        );
        assert_eq!(
            sts_authority("us-east-1", Some("   ")),
            "sts.us-east-1.amazonaws.com"
        );
    }

    #[test]
    fn sts_hostname_covers_govcloud_and_additional_standard_regions() {
        let cases = [
            ("us-gov-west-1", "sts.us-gov-west-1.amazonaws.com"),
            ("us-gov-east-1", "sts.us-gov-east-1.amazonaws.com"),
            ("af-south-1", "sts.af-south-1.amazonaws.com"),
            ("eu-central-1", "sts.eu-central-1.amazonaws.com"),
            ("ap-southeast-2", "sts.ap-southeast-2.amazonaws.com"),
            ("cn-north-1", "sts.cn-north-1.amazonaws.com.cn"),
            ("us-isob-east-1", "sts.us-isob-east-1.sc2s.sgov.gov"),
            ("us-iso-east-1", "sts.us-iso-east-1.c2s.ic.gov"),
            ("eu-isoe-west-1", "sts.eu-isoe-west-1.cloud.adc-e.uk"),
            ("us-isof-south-1", "sts.us-isof-south-1.csp.hci.ic.gov"),
            ("eusc-de-east-1", "sts.eusc-de-east-1.amazonaws.eu"),
        ];
        for (region, expected) in cases {
            assert_eq!(sts_authority(region, None), expected, "region {region}");
        }
    }

    #[test]
    fn parse_workload_identity_host_accepts_bare_host_url_and_port() {
        let bare = parse_workload_identity_host("sts.sc2s.sgov.gov").expect("bare host");
        assert_eq!(bare.authority, "sts.sc2s.sgov.gov");
        assert_eq!(bare.base_url, "https://sts.sc2s.sgov.gov");
        assert!(bare.overridden);

        let https = parse_workload_identity_host("https://sts.sc2s.sgov.gov").expect("https host");
        assert_eq!(https.authority, "sts.sc2s.sgov.gov");
        assert_eq!(https.base_url, "https://sts.sc2s.sgov.gov");

        let http = parse_workload_identity_host("http://sts.example.snowflake.com:8443")
            .expect("http host with port");
        assert_eq!(http.authority, "sts.example.snowflake.com:8443");
        assert_eq!(http.base_url, "http://sts.example.snowflake.com:8443");

        let mixed = parse_workload_identity_host("STS.Example.Snowflake.COM").expect("mixed case");
        assert_eq!(mixed.authority, "sts.example.snowflake.com");

        let with_path = parse_workload_identity_host("https://sts.example.snowflake.com/foo/")
            .expect("host with path");
        assert_eq!(with_path.authority, "sts.example.snowflake.com");
        assert_eq!(with_path.base_url, "https://sts.example.snowflake.com/foo");

        let ipv6 = parse_workload_identity_host("https://[2001:db8::1]:8443").expect("ipv6 host");
        assert_eq!(ipv6.authority, "[2001:db8::1]:8443");
        assert_eq!(ipv6.base_url, "https://[2001:db8::1]:8443");
    }

    #[test]
    fn parse_workload_identity_host_rejects_empty_userinfo_query_fragment_and_bad_scheme() {
        for (input, echoed) in [
            ("", ""),
            (
                "user:pass@sts.example.snowflake.com",
                "sts.example.snowflake.com",
            ),
            (
                "https://user:pass@sts.example.snowflake.com",
                "sts.example.snowflake.com",
            ),
            (
                "https://sts.example.snowflake.com?x=1",
                "sts.example.snowflake.com",
            ),
            (
                "https://sts.example.snowflake.com#frag",
                "sts.example.snowflake.com",
            ),
            (
                "ftp://sts.example.snowflake.com",
                "sts.example.snowflake.com",
            ),
            (
                "https://sts.example.snowflake.com/foo?x=1",
                "sts.example.snowflake.com/foo",
            ),
            ("blob:https://user:LEAK@host/id?token=SECRET#frag", ""),
            (
                "view-source:https://user:LEAK@example.com/path?token=SECRET#frag",
                "",
            ),
            ("javascript:https://user:LEAK@example.com/x?y=SECRET#f", ""),
            ("filesystem:https://user:LEAK@example.com/secret?x=1#f", ""),
        ] {
            let err = parse_workload_identity_host(input).expect_err("rejected wif_host");
            assert!(
                matches!(err, AwsAttestationError::InvalidWifHost { .. }),
                "expected InvalidWifHost for {input:?}, got {err:?}"
            );
            assert_eq!(err.to_string(), format!("invalid wif_host '{echoed}'"));
        }
    }

    #[test]
    fn default_sts_endpoint_fails_closed_on_invalid_region() {
        let err = default_sts_endpoint("not a region").unwrap_err();
        assert!(
            matches!(err, AwsAttestationError::StsEndpointResolve { .. }),
            "expected fail-closed resolve error, got {err:?}"
        );
    }

    #[test]
    fn caller_identity_request_matches_sigv4_reference_with_session_token()
    -> Result<(), serde_json::Error> {
        let request = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            Some("session-token"),
            "us-east-1",
            "20240101T120000Z",
            "20240101",
            None,
        )
        .expect("regional STS host");

        let json = serde_json::to_string(&request)?;
        let decoded: serde_json::Value = serde_json::from_str(&json)?;

        assert_eq!(
            decoded["url"],
            "https://sts.us-east-1.amazonaws.com/?Action=GetCallerIdentity&Version=2011-06-15"
        );
        assert_eq!(decoded["method"], "POST");

        let headers = &decoded["headers"];
        assert_eq!(headers["Host"], "sts.us-east-1.amazonaws.com");
        assert_eq!(headers["X-Snowflake-Audience"], "snowflakecomputing.com");
        assert_eq!(headers["X-Amz-Date"], "20240101T120000Z");
        assert_eq!(headers["X-Amz-Security-Token"], "session-token");
        // Independent SigV4 reference using Python's hashlib/hmac over the
        // canonical POST request, including the session token in SignedHeaders.
        assert_eq!(
            headers["Authorization"],
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20240101/us-east-1/sts/aws4_request, SignedHeaders=host;x-amz-date;x-amz-security-token;x-snowflake-audience, Signature=ce598263c65db61df01e9b76a7226fdfcd6673fc474cbca3cbdc7ae4ccf53bca"
        );

        Ok(())
    }

    #[test]
    fn caller_identity_request_uses_wif_host_verbatim() -> Result<(), serde_json::Error> {
        let request = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            None,
            "cn-north-1",
            "20240101T120000Z",
            "20240101",
            Some("sts.sc2s.sgov.gov"),
        )
        .expect("wif_host override");

        let json = serde_json::to_string(&request)?;
        let decoded: serde_json::Value = serde_json::from_str(&json)?;

        assert_eq!(
            decoded["url"],
            "https://sts.sc2s.sgov.gov/?Action=GetCallerIdentity&Version=2011-06-15"
        );
        assert_eq!(decoded["headers"]["Host"], "sts.sc2s.sgov.gov");

        Ok(())
    }

    #[test]
    fn caller_identity_request_strips_http_prefix_from_wif_host() -> Result<(), serde_json::Error> {
        let request = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            None,
            "cn-north-1",
            "20240101T120000Z",
            "20240101",
            Some("https://sts.sc2s.sgov.gov"),
        )
        .expect("https wif_host override");
        let decoded: serde_json::Value = serde_json::from_str(&serde_json::to_string(&request)?)?;
        assert_eq!(
            decoded["url"],
            "https://sts.sc2s.sgov.gov/?Action=GetCallerIdentity&Version=2011-06-15"
        );
        assert_eq!(decoded["headers"]["Host"], "sts.sc2s.sgov.gov");

        let http = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            None,
            "cn-north-1",
            "20240101T120000Z",
            "20240101",
            Some("http://sts.example.snowflake.com:8443"),
        )
        .expect("http wif_host override");
        let http: serde_json::Value = serde_json::from_str(&serde_json::to_string(&http)?)?;
        assert_eq!(
            http["url"],
            "http://sts.example.snowflake.com:8443/?Action=GetCallerIdentity&Version=2011-06-15"
        );
        assert_eq!(http["headers"]["Host"], "sts.example.snowflake.com:8443");

        let with_path = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            None,
            "cn-north-1",
            "20240101T120000Z",
            "20240101",
            Some("https://sts.example.snowflake.com/foo/"),
        )
        .expect("path wif_host override");
        let with_path: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&with_path)?)?;
        assert_eq!(
            with_path["url"],
            "https://sts.example.snowflake.com/foo?Action=GetCallerIdentity&Version=2011-06-15"
        );
        assert_eq!(with_path["headers"]["Host"], "sts.example.snowflake.com");

        let ipv6 = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            None,
            "cn-north-1",
            "20240101T120000Z",
            "20240101",
            Some("https://[2001:db8::1]:8443"),
        )
        .expect("ipv6 wif_host override");
        let ipv6: serde_json::Value = serde_json::from_str(&serde_json::to_string(&ipv6)?)?;
        assert_eq!(
            ipv6["url"],
            "https://[2001:db8::1]:8443/?Action=GetCallerIdentity&Version=2011-06-15"
        );
        assert_eq!(ipv6["headers"]["Host"], "[2001:db8::1]:8443");

        Ok(())
    }

    #[test]
    fn caller_identity_request_matches_sigv4_reference_without_session_token()
    -> Result<(), serde_json::Error> {
        let request = build_signed_caller_identity_request(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            None,
            "eu-west-1",
            "20240101T120000Z",
            "20240101",
            None,
        )
        .expect("regional STS host");

        let json = serde_json::to_string(&request)?;
        let decoded: serde_json::Value = serde_json::from_str(&json)?;

        assert!(decoded["headers"].get("X-Amz-Security-Token").is_none());
        assert_eq!(
            decoded["headers"]["Authorization"],
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20240101/eu-west-1/sts/aws4_request, SignedHeaders=host;x-amz-date;x-snowflake-audience, Signature=7f6f01b5ad72d73b9af0f5e06e3c782fd3d63600efd29a1bcdd28fa379d0fbce"
        );

        Ok(())
    }
}
