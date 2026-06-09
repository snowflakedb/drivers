use super::types::{
    CloudCredentials, DownloadResponse, EncryptedFileMetadata, MaterialDescription, PreparedUpload,
    StageInfo, StageInfoRefreshError, StageInfoRefresher, UploadStatus,
};
use crate::config::retry::{BackoffConfig, Jitter, RetryPolicy};
use crate::refresh::{Refresher, execute_with_refresh};
use snafu::{IntoError, Location, ResultExt, Snafu};
use std::marker::PhantomData;
use std::time::Duration;

// AWS SDK imports
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::config::retry::RetryConfig as AwsRetryConfig;
use aws_sdk_s3::config::timeout::TimeoutConfig as AwsTimeoutConfig;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};

const SNOWFLAKE_UPLOAD_PROVIDER: &str = "snowflake-upload";
const SNOWFLAKE_DOWNLOAD_PROVIDER: &str = "snowflake-download";
const CONTENT_TYPE_OCTET_STREAM: &str = "application/octet-stream";

/// Per-attempt HTTP timeout applied to every S3 SDK operation.
///
/// Matches the Azure/GCS transfer timeout (300s). The retry budget
/// (`max_elapsed` in `s3_retry_policy`) must exceed this so at least one
/// full attempt can complete.
const REQUEST_TIMEOUT_SECS: u64 = 300;

// TODO: streaming instead of loading the whole file into memory

/// Uploads a file to S3, skipping if it already exists and `overwrite` is false.
///
/// On AWS `ExpiredToken` the `refresher` (if any) is invoked to fetch fresh
/// STS credentials, which it writes into the shared `StageInfoCache`; the
/// upload then retries with the new creds. The refresher is responsible for
/// coalescing rapid-fire calls (the production implementation caches a
/// successful refresh for 10 minutes, matching ODBC's `m_lastRefreshTokenSec`
/// gate). The refreshed credentials are visible to other files in the batch
/// via the shared cache — no return-value plumbing required.
///
/// S3 callers only project `creds` out of `StageInfoSnapshot`; the GCS-only
/// `presigned_url` / `presigned_urls` fields are ignored here.
///
/// When `refresher` is `Some`, the retry loop is driven by
/// [`crate::refresh::execute_with_refresh`] via the [`S3StsRefresher`]
/// implementation in this module.
pub async fn upload_to_s3_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadStatus, UploadFileError> {
    let s3_key = format!("{}{filename}", stage_info.key_prefix);

    let attempt = |creds: CloudCredentials| {
        let prepared = prepared.clone();
        let stage_info = with_creds(stage_info, creds);
        let s3_key = s3_key.clone();
        async move {
            let s3_client = create_s3_client(&stage_info, SNOWFLAKE_UPLOAD_PROVIDER)
                .await
                .map_err(|e| S3AttemptError::Other(UploadFileError::from(e)))?;

            if !overwrite
                && check_if_file_exists(&s3_client, &stage_info, &s3_key)
                    .await
                    .map_err(S3AttemptError::Other)?
            {
                tracing::info!("File already exists in S3: {}", s3_key);
                return Ok(UploadStatus::Skipped);
            }

            put_object(prepared, &s3_client, &stage_info, &s3_key).await?;
            Ok(UploadStatus::Uploaded)
        }
    };

    run_s3_with_sts_refresh(
        refresher,
        &stage_info.creds,
        |e| upload_file_error::StageInfoRefreshFailedSnafu.into_error(e),
        // Refresher declined to rotate (or there was none). Surface the
        // original AWS error as a normal upload failure — same shape as
        // any other S3 PUT error.
        |aws_err| upload_file_error::S3UploadSnafu.into_error(aws_err),
        attempt,
    )
    .await
}

/// Internal error type for one attempt of an S3 operation. The `StsExpired`
/// arm is the recoverable signal `should_refresh` matches on; everything
/// else lives in `Other`. This stays internal — `UploadFileError` /
/// `DownloadFileError` have no `StsExpiredToken` variant, so callers cannot
/// observe a refresh-internal state.
#[derive(Debug)]
enum S3AttemptError<E> {
    StsExpired(aws_sdk_s3::Error),
    Other(E),
}

/// S3 STS implementation of the generic [`Refresher`] trait. Drives the
/// retry loop in `execute_with_refresh` by reading credentials from a
/// `StageInfoRefresher`'s shared cache and asking it to rotate when the
/// AWS SDK reports `ExpiredToken`.
///
/// `map_refresh_err` keeps snafu's source-location stamping at the call
/// site: each operation translates `StageInfoRefreshError` into its own
/// error variant locally rather than via a blanket
/// `From<StageInfoRefreshError>` impl on `UploadFileError` /
/// `DownloadFileError`, which would lose location info.
///
/// Tracks the last AWS key id handed out so a refresh that doesn't
/// actually rotate (refresher inside its coalescing window) reports
/// `Ok(false)` and the helper propagates the original error rather than
/// spinning.
struct S3StsRefresher<'a, E, W> {
    refresher: &'a mut dyn StageInfoRefresher,
    last_seen_key: Option<String>,
    map_refresh_err: W,
    _marker: PhantomData<fn() -> E>,
}

impl<'a, E, W> S3StsRefresher<'a, E, W>
where
    W: Fn(StageInfoRefreshError) -> E,
{
    fn new(
        refresher: &'a mut dyn StageInfoRefresher,
        initial: &CloudCredentials,
        map_refresh_err: W,
    ) -> Self {
        Self {
            refresher,
            last_seen_key: aws_key_id(initial).map(str::to_string),
            map_refresh_err,
            _marker: PhantomData,
        }
    }
}

impl<'a, E, W> Refresher<CloudCredentials, S3AttemptError<E>> for S3StsRefresher<'a, E, W>
where
    E: Send,
    W: Fn(StageInfoRefreshError) -> E + Send,
{
    fn current(
        &mut self,
    ) -> crate::refresh::RefreshFuture<'_, Result<CloudCredentials, S3AttemptError<E>>> {
        // S3 only cares about creds — project them out of the broader snapshot.
        let creds = self.refresher.cache().snapshot().creds;
        Box::pin(async move { Ok(creds) })
    }

    fn should_refresh(&self, err: &S3AttemptError<E>) -> bool {
        matches!(err, S3AttemptError::StsExpired(_))
    }

    fn refresh(&mut self) -> crate::refresh::RefreshFuture<'_, Result<bool, S3AttemptError<E>>> {
        Box::pin(async move {
            tracing::info!("S3 hit ExpiredToken; refreshing stage credentials");
            self.refresher
                .refresh()
                .await
                .map_err(|e| S3AttemptError::Other((self.map_refresh_err)(e)))?;
            let new = self.refresher.cache().snapshot().creds;
            let new_key = aws_key_id(&new).map(str::to_string);
            if new_key == self.last_seen_key {
                // Refresher coalesced or returned the same creds — retrying
                // would loop, so decline further rotations.
                return Ok(false);
            }
            self.last_seen_key = new_key;
            Ok(true)
        })
    }
}

/// Runs `attempt` once (no refresher) or in a refresh-retry loop (with
/// refresher), folding `S3AttemptError<E>` back to `E` at the boundary so
/// callers see a uniform error type. With no refresher, an `StsExpired`
/// outcome surfaces as the caller-supplied AWS error path — same shape as
/// any other S3 PUT/GET error.
///
/// `map_refresh_err` / `map_sts_err` keep the call sites' snafu
/// instrumentation at the boundary so source locations land on the
/// operation's own error variants rather than on this helper.
async fn run_s3_with_sts_refresh<F, Fut, T, E>(
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
    initial_creds: &CloudCredentials,
    map_refresh_err: impl Fn(StageInfoRefreshError) -> E + Send,
    map_sts_err: impl FnOnce(aws_sdk_s3::Error) -> E,
    attempt: F,
) -> Result<T, E>
where
    F: Fn(CloudCredentials) -> Fut,
    Fut: Future<Output = Result<T, S3AttemptError<E>>>,
    E: Send,
{
    let outcome = match refresher.as_deref_mut() {
        Some(r) => {
            let mut sts_refresher = S3StsRefresher::new(r, initial_creds, map_refresh_err);
            execute_with_refresh(&mut sts_refresher, attempt).await
        }
        None => attempt(initial_creds.clone()).await,
    };
    outcome.map_err(|e| match e {
        S3AttemptError::Other(err) => err,
        S3AttemptError::StsExpired(aws_err) => map_sts_err(aws_err),
    })
}

/// Returns the AWS key id from S3 credentials, or None for non-S3 variants.
/// Used as the rotation marker; a different key id implies a fresh STS
/// rotation from GS.
fn aws_key_id(creds: &CloudCredentials) -> Option<&str> {
    match creds {
        CloudCredentials::S3 { aws_key_id, .. } => Some(aws_key_id.as_str()),
        _ => None,
    }
}

/// Returns a copy of `stage_info` with `creds` replaced. Bucket/region/
/// key_prefix are immutable for the lifetime of one PUT/GET command.
fn with_creds(stage_info: &StageInfo, creds: CloudCredentials) -> StageInfo {
    let mut info = stage_info.clone();
    info.creds = creds;
    info
}

/// Returns true if the file exists in S3, false if it does not.
/// When the check cannot be performed due to 403 Forbidden (limited
/// temporary credentials that allow PUT but not HEAD), returns false
/// so the caller proceeds with upload.
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
        Err(e) => Err(aws_sdk_s3::Error::from(e)).context(upload_file_error::S3HeadSnafu),
    }
}

/// Returns `true` only when S3 surfaced HTTP 400 + `<Code>ExpiredToken</Code>`.
/// Other codes (InvalidToken, AccessDenied, 403, 5xx, throttling) return false
/// so they stay on the normal error path. Matches the Python / ODBC detector.
fn is_expired_token_error(err: &aws_sdk_s3::Error) -> bool {
    use aws_sdk_s3::error::ProvideErrorMetadata;
    err.code() == Some("ExpiredToken")
}

/// Issues the S3 `PutObject` call and folds `ExpiredToken` into the
/// `S3AttemptError::StsExpired` arm so the generic refresh helper can catch it.
///
/// `disable_payload_signing()` skips a redundant full pass over the file:
/// TLS covers transit, and Snowflake verifies the upload via the `sfc-digest`
/// metadata header rather than the SigV4 body hash.
async fn put_object(
    prepared: PreparedUpload,
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<(), S3AttemptError<UploadFileError>> {
    let mut put_object_request = s3_client
        .put_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .body(ByteStream::from(prepared.data))
        .content_type(CONTENT_TYPE_OCTET_STREAM)
        .metadata("sfc-digest", &prepared.digest);

    if let Some(ref enc_meta) = prepared.encryption_metadata {
        let mat_desc = serde_json::to_string(&enc_meta.material_desc)
            .map_err(|e| upload_file_error::SerializationSnafu.into_error(e))
            .map_err(S3AttemptError::Other)?;
        put_object_request = put_object_request
            .metadata("x-amz-iv", &enc_meta.iv)
            .metadata("x-amz-key", &enc_meta.encrypted_key)
            .metadata("x-amz-matdesc", mat_desc);
    }

    tracing::trace!("PUT object request: {:?}", put_object_request);

    match put_object_request
        .customize()
        .disable_payload_signing()
        .send()
        .await
    {
        Ok(res) => {
            tracing::debug!("S3 upload result: {:?}", res);
            Ok(())
        }
        Err(sdk_err) => {
            let aws_err = aws_sdk_s3::Error::from(sdk_err);
            if is_expired_token_error(&aws_err) {
                tracing::warn!("S3 upload failed with ExpiredToken");
                Err(S3AttemptError::StsExpired(aws_err))
            } else {
                Err(S3AttemptError::Other(
                    upload_file_error::S3UploadSnafu.into_error(aws_err),
                ))
            }
        }
    }
}

/// Downloads a file from S3. For SSE stages the encryption metadata headers
/// will be absent and `file_metadata` is `None`. See `upload_to_s3_or_skip`
/// for the `refresher` semantics; refreshed credentials are written into the
/// shared `StageInfoCache` rather than returned.
///
/// `cloud_byte_count` on the returned `DownloadResponse` reflects the
/// on-cloud (pre-decryption) byte count of the blob — taken from the
/// collected body length, which equals the S3 `Content-Length` for
/// non-streamed responses.
pub async fn download_from_s3(
    stage_info: &StageInfo,
    filename: &str,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<DownloadResponse, DownloadFileError> {
    let s3_key = format!("{}{filename}", stage_info.key_prefix);

    let attempt = |creds: CloudCredentials| {
        let stage_info = with_creds(stage_info, creds);
        let s3_key = s3_key.clone();
        async move {
            let s3_client = create_s3_client(&stage_info, SNOWFLAKE_DOWNLOAD_PROVIDER)
                .await
                .map_err(|e| S3AttemptError::Other(DownloadFileError::from(e)))?;
            get_object(&s3_client, &stage_info, &s3_key).await
        }
    };

    let response = run_s3_with_sts_refresh(
        refresher,
        &stage_info.creds,
        |e| download_file_error::StageInfoRefreshFailedSnafu.into_error(e),
        |aws_err| download_file_error::S3DownloadSnafu.into_error(aws_err),
        attempt,
    )
    .await?;

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
    let cloud_byte_count = data.len() as i64;

    Ok(DownloadResponse {
        data,
        digest,
        file_metadata,
        cloud_byte_count,
    })
}

/// Issues the S3 `GetObject` call and folds `ExpiredToken` into the
/// `S3AttemptError::StsExpired` arm so the generic refresh helper can catch it.
async fn get_object(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<aws_sdk_s3::operation::get_object::GetObjectOutput, S3AttemptError<DownloadFileError>> {
    match s3_client
        .get_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .send()
        .await
    {
        Ok(out) => Ok(out),
        Err(sdk_err) => {
            let aws_err = aws_sdk_s3::Error::from(sdk_err);
            if is_expired_token_error(&aws_err) {
                tracing::warn!("S3 download failed with ExpiredToken");
                Err(S3AttemptError::StsExpired(aws_err))
            } else {
                Err(S3AttemptError::Other(
                    download_file_error::S3DownloadSnafu.into_error(aws_err),
                ))
            }
        }
    }
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
/// and retrying is rarely productive — `create_s3_client` is called per
/// operation, so an expired STS token surfaces as a non-retryable 403 and the
/// caller can re-fetch credentials via a new PUT/GET parse.
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
) -> Result<S3Client, S3CredentialError> {
    let super::types::CloudCredentials::S3 {
        ref aws_key_id,
        ref aws_secret_key,
        ref aws_token,
    } = stage_info.creds
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
    if let Some(endpoint_url) = resolve_s3_endpoint(stage_info) {
        tracing::debug!("Using S3 endpoint: {endpoint_url}");
        s3_config = s3_config.endpoint_url(endpoint_url);
    }

    Ok(S3Client::from_conf(s3_config.build()))
}

/// Resolves the explicit S3 endpoint URL to hand to the AWS SDK builder, or
/// `None` to let the SDK derive the endpoint from the region.
///
/// Precedence (matches `snowflake-jdbc` and `libsnowflakeclient`):
/// 1. `stage_info.endpoint` set (FIPS / VPCE / custom): used verbatim, with
///    `https://` prepended if no scheme is present.
/// 2. `stage_info.use_s3_regional_url` set: route to
///    `s3.<region>.amazonaws.com[.cn]`.
/// 3. Neither: `None` — the SDK uses its default endpoint resolver, which
///    handles standard regions, GovCloud, and `cn-*` correctly on its own.
///
/// Extracted as a pure function so callers (and tests) can verify the chosen
/// endpoint without going through `aws_sdk_s3::Config`, which doesn't expose
/// the configured URL.
fn resolve_s3_endpoint(stage_info: &StageInfo) -> Option<String> {
    if let Some(ep) = stage_info.endpoint.as_deref() {
        let endpoint_url = if ep.starts_with("https://") || ep.starts_with("http://") {
            ep.to_string()
        } else {
            format!("https://{ep}")
        };
        return Some(endpoint_url);
    }
    if stage_info.use_s3_regional_url {
        return Some(regional_s3_endpoint(&stage_info.region));
    }
    None
}

/// Builds the S3 regional endpoint URL for a given region. China regions
/// (`cn-*`) use the `amazonaws.com.cn` suffix; everything else uses
/// `amazonaws.com`. Mirrors `getDomainSuffixForRegionalUrl` in
/// snowflake-jdbc's `SnowflakeS3Client`.
fn regional_s3_endpoint(region: &str) -> String {
    let suffix = if region.to_ascii_lowercase().starts_with("cn-") {
        "amazonaws.com.cn"
    } else {
        "amazonaws.com"
    };
    format!("https://s3.{region}.{suffix}")
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
    #[snafu(display("Failed to refresh S3 stage credentials after ExpiredToken"))]
    StageInfoRefreshFailed {
        #[snafu(source(from(StageInfoRefreshError, Box::new)))]
        source: Box<StageInfoRefreshError>,
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
    #[snafu(display("Failed to refresh S3 stage credentials after ExpiredToken"))]
    StageInfoRefreshFailed {
        #[snafu(source(from(StageInfoRefreshError, Box::new)))]
        source: Box<StageInfoRefreshError>,
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

    // --- Regional endpoint construction ---

    #[test]
    fn regional_s3_endpoint_default_suffix() {
        assert_eq!(
            regional_s3_endpoint("us-east-1"),
            "https://s3.us-east-1.amazonaws.com"
        );
    }

    #[test]
    fn regional_s3_endpoint_china_suffix() {
        assert_eq!(
            regional_s3_endpoint("cn-north-1"),
            "https://s3.cn-north-1.amazonaws.com.cn"
        );
    }

    #[test]
    fn regional_s3_endpoint_china_match_is_case_insensitive() {
        // GS could conceivably send the region in upper case; the suffix
        // detection must not depend on case.
        assert_eq!(
            regional_s3_endpoint("CN-NORTH-1"),
            "https://s3.CN-NORTH-1.amazonaws.com.cn"
        );
    }

    #[test]
    fn regional_s3_endpoint_govcloud_uses_default_suffix() {
        // GovCloud regions are still under amazonaws.com (e.g.
        // s3.us-gov-west-1.amazonaws.com); only `cn-*` gets the .cn TLD.
        assert_eq!(
            regional_s3_endpoint("us-gov-west-1"),
            "https://s3.us-gov-west-1.amazonaws.com"
        );
    }

    // --- Endpoint resolution ---
    //
    // Exercises the four cases the AWS SDK can't surface for us because
    // `aws_sdk_s3::Config` does not expose the resolved URL: explicit
    // endpoint, regional flag, neither, and scheme-less endpoint.

    use crate::file_manager::types::LocationType;
    use crate::sensitive::SensitiveString;

    fn s3_stage(endpoint: Option<&str>, use_s3_regional_url: bool) -> StageInfo {
        StageInfo {
            location_type: LocationType::S3,
            bucket: "my-bucket".to_string(),
            key_prefix: "prefix/".to_string(),
            region: "us-east-1".to_string(),
            creds: CloudCredentials::S3 {
                aws_key_id: "k".to_string(),
                aws_secret_key: SensitiveString::from("s"),
                aws_token: SensitiveString::from("t"),
            },
            endpoint: endpoint.map(str::to_string),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url,
            storage_account: None,
        }
    }

    // --- ExpiredToken detector ---
    //
    // Constructing `aws_sdk_s3::Error` with a chosen error code is a little
    // awkward because `Error::Unhandled` has private fields, but any typed
    // variant carries metadata via its builder. `NoSuchKey` is convenient —
    // we pin an arbitrary code onto its `ErrorMetadata` and upcast. This
    // exercises the real `ProvideErrorMetadata::code` path the production
    // detector relies on.

    use aws_sdk_s3::Error as S3Error;
    use aws_sdk_s3::error::ErrorMetadata;
    use aws_sdk_s3::types::error::NoSuchKey;

    fn s3_error_with_code(code: &str) -> S3Error {
        S3Error::NoSuchKey(
            NoSuchKey::builder()
                .meta(ErrorMetadata::builder().code(code).build())
                .build(),
        )
    }

    fn s3_error_without_code() -> S3Error {
        S3Error::NoSuchKey(NoSuchKey::builder().build())
    }

    #[test]
    fn expired_token_code_is_detected() {
        assert!(is_expired_token_error(&s3_error_with_code("ExpiredToken")));
    }

    #[test]
    fn other_aws_codes_are_not_treated_as_expired_token() {
        // These are the close-but-different codes that must NOT trigger an STS
        // refresh. InvalidToken/TokenRefreshRequired mean the creds are bad in
        // a way refreshing won't fix; AccessDenied means policy, not expiry;
        // the others are transient SDK concerns handled by retry, not refresh.
        for code in [
            "InvalidToken",
            "TokenRefreshRequired",
            "AccessDenied",
            "SignatureDoesNotMatch",
            "InvalidAccessKeyId",
            "RequestTimeTooSkewed",
            "SlowDown",
            "InternalError",
            "NoSuchKey",
        ] {
            assert!(
                !is_expired_token_error(&s3_error_with_code(code)),
                "{code} must not trigger STS refresh"
            );
        }
    }

    #[test]
    fn resolve_endpoint_explicit_endpoint_wins_over_regional_flag() {
        // GS-supplied endpoint always wins — FIPS / VPCE / custom must not
        // be silently overridden by the regional flag.
        let stage = s3_stage(Some("https://my-fips.us-east-1.amazonaws.com"), true);
        assert_eq!(
            resolve_s3_endpoint(&stage).as_deref(),
            Some("https://my-fips.us-east-1.amazonaws.com")
        );
    }

    #[test]
    fn resolve_endpoint_uses_regional_when_only_flag_set() {
        let stage = s3_stage(None, true);
        assert_eq!(
            resolve_s3_endpoint(&stage).as_deref(),
            Some("https://s3.us-east-1.amazonaws.com")
        );
    }

    #[test]
    fn resolve_endpoint_returns_none_when_neither_set() {
        // Falls through to the AWS SDK's default endpoint resolver — we
        // must NOT pre-pin an endpoint, otherwise the SDK can't apply its
        // own `cn-*` / GovCloud handling.
        let stage = s3_stage(None, false);
        assert_eq!(resolve_s3_endpoint(&stage), None);
    }

    #[test]
    fn resolve_endpoint_prepends_https_when_scheme_missing() {
        // GS sometimes sends `endPoint` without a scheme (host only).
        // The SDK's `endpoint_url` requires a scheme, so we add `https://`.
        let stage = s3_stage(Some("my-fips.us-east-1.amazonaws.com"), false);
        assert_eq!(
            resolve_s3_endpoint(&stage).as_deref(),
            Some("https://my-fips.us-east-1.amazonaws.com")
        );
    }

    #[test]
    fn resolve_endpoint_preserves_http_scheme() {
        // If GS or a test fixture supplies `http://`, we must not double-
        // prefix or upgrade the scheme.
        let stage = s3_stage(Some("http://localhost:9000"), false);
        assert_eq!(
            resolve_s3_endpoint(&stage).as_deref(),
            Some("http://localhost:9000")
        );
    }

    #[test]
    fn missing_code_is_not_treated_as_expired_token() {
        assert!(!is_expired_token_error(&s3_error_without_code()));
    }

    // --- creds_unchanged short-circuit ---
    //
    // Compared on `aws_key_id`. A different key id implies a fresh STS
    // rotation from GS; same key id means we're inside the refresher's
    // coalescing window and retrying would loop.

    fn s3_creds(key: &str) -> CloudCredentials {
        CloudCredentials::S3 {
            aws_key_id: key.to_string(),
            aws_secret_key: "secret".to_string().into(),
            aws_token: "token".to_string().into(),
        }
    }

    // --- aws_key_id helper used by S3StsRefresher's rotation check ---
    //
    // S3StsRefresher compares AWS key ids before/after refresh: a different
    // key id implies a fresh STS rotation from GS, the same key id means
    // we're inside the refresher's coalescing window and retrying would
    // loop. Non-S3 variants intentionally return None so the comparison
    // never deadlocks the retry on Gcs/Azure stages.

    #[test]
    fn aws_key_id_returns_some_for_s3_creds() {
        assert_eq!(aws_key_id(&s3_creds("AKIA1")), Some("AKIA1"));
    }

    #[test]
    fn aws_key_id_returns_none_for_non_s3_variants() {
        let gcs = CloudCredentials::Gcs {
            gcs_access_token: Some("g".to_string().into()),
        };
        let azure = CloudCredentials::Azure {
            sas_token: "a".to_string().into(),
        };
        assert_eq!(aws_key_id(&gcs), None);
        assert_eq!(aws_key_id(&azure), None);
    }

    // --- S3StsRefresher::refresh ---
    //
    // A fake StageInfoRefresher records call counts and exposes a mutable
    // cache so tests can simulate "refresh rotated the creds" vs "refresh
    // coalesced and returned the same creds".

    use super::super::types::{StageInfoCache, StageInfoRefreshError, StageInfoSnapshot};
    use crate::refresh::Refresher;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    struct FakeRefresher {
        cache: StageInfoCache,
        next_creds: std::sync::Mutex<Option<CloudCredentials>>,
        refresh_calls: AtomicUsize,
    }

    impl FakeRefresher {
        fn new(initial: CloudCredentials) -> Self {
            Self {
                cache: StageInfoCache::new_with_creds(initial),
                next_creds: std::sync::Mutex::new(None),
                refresh_calls: AtomicUsize::new(0),
            }
        }

        /// Set what the cache will hold after the next `refresh()` call.
        fn arm(&self, creds: CloudCredentials) {
            *self.next_creds.lock().unwrap() = Some(creds);
        }
    }

    impl StageInfoRefresher for FakeRefresher {
        fn refresh(&mut self) -> super::super::types::RefreshFuture<'_> {
            self.refresh_calls.fetch_add(1, AtomicOrdering::SeqCst);
            let next = self.next_creds.lock().unwrap().take();
            if let Some(c) = next {
                self.cache.store(StageInfoSnapshot::creds_only(c));
            }
            Box::pin(async { Ok::<(), StageInfoRefreshError>(()) })
        }

        fn refresh_url(&mut self) -> super::super::types::RefreshFuture<'_> {
            // S3 tests never trigger URL refresh; share the same path so the
            // trait is satisfied. Production GCS tests provide a dedicated fake.
            self.refresh()
        }

        fn cache(&self) -> &StageInfoCache {
            &self.cache
        }
    }

    /// Identity `map_refresh_err` for tests that don't care about error
    /// translation — keeps the `StageInfoRefreshError` as-is.
    fn identity_map(e: StageInfoRefreshError) -> StageInfoRefreshError {
        e
    }

    #[tokio::test]
    async fn s3_sts_refresher_refresh_returns_true_when_creds_rotate() {
        let mut fake = FakeRefresher::new(s3_creds("AKIA1"));
        fake.arm(s3_creds("AKIA2"));
        let initial = s3_creds("AKIA1");
        let mut sts_refresher = S3StsRefresher::new(&mut fake, &initial, identity_map);

        let rotated = sts_refresher.refresh().await.unwrap();

        assert!(rotated);
        assert_eq!(fake.refresh_calls.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(
            aws_key_id(&fake.cache().snapshot().creds),
            Some("AKIA2"),
            "cache holds rotated creds"
        );
    }

    #[tokio::test]
    async fn s3_sts_refresher_refresh_returns_false_when_creds_unchanged() {
        // FakeRefresher with no `arm()` leaves the cache holding the
        // initial creds — simulating a hit inside the refresher's
        // coalescing window. The S3StsRefresher must report Ok(false) so
        // the generic helper propagates the original error rather than
        // spinning.
        let mut fake = FakeRefresher::new(s3_creds("AKIA1"));
        let initial = s3_creds("AKIA1");
        let mut sts_refresher = S3StsRefresher::new(&mut fake, &initial, identity_map);

        let rotated = sts_refresher.refresh().await.unwrap();

        assert!(
            !rotated,
            "unchanged creds → S3StsRefresher declines further rotations"
        );
        assert_eq!(fake.refresh_calls.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test]
    async fn s3_sts_refresher_refresh_tracks_last_seen_key_across_rotations() {
        // After a successful rotation, last_seen_key is updated. A
        // subsequent unchanged refresh against the new key should still
        // report Ok(false) — confirming the marker followed the rotation.
        let mut fake = FakeRefresher::new(s3_creds("AKIA1"));
        fake.arm(s3_creds("AKIA2"));
        let initial = s3_creds("AKIA1");
        let mut sts_refresher = S3StsRefresher::new(&mut fake, &initial, identity_map);

        assert!(sts_refresher.refresh().await.unwrap()); // AKIA1 -> AKIA2
        // Cache still holds AKIA2; arming nothing means refresh() leaves
        // it as-is. S3StsRefresher must see "no rotation" against AKIA2
        // (not AKIA1).
        assert!(!sts_refresher.refresh().await.unwrap());
    }

    // The SigV4 payload hash is set inside aws-sigv4 and only appears on the
    // wire, so wiremock at the HTTP layer is the only place it's observable.

    use wiremock::matchers::{header, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn assert_put_sends_unsigned_payload(prepared: PreparedUpload) {
        let mock = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(header("x-amz-content-sha256", "UNSIGNED-PAYLOAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock)
            .await;

        let stage_info = StageInfo {
            location_type: crate::file_manager::types::LocationType::S3,
            bucket: "test-bucket".to_string(),
            key_prefix: "prefix/".to_string(),
            region: "us-east-1".to_string(),
            creds: s3_creds("AKIA-TEST"),
            endpoint: Some(mock.uri()),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            storage_account: None,
        };

        // overwrite=true skips the HEAD probe.
        upload_to_s3_or_skip(prepared, &stage_info, "f.dat", true, &mut None)
            .await
            .expect("upload should succeed against the mock");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn put_object_sends_unsigned_payload_for_unencrypted_upload() {
        assert_put_sends_unsigned_payload(PreparedUpload {
            data: b"hello world".to_vec(),
            digest: "0".repeat(64),
            encryption_metadata: None,
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn put_object_sends_unsigned_payload_for_encrypted_upload() {
        // Most production stage uploads are client-side encrypted, which
        // attaches three extra metadata headers before signing — confirm
        // the override still applies on that path.
        assert_put_sends_unsigned_payload(PreparedUpload {
            data: b"hello world".to_vec(),
            digest: "0".repeat(64),
            encryption_metadata: Some(EncryptedFileMetadata {
                encrypted_key: "k".to_string(),
                iv: "i".to_string(),
                material_desc: MaterialDescription {
                    query_id: "q".to_string(),
                    smk_id: "1".to_string(),
                    key_size: "256".to_string(),
                },
            }),
        })
        .await;
    }
}
