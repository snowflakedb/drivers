use super::types::{
    CloudCredentials, CredentialRefreshError, CredentialRefresher, EncryptedFileMetadata,
    MaterialDescription, PreparedUpload, StageInfo, UploadStatus,
};
use crate::config::retry::{BackoffConfig, Jitter, RetryPolicy};
use snafu::{Location, ResultExt, Snafu};
use std::sync::Arc;
use std::time::Duration;

// AWS SDK imports
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::config::retry::RetryConfig as AwsRetryConfig;
use aws_sdk_s3::config::timeout::TimeoutConfig as AwsTimeoutConfig;
use aws_sdk_s3::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};

const SNOWFLAKE_UPLOAD_PROVIDER: &str = "snowflake-upload";
const SNOWFLAKE_DOWNLOAD_PROVIDER: &str = "snowflake-download";
const CONTENT_TYPE_OCTET_STREAM: &str = "application/octet-stream";

/// AWS error code returned when an STS session token has expired. Stable
/// across S3 and STS and surfaced via `ProvideErrorMetadata::code()` for
/// responses with bodies. HEAD responses carry no body (the SDK drops them),
/// so S3 mirrors this code into the `x-amz-error-code` response header; we
/// inspect that header explicitly for HEAD.
const EXPIRED_TOKEN_CODE: &str = "ExpiredToken";

const X_AMZ_ERROR_CODE_HEADER: &str = "x-amz-error-code";

/// Maximum number of credential-refresh rounds per transfer. One refresh is
/// typically sufficient (STS credentials are long-lived); a second catches
/// the edge case where a refresh races with another expiry. Additional
/// attempts would loop on a broken refresher.
const MAX_CREDENTIAL_REFRESH_ATTEMPTS: u32 = 2;

/// Per-attempt HTTP timeout applied to every S3 SDK operation.
///
/// Matches the Azure/GCS transfer timeout (300s). The retry budget
/// (`max_elapsed` in `s3_retry_policy`) must exceed this so at least one
/// full attempt can complete.
const REQUEST_TIMEOUT_SECS: u64 = 300;

// TODO: streaming instead of loading the whole file into memory

/// Uploads a file to S3, skipping if it already exists and `overwrite` is false.
///
/// If `credential_refresher` is `Some` and the S3 API returns `ExpiredToken`,
/// fresh credentials are fetched and the operation is retried with a rebuilt
/// S3 client (bounded by `MAX_CREDENTIAL_REFRESH_ATTEMPTS`).
pub async fn upload_to_s3_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
    credential_refresher: Option<Arc<dyn CredentialRefresher>>,
) -> Result<UploadStatus, UploadFileError> {
    let s3_key = format!("{}{filename}", stage_info.key_prefix);
    // Creds used by the S3 client for this iteration. Starts as `stage_info.creds`
    // (via `None` → `create_s3_client` falls back to stage_info) and is replaced
    // by refreshed creds after an ExpiredToken.
    let mut active_creds: Option<CloudCredentials> = None;
    let mut refreshes_done: u32 = 0;
    loop {
        let s3_client =
            create_s3_client(stage_info, SNOWFLAKE_UPLOAD_PROVIDER, active_creds.as_ref()).await?;
        match try_upload_once(&prepared, &s3_client, stage_info, &s3_key, overwrite).await {
            Ok(status) => return Ok(status),
            Err(e) => match try_refresh_after_expired_token(
                is_expired_token_upload_error(&e),
                credential_refresher.as_deref(),
                refreshes_done,
                |source| UploadFileError::CredentialRefresh {
                    source,
                    location: Location::default(),
                },
            )
            .await?
            {
                Some(fresh) => {
                    active_creds = Some(fresh);
                    refreshes_done += 1;
                }
                None => return Err(e),
            },
        }
    }
}

/// Performs one S3 upload attempt: existence check (if `!overwrite`) followed
/// by PUT. Isolated so the caller's refresh loop sees a single `Result` per
/// attempt rather than two phases.
async fn try_upload_once(
    prepared: &PreparedUpload,
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
    overwrite: bool,
) -> Result<UploadStatus, UploadFileError> {
    if !overwrite && check_if_file_exists(s3_client, stage_info, s3_key).await? {
        tracing::info!("File already exists in S3: {}", s3_key);
        return Ok(UploadStatus::Skipped);
    }
    // Clone per attempt — STS refresh is rare (~hourly), not a hot path.
    upload_to_s3(prepared.clone(), s3_client, stage_info, s3_key).await?;
    Ok(UploadStatus::Uploaded)
}

/// Returns `Some(fresh_creds)` when the caller should retry with fresh
/// credentials; `Ok(None)` when the caller should propagate the original
/// error unchanged (not an expired-token error, no refresher configured, or
/// the refresh budget is exhausted).
///
/// `on_refresh_error` lifts a `CredentialRefreshError` into the caller's
/// storage-client error type — each caller picks the right variant (e.g.
/// `UploadFileError::CredentialRefresh` vs `DownloadFileError::CredentialRefresh`).
async fn try_refresh_after_expired_token<E>(
    is_expired_token: bool,
    refresher: Option<&dyn CredentialRefresher>,
    refreshes_done: u32,
    on_refresh_error: impl FnOnce(CredentialRefreshError) -> E,
) -> Result<Option<CloudCredentials>, E> {
    if !is_expired_token {
        return Ok(None);
    }
    let Some(refresher) = refresher else {
        tracing::debug!("S3 ExpiredToken but no credential refresher configured");
        return Ok(None);
    };
    if refreshes_done >= MAX_CREDENTIAL_REFRESH_ATTEMPTS {
        tracing::warn!(
            refreshes_done,
            "S3 ExpiredToken persists after {refreshes_done} refresh(es); giving up"
        );
        return Ok(None);
    }
    tracing::debug!(
        refreshes_done,
        "Refreshing S3 credentials after ExpiredToken"
    );
    refresher
        .refresh()
        .await
        .map(Some)
        .map_err(on_refresh_error)
}

/// Returns true if the file exists in S3, false if it does not.
///
/// When the check cannot be performed due to 403 Forbidden (limited temporary
/// credentials that allow PUT but not HEAD), returns false so the caller
/// proceeds with upload. An expired STS token is surfaced via the
/// `x-amz-error-code` header (HEAD responses carry no body, so the SDK cannot
/// populate the normal `ProvideErrorMetadata::code()` path).
async fn check_if_file_exists(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<bool, UploadFileError> {
    match s3_client
        .head_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .send()
        .await
    {
        Ok(_) => Ok(true),
        Err(SdkError::ServiceError(err)) if err.err().is_not_found() => Ok(false),
        Err(SdkError::ServiceError(ref err)) if err.raw().status().as_u16() == 403 => {
            tracing::warn!(
                "Access denied when checking if file exists in S3 ({s3_key}), proceeding with upload"
            );
            Ok(false)
        }
        Err(SdkError::ServiceError(ref err))
            if err
                .raw()
                .headers()
                .get(X_AMZ_ERROR_CODE_HEADER)
                .is_some_and(|code| code == EXPIRED_TOKEN_CODE) =>
        {
            Err(UploadFileError::HeadExpiredToken {
                location: Location::default(),
            })
        }
        Err(e) => Err(aws_sdk_s3::Error::from(e)).context(upload_file_error::S3HeadSnafu),
    }
}

async fn upload_to_s3(
    prepared: PreparedUpload,
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<(), UploadFileError> {
    let mut put_object_request = s3_client
        .put_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .body(ByteStream::from(prepared.data))
        .content_type(CONTENT_TYPE_OCTET_STREAM)
        .metadata("sfc-digest", &prepared.digest);

    if let Some(ref enc_meta) = prepared.encryption_metadata {
        let mat_desc = serde_json::to_string(&enc_meta.material_desc)
            .context(upload_file_error::SerializationSnafu)?;
        put_object_request = put_object_request
            .metadata("x-amz-iv", &enc_meta.iv)
            .metadata("x-amz-key", &enc_meta.encrypted_key)
            .metadata("x-amz-matdesc", mat_desc);
    }

    tracing::trace!("PUT object request: {:?}", put_object_request);

    let result = put_object_request
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
        .context(upload_file_error::S3UploadSnafu)?;

    tracing::debug!("S3 upload result: {:?}", result);

    Ok(())
}

/// Downloads a file from S3 and returns the data with optional encryption metadata.
/// For SSE stages the metadata headers will be absent and `None` is returned.
///
/// If `credential_refresher` is `Some` and the S3 API returns an `ExpiredToken`,
/// fresh credentials are fetched and the operation is retried with a rebuilt
/// S3 client (bounded by `MAX_CREDENTIAL_REFRESH_ATTEMPTS`).
pub async fn download_from_s3(
    stage_info: &StageInfo,
    filename: &str,
    credential_refresher: Option<Arc<dyn CredentialRefresher>>,
) -> Result<(Vec<u8>, Option<String>, Option<EncryptedFileMetadata>), DownloadFileError> {
    let s3_key = format!("{}{filename}", stage_info.key_prefix);
    let mut active_creds: Option<CloudCredentials> = None;
    let mut refreshes_done: u32 = 0;
    loop {
        let s3_client = create_s3_client(
            stage_info,
            SNOWFLAKE_DOWNLOAD_PROVIDER,
            active_creds.as_ref(),
        )
        .await?;
        match try_download_once(&s3_client, stage_info, &s3_key).await {
            Ok(parsed) => return Ok(parsed),
            Err(e) => match try_refresh_after_expired_token(
                is_expired_token_download_error(&e),
                credential_refresher.as_deref(),
                refreshes_done,
                |source| DownloadFileError::CredentialRefresh {
                    source,
                    location: Location::default(),
                },
            )
            .await?
            {
                Some(fresh) => {
                    active_creds = Some(fresh);
                    refreshes_done += 1;
                }
                None => return Err(e),
            },
        }
    }
}

async fn try_download_once(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<(Vec<u8>, Option<String>, Option<EncryptedFileMetadata>), DownloadFileError> {
    let response = s3_client
        .get_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
        .context(download_file_error::S3DownloadSnafu)?;

    let metadata_map = response.metadata().cloned().unwrap_or_default();

    let digest = metadata_map.get("sfc-digest").cloned();

    let mat_desc = metadata_map.get("x-amz-matdesc");
    let encrypted_key = metadata_map.get("x-amz-key");
    let iv = metadata_map.get("x-amz-iv");

    let file_metadata = match (mat_desc, encrypted_key, iv) {
        (Some(mat_desc_str), Some(key), Some(iv_val)) => {
            let material_desc: MaterialDescription = serde_json::from_str(mat_desc_str)
                .context(download_file_error::DeserializationSnafu)?;
            Some(EncryptedFileMetadata {
                encrypted_key: key.to_owned(),
                iv: iv_val.to_owned(),
                material_desc,
            })
        }
        (None, None, None) => None,
        _ => {
            return download_file_error::MissingFileMetadataSnafu {
                field: "partial encryption headers (x-amz-matdesc, x-amz-key, x-amz-iv)"
                    .to_string(),
            }
            .fail();
        }
    };

    let data = response
        .body
        .collect()
        .await
        .context(download_file_error::ByteStreamSnafu)?
        .into_bytes()
        .to_vec();

    Ok((data, digest, file_metadata))
}

/// Returns a retry policy tuned for S3 file-transfer operations.
///
/// Mirrors the shape and budget of the GCS/Azure policies so that cross-cloud
/// behavior is consistent: 6 attempts, exponential backoff from 1s to 16s,
/// and a total retry budget of 600s (2× `REQUEST_TIMEOUT_SECS`) so at least
/// one full-timeout attempt can complete before the budget expires.
///
/// The AWS SDK's standard retry strategy already covers transient transport
/// errors, 5xx server errors, and throttling (429, SlowDown). 403 is left to
/// the SDK's defaults: unlike GCS/Azure (where 403 commonly means "token not
/// yet propagated" / "SAS clock skew"), S3 returns 403 for genuine AccessDenied
/// and retrying is rarely productive. STS token expiry is surfaced by S3 as a
/// 400 with error code `ExpiredToken`; that case is caught by the
/// credential-refresh loop (`upload_to_s3_or_skip`, `download_from_s3`) rather
/// than by the SDK's retry strategy.
pub(crate) fn s3_retry_policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 6,
        backoff: BackoffConfig {
            base: Duration::from_secs(1),
            factor: 2.0,
            cap: Duration::from_secs(16),
            jitter: Jitter::None,
        },
        // Must exceed REQUEST_TIMEOUT_SECS (300s) to allow at least one full
        // request + retries. 600s accommodates ~2 full-timeout attempts plus backoff.
        max_elapsed: Duration::from_secs(600),
        per_request_timeout: Some(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
        extra_retryable_statuses: Vec::new(),
        ..RetryPolicy::default()
    }
}

/// Translates the driver's `RetryPolicy` into the AWS SDK's `RetryConfig`.
///
/// The SDK owns the retry loop for S3, so we hand it our knobs — attempt count
/// and backoff bounds — and let it classify errors. AWS's standard mode already
/// retries transient transport faults, 5xx, and throttling (429, SlowDown)
/// with exponential backoff and jitter.
fn to_aws_retry_config(policy: &RetryPolicy) -> AwsRetryConfig {
    AwsRetryConfig::standard()
        .with_max_attempts(policy.max_attempts)
        .with_initial_backoff(policy.backoff.base)
        .with_max_backoff(policy.backoff.cap)
}

/// Builds the SDK's `TimeoutConfig` from our policy.
///
/// - `operation_attempt_timeout` bounds a single try (so retries are actually
///   triggered on stuck connections rather than hanging forever).
/// - `operation_timeout` bounds the total retry budget.
fn to_aws_timeout_config(policy: &RetryPolicy) -> AwsTimeoutConfig {
    let mut builder = AwsTimeoutConfig::builder().operation_timeout(policy.max_elapsed);
    if let Some(per_attempt) = policy.per_request_timeout {
        builder = builder.operation_attempt_timeout(per_attempt);
    }
    builder.build()
}

async fn create_s3_client(
    stage_info: &StageInfo,
    provider_name: &'static str,
    creds_override: Option<&CloudCredentials>,
) -> Result<S3Client, S3CredentialError> {
    let effective_creds = creds_override.unwrap_or(&stage_info.creds);
    let super::types::CloudCredentials::S3 {
        ref aws_key_id,
        ref aws_secret_key,
        ref aws_token,
    } = *effective_creds
    else {
        return Err(S3CredentialError);
    };

    let credentials = Credentials::new(
        aws_key_id,
        aws_secret_key.reveal(),
        Some(aws_token.reveal().to_string()),
        None,
        provider_name,
    );

    let policy = s3_retry_policy();
    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(stage_info.region.clone()))
        .retry_config(to_aws_retry_config(&policy))
        .timeout_config(to_aws_timeout_config(&policy))
        .load()
        .await;

    let mut s3_config = aws_sdk_s3::config::Builder::from(&config);
    if let Some(end_point) = &stage_info.end_point {
        let endpoint_url = if end_point.starts_with("https://") || end_point.starts_with("http://")
        {
            end_point.clone()
        } else {
            format!("https://{end_point}")
        };
        tracing::debug!("Using Snowflake-provided S3 endpoint: {endpoint_url}");
        s3_config = s3_config.endpoint_url(endpoint_url);
    }

    Ok(S3Client::from_conf(s3_config.build()))
}

/// Returns true if this `UploadFileError` wraps an S3 error with code
/// `ExpiredToken`. The SDK parses the XML error body and exposes the code via
/// `ProvideErrorMetadata`. HEAD requests surface the code via a typed variant
/// because their responses have no body to parse.
fn is_expired_token_upload_error(err: &UploadFileError) -> bool {
    match err {
        UploadFileError::S3Upload { source, .. } | UploadFileError::S3Head { source, .. } => {
            source.code() == Some(EXPIRED_TOKEN_CODE)
        }
        UploadFileError::HeadExpiredToken { .. } => true,
        _ => false,
    }
}

fn is_expired_token_download_error(err: &DownloadFileError) -> bool {
    match err {
        DownloadFileError::S3Download { source, .. } => source.code() == Some(EXPIRED_TOKEN_CODE),
        _ => false,
    }
}

/// Error returned when `create_s3_client` is called with non-S3 credentials.
#[derive(Debug)]
struct S3CredentialError;

impl From<S3CredentialError> for UploadFileError {
    fn from(_: S3CredentialError) -> Self {
        UploadFileError::MissingS3Credentials {
            location: Location::default(),
        }
    }
}

impl From<S3CredentialError> for DownloadFileError {
    fn from(_: S3CredentialError) -> Self {
        DownloadFileError::MissingS3Credentials {
            location: Location::default(),
        }
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum UploadFileError {
    #[snafu(display("Failed to upload file to S3"))]
    S3Upload {
        #[snafu(source(from(aws_sdk_s3::Error, Box::new)))]
        source: Box<aws_sdk_s3::Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to check if file exists in S3"))]
    S3Head {
        #[snafu(source(from(aws_sdk_s3::Error, Box::new)))]
        source: Box<aws_sdk_s3::Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize metadata during file upload"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing S3 credentials"))]
    MissingS3Credentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("S3 HEAD returned ExpiredToken"))]
    HeadExpiredToken {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to refresh S3 credentials during upload"))]
    CredentialRefresh {
        source: CredentialRefreshError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum DownloadFileError {
    #[snafu(display("Failed to download file from S3"))]
    S3Download {
        #[snafu(source(from(aws_sdk_s3::Error, Box::new)))]
        source: Box<aws_sdk_s3::Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to deserialize metadata during file download"))]
    Deserialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("File metadata missing: {field}"))]
    MissingFileMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read byte stream from S3"))]
    ByteStream {
        source: aws_sdk_s3::primitives::ByteStreamError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing S3 credentials"))]
    MissingS3Credentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to refresh S3 credentials during download"))]
    CredentialRefresh {
        source: CredentialRefreshError,
        #[snafu(implicit)]
        location: Location,
    },
}

// --- Unit tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s3_retry_policy_max_attempts_matches_gcs_and_azure() {
        let policy = s3_retry_policy();
        assert_eq!(policy.max_attempts, 6);
    }

    #[test]
    fn s3_retry_policy_backoff_bounds() {
        let policy = s3_retry_policy();
        assert_eq!(policy.backoff.base, Duration::from_secs(1));
        assert_eq!(policy.backoff.cap, Duration::from_secs(16));
        assert_eq!(policy.backoff.factor, 2.0);
    }

    #[test]
    fn s3_retry_policy_max_elapsed_exceeds_request_timeout() {
        let policy = s3_retry_policy();
        assert!(
            policy.max_elapsed > Duration::from_secs(REQUEST_TIMEOUT_SECS),
            "retry budget must exceed a single request timeout"
        );
        assert_eq!(policy.max_elapsed, Duration::from_secs(600));
    }

    #[test]
    fn s3_retry_policy_has_per_request_timeout() {
        let policy = s3_retry_policy();
        assert_eq!(
            policy.per_request_timeout,
            Some(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
            "per_request_timeout must be set so the SDK cancels stuck attempts"
        );
    }

    #[test]
    fn to_aws_retry_config_translates_policy() {
        let policy = s3_retry_policy();
        let aws = to_aws_retry_config(&policy);
        assert_eq!(aws.max_attempts(), policy.max_attempts);
        assert_eq!(aws.initial_backoff(), policy.backoff.base);
        assert_eq!(aws.max_backoff(), policy.backoff.cap);
    }

    #[test]
    fn to_aws_timeout_config_sets_attempt_and_operation_timeouts() {
        let policy = s3_retry_policy();
        let cfg = to_aws_timeout_config(&policy);
        assert_eq!(cfg.operation_timeout(), Some(policy.max_elapsed));
        assert_eq!(cfg.operation_attempt_timeout(), policy.per_request_timeout);
    }

    // --- Credential refresh tests ---
    //
    // Drive the refresh path end-to-end against a `wiremock` MockServer that
    // impersonates S3. For PUT/GET the SDK parses the XML error body; for HEAD
    // the SDK reads the `x-amz-error-code` response header (HEAD has no body).

    use super::super::types::{CloudCredentials, CredentialRefresher, PreparedUpload, StageInfo};
    use crate::file_manager::LocationType;
    use crate::sensitive::SensitiveString;
    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    const EXPIRED_TOKEN_XML: &str = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        "<Error><Code>ExpiredToken</Code>",
        "<Message>The provided token has expired.</Message>",
        "<RequestId>req-0</RequestId><HostId>host-0</HostId></Error>"
    );

    fn make_s3_creds(key: &str, secret: &str, token: &str) -> CloudCredentials {
        CloudCredentials::S3 {
            aws_key_id: key.to_string(),
            aws_secret_key: SensitiveString::from(secret),
            aws_token: SensitiveString::from(token),
        }
    }

    fn make_stage_info(endpoint: &str, creds: CloudCredentials) -> StageInfo {
        StageInfo {
            location_type: LocationType::S3,
            bucket: "test-bucket".to_string(),
            key_prefix: "prefix/".to_string(),
            region: "us-east-1".to_string(),
            creds,
            end_point: Some(endpoint.to_string()),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            storage_account: None,
        }
    }

    /// Responds with `ExpiredToken` for the first N calls, then delegates to `on_success`.
    struct ExpireThenSucceed {
        expires_remaining: AtomicUsize,
        on_success_status: u16,
        on_success_body: Vec<u8>,
        on_success_headers: Vec<(String, String)>,
    }

    impl ExpireThenSucceed {
        fn new(
            expire_count: usize,
            status: u16,
            body: Vec<u8>,
            headers: Vec<(String, String)>,
        ) -> Self {
            Self {
                expires_remaining: AtomicUsize::new(expire_count),
                on_success_status: status,
                on_success_body: body,
                on_success_headers: headers,
            }
        }
    }

    impl Respond for ExpireThenSucceed {
        fn respond(&self, req: &Request) -> ResponseTemplate {
            // fetch_sub returns the previous value; > 0 means we still owe an expiry.
            let prev = self.expires_remaining.fetch_sub(1, Ordering::SeqCst);
            if prev == 0 {
                // Saturate back to 0 so repeated success calls don't underflow.
                self.expires_remaining.store(0, Ordering::SeqCst);
                let mut tmpl = ResponseTemplate::new(self.on_success_status)
                    .set_body_bytes(self.on_success_body.clone());
                for (k, v) in &self.on_success_headers {
                    tmpl = tmpl.insert_header(k.as_str(), v.as_str());
                }
                tmpl
            } else if prev > 0 {
                // HEAD has no response body per HTTP spec, and the AWS SDK
                // explicitly drops HEAD bodies (see `parse_http_error_metadata`
                // in aws-sdk-s3). Real S3 therefore mirrors the XML error code
                // into the `x-amz-error-code` header for HEAD. Match that here.
                let base = ResponseTemplate::new(400)
                    .insert_header(X_AMZ_ERROR_CODE_HEADER, EXPIRED_TOKEN_CODE);
                if req.method.as_str().eq_ignore_ascii_case("HEAD") {
                    base
                } else {
                    base.set_body_string(EXPIRED_TOKEN_XML)
                        .insert_header("Content-Type", "application/xml")
                }
            } else {
                // Shouldn't happen — fetch_sub saturated at 0 via the branch above.
                ResponseTemplate::new(500)
            }
        }
    }

    /// Refresher that records how many times it was called and returns fresh
    /// creds each time. Stand-in for the production PUT/GET re-execution.
    struct CountingRefresher {
        calls: Arc<AtomicUsize>,
        creds: StdMutex<Option<CloudCredentials>>,
    }

    impl CountingRefresher {
        fn new(creds: CloudCredentials) -> (Arc<Self>, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            let this = Arc::new(Self {
                calls: calls.clone(),
                creds: StdMutex::new(Some(creds)),
            });
            (this, calls)
        }
    }

    impl CredentialRefresher for CountingRefresher {
        fn refresh(
            &self,
        ) -> futures::future::BoxFuture<'_, Result<CloudCredentials, CredentialRefreshError>>
        {
            use futures::FutureExt;
            self.calls.fetch_add(1, Ordering::SeqCst);
            let creds = self
                .creds
                .lock()
                .unwrap()
                .clone()
                .expect("test refresher should always have creds ready");
            async move { Ok(creds) }.boxed()
        }
    }

    #[tokio::test]
    async fn upload_retries_and_refreshes_creds_on_expired_token() {
        let server = MockServer::start().await;

        // First HEAD (exists check) → 404 (not found) so upload proceeds.
        // First PUT → ExpiredToken. Second PUT → success.
        // Using a single catch-all responder simplifies matching; wiremock replays
        // mounted mocks in registration order and consumes `expires_remaining`
        // across all calls.
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ExpireThenSucceed::new(
                1,
                200,
                b"".to_vec(),
                vec![("ETag".into(), "\"abc\"".into())],
            ))
            .mount(&server)
            .await;

        let stage_info = make_stage_info(
            &server.uri(),
            make_s3_creds("STALE_KEY", "STALE_SECRET", "STALE_TOKEN"),
        );
        let (refresher, calls) =
            CountingRefresher::new(make_s3_creds("FRESH_KEY", "FRESH_SECRET", "FRESH_TOKEN"));

        let prepared = PreparedUpload {
            data: b"hello world".to_vec(),
            digest: "deadbeef".to_string(),
            encryption_metadata: None,
        };

        let status = upload_to_s3_or_skip(
            prepared,
            &stage_info,
            "file.txt",
            /*overwrite=*/ true,
            Some(refresher),
        )
        .await
        .expect("upload should succeed after credential refresh");

        assert_eq!(status, UploadStatus::Uploaded);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "exactly one credential refresh should happen"
        );
    }

    #[tokio::test]
    async fn upload_without_refresher_fails_fast_on_expired_token() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_string(EXPIRED_TOKEN_XML)
                    .insert_header("Content-Type", "application/xml"),
            )
            .mount(&server)
            .await;

        let stage_info = make_stage_info(
            &server.uri(),
            make_s3_creds("STALE_KEY", "STALE_SECRET", "STALE_TOKEN"),
        );
        let prepared = PreparedUpload {
            data: b"hello".to_vec(),
            digest: "d".to_string(),
            encryption_metadata: None,
        };

        let err = upload_to_s3_or_skip(
            prepared,
            &stage_info,
            "file.txt",
            /*overwrite=*/ true,
            None,
        )
        .await
        .expect_err("upload must fail when there is no refresher");

        // Surfaced as the original S3 upload error, not as ExpiredTokenRefreshExhausted:
        // without a refresher we propagate immediately on the first error.
        assert!(
            matches!(err, UploadFileError::S3Upload { .. }),
            "expected S3Upload error, got {err:?}"
        );
        assert!(
            is_expired_token_upload_error(&err),
            "error should be classified as ExpiredToken"
        );
    }

    #[tokio::test]
    async fn upload_gives_up_after_max_refresh_attempts() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        // Always return ExpiredToken — refresher keeps producing fresh creds,
        // but S3 still rejects them. We expect a bounded number of refreshes.
        Mock::given(method("PUT"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_string(EXPIRED_TOKEN_XML)
                    .insert_header("Content-Type", "application/xml"),
            )
            .mount(&server)
            .await;

        let stage_info = make_stage_info(
            &server.uri(),
            make_s3_creds("STALE_KEY", "STALE_SECRET", "STALE_TOKEN"),
        );
        let (refresher, calls) =
            CountingRefresher::new(make_s3_creds("F_KEY", "F_SECRET", "F_TOKEN"));

        let prepared = PreparedUpload {
            data: b"x".to_vec(),
            digest: "d".to_string(),
            encryption_metadata: None,
        };

        let err = upload_to_s3_or_skip(
            prepared,
            &stage_info,
            "file.txt",
            /*overwrite=*/ true,
            Some(refresher),
        )
        .await
        .expect_err("upload must fail when ExpiredToken persists");

        assert_eq!(
            calls.load(Ordering::SeqCst) as u32,
            MAX_CREDENTIAL_REFRESH_ATTEMPTS,
            "should refresh exactly MAX_CREDENTIAL_REFRESH_ATTEMPTS times",
        );
        // After the bound is hit, the final ExpiredToken propagates as-is.
        assert!(
            matches!(err, UploadFileError::S3Upload { .. }),
            "expected S3Upload error after giving up, got {err:?}"
        );
    }

    #[tokio::test]
    async fn head_check_refreshes_creds_on_expired_token() {
        // Mirrors the legacy `test_get_header_expiry_error` (test_s3_util.py:130):
        // when the HEAD-based file-exists probe hits ExpiredToken, the refresh
        // loop must invoke the refresher and retry on the second attempt. Here
        // the second HEAD returns 404 (file not found) so upload proceeds and
        // the PUT succeeds with the fresh creds.
        let server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ExpireThenSucceed::new(
                /*expire_count=*/ 1,
                /*status=*/ 404,
                b"".to_vec(),
                vec![],
            ))
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"abc\""))
            .mount(&server)
            .await;

        let stage_info = make_stage_info(
            &server.uri(),
            make_s3_creds("STALE_KEY", "STALE_SECRET", "STALE_TOKEN"),
        );
        let (refresher, calls) =
            CountingRefresher::new(make_s3_creds("FRESH_KEY", "FRESH_SECRET", "FRESH_TOKEN"));

        let prepared = PreparedUpload {
            data: b"hello world".to_vec(),
            digest: "deadbeef".to_string(),
            encryption_metadata: None,
        };

        // overwrite=false so the HEAD exists-check actually runs.
        let status = upload_to_s3_or_skip(
            prepared,
            &stage_info,
            "file.txt",
            /*overwrite=*/ false,
            Some(refresher),
        )
        .await
        .expect("upload should succeed after HEAD-triggered credential refresh");

        assert_eq!(status, UploadStatus::Uploaded);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "refresher should be invoked exactly once from the HEAD branch"
        );
    }

    #[tokio::test]
    async fn download_retries_and_refreshes_creds_on_expired_token() {
        let server = MockServer::start().await;

        // GET → ExpiredToken on first call, then 200 with an empty body.
        Mock::given(method("GET"))
            .respond_with(ExpireThenSucceed::new(1, 200, b"payload".to_vec(), vec![]))
            .mount(&server)
            .await;

        let stage_info = make_stage_info(
            &server.uri(),
            make_s3_creds("STALE_KEY", "STALE_SECRET", "STALE_TOKEN"),
        );
        let (refresher, calls) =
            CountingRefresher::new(make_s3_creds("FRESH_KEY", "FRESH_SECRET", "FRESH_TOKEN"));

        let (data, _digest, meta) = download_from_s3(&stage_info, "file.txt", Some(refresher))
            .await
            .expect("download should succeed after credential refresh");

        assert_eq!(data, b"payload");
        assert!(meta.is_none());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "exactly one credential refresh should happen"
        );
    }

    #[tokio::test]
    async fn upload_with_non_expired_token_error_does_not_trigger_refresh() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        // Return a 400 with some *other* error code — we must not treat this
        // as ExpiredToken and must not invoke the refresher.
        let other_err = "<?xml version=\"1.0\"?><Error><Code>InvalidRequest</Code><Message>bad</Message></Error>";
        Mock::given(method("PUT"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_string(other_err)
                    .insert_header("Content-Type", "application/xml"),
            )
            .mount(&server)
            .await;

        let stage_info = make_stage_info(
            &server.uri(),
            make_s3_creds("STALE_KEY", "STALE_SECRET", "STALE_TOKEN"),
        );
        let (refresher, calls) =
            CountingRefresher::new(make_s3_creds("F_KEY", "F_SECRET", "F_TOKEN"));

        let prepared = PreparedUpload {
            data: b"x".to_vec(),
            digest: "d".to_string(),
            encryption_metadata: None,
        };
        let err = upload_to_s3_or_skip(prepared, &stage_info, "file.txt", true, Some(refresher))
            .await
            .expect_err("non-ExpiredToken error should propagate");

        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "refresher must not be invoked for non-ExpiredToken errors"
        );
        assert!(matches!(err, UploadFileError::S3Upload { .. }));
        assert!(!is_expired_token_upload_error(&err));
    }
}
