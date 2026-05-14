use super::types::{
    EncryptedFileMetadata, MaterialDescription, PreparedUpload, StageInfo, UploadStatus,
};
use crate::config::retry::{BackoffConfig, Jitter, RetryPolicy};
use snafu::{Location, ResultExt, Snafu};
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

/// S3 user-metadata key storing the base64 SHA-256 of the uploaded
/// object. The AWS SDK strips the `x-amz-meta-` prefix, so the raw
/// `x-amz-meta-sfc-digest` header surfaces here as `sfc-digest`.
const SFC_DIGEST_META_KEY: &str = "sfc-digest";

/// Uploads a file to S3 unless a skip condition holds:
/// - `overwrite=false` and an object exists at the destination key
///   (classic existence check).
/// - `overwrite=true && skip_upload_on_content_match=true` and the
///   remote object's `sfc-digest` already equals the local digest
///   (mirrors Python's `_skip_upload_on_content_match`).
pub async fn upload_to_s3_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
    skip_upload_on_content_match: bool,
) -> Result<UploadStatus, UploadFileError> {
    let s3_client = create_s3_client(stage_info, SNOWFLAKE_UPLOAD_PROVIDER).await?;
    let s3_key = format!("{}{filename}", stage_info.key_prefix);

    let need_head = !overwrite || skip_upload_on_content_match;
    if need_head {
        let remote = head_object(&s3_client, stage_info, &s3_key).await?;
        if let Some(head) = remote {
            if !overwrite {
                tracing::info!("File already exists in S3: {s3_key}");
                return Ok(UploadStatus::Skipped);
            }
            let remote_digest = head.metadata().and_then(|m| m.get(SFC_DIGEST_META_KEY));
            if skip_upload_on_content_match && remote_digest == Some(&prepared.digest) {
                tracing::info!("Remote object {s3_key} already matches local digest, skipping");
                return Ok(UploadStatus::Skipped);
            }
        }
    }

    upload_to_s3(prepared, &s3_client, stage_info, &s3_key).await?;
    Ok(UploadStatus::Uploaded)
}

/// HEADs the remote object. Returns `Some(output)` when the object
/// exists and `None` for a 404. 403 is treated as "unknown, proceed as
/// if absent" — limited temporary credentials may allow PUT but not
/// HEAD; short-circuiting the upload based on an uncertain result would
/// lose data correctness.
async fn head_object(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<Option<aws_sdk_s3::operation::head_object::HeadObjectOutput>, UploadFileError> {
    match s3_client
        .head_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .send()
        .await
    {
        Ok(output) => Ok(Some(output)),
        Err(SdkError::ServiceError(err)) if err.err().is_not_found() => Ok(None),
        Err(SdkError::ServiceError(ref err)) if err.raw().status().as_u16() == 403 => {
            tracing::warn!(
                "Access denied when checking remote object in S3 ({s3_key}), proceeding with upload"
            );
            Ok(None)
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
        .metadata(SFC_DIGEST_META_KEY, &prepared.digest);

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
pub async fn download_from_s3(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, Option<String>, Option<EncryptedFileMetadata>), DownloadFileError> {
    let s3_client = create_s3_client(stage_info, SNOWFLAKE_DOWNLOAD_PROVIDER).await?;
    let s3_key = format!("{}{filename}", stage_info.key_prefix);

    let response = s3_client
        .get_object()
        .bucket(stage_info.bucket.clone())
        .key(&s3_key)
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
}

// --- Unit tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_manager::types::CloudCredentials;
    use crate::sensitive::SensitiveString;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    const LOCAL_DIGEST: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    const OTHER_DIGEST: &str = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";

    fn make_s3_stage(end_point: &str) -> StageInfo {
        StageInfo {
            location_type: super::super::types::LocationType::S3,
            bucket: "my-bucket".to_string(),
            key_prefix: "prefix/".to_string(),
            region: "us-west-2".to_string(),
            creds: CloudCredentials::S3 {
                aws_key_id: "AKIAFAKE".to_string(),
                aws_secret_key: SensitiveString::from("secret"),
                aws_token: SensitiveString::from("token"),
            },
            end_point: Some(end_point.to_string()),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            storage_account: None,
        }
    }

    fn prepared(digest: &str) -> PreparedUpload {
        PreparedUpload {
            data: b"hello".to_vec(),
            digest: digest.to_string(),
            encryption_metadata: None,
        }
    }

    /// Counts how many times each HTTP method hit the mock, so we can
    /// assert "HEAD only, no PUT" or "HEAD + PUT" outcomes.
    #[derive(Clone, Default)]
    struct MethodCounter {
        head: Arc<AtomicUsize>,
        put: Arc<AtomicUsize>,
    }

    impl Respond for MethodCounter {
        fn respond(&self, req: &Request) -> ResponseTemplate {
            match req.method.as_str() {
                "HEAD" => {
                    self.head.fetch_add(1, Ordering::SeqCst);
                }
                "PUT" => {
                    self.put.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
            ResponseTemplate::new(200)
        }
    }

    async fn head_200(server: &MockServer, sfc_digest: Option<&str>) {
        let mut tpl = ResponseTemplate::new(200);
        if let Some(d) = sfc_digest {
            tpl = tpl.insert_header("x-amz-meta-sfc-digest", d);
        }
        Mock::given(method("HEAD"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(tpl)
            .mount(server)
            .await;
    }

    async fn head_404(server: &MockServer) {
        Mock::given(method("HEAD"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(ResponseTemplate::new(404))
            .mount(server)
            .await;
    }

    async fn put_200(server: &MockServer) {
        Mock::given(method("PUT"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(ResponseTemplate::new(200))
            .mount(server)
            .await;
    }

    async fn expect_put_carries_digest(server: &MockServer, digest: &str) {
        Mock::given(method("PUT"))
            .and(path("/my-bucket/prefix/file.csv"))
            .and(header("x-amz-meta-sfc-digest", digest))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn uploads_when_overwrite_true_and_remote_absent() {
        let server = MockServer::start().await;
        head_404(&server).await;
        expect_put_carries_digest(&server, LOCAL_DIGEST).await;

        let stage = make_s3_stage(&server.uri());
        let status = upload_to_s3_or_skip(prepared(LOCAL_DIGEST), &stage, "file.csv", true, true)
            .await
            .expect("upload should succeed");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    #[tokio::test]
    async fn skips_when_overwrite_true_and_remote_digest_matches() {
        let server = MockServer::start().await;
        head_200(&server, Some(LOCAL_DIGEST)).await;
        // No PUT mock mounted: if the code attempts to upload, the test
        // fails with an unmatched-request error.
        let stage = make_s3_stage(&server.uri());
        let status = upload_to_s3_or_skip(prepared(LOCAL_DIGEST), &stage, "file.csv", true, true)
            .await
            .expect("skip path should succeed");
        assert_eq!(status, UploadStatus::Skipped);
    }

    #[tokio::test]
    async fn uploads_when_overwrite_true_and_remote_digest_differs() {
        let server = MockServer::start().await;
        head_200(&server, Some(OTHER_DIGEST)).await;
        expect_put_carries_digest(&server, LOCAL_DIGEST).await;

        let stage = make_s3_stage(&server.uri());
        let status = upload_to_s3_or_skip(prepared(LOCAL_DIGEST), &stage, "file.csv", true, true)
            .await
            .expect("upload should succeed");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    #[tokio::test]
    async fn uploads_when_overwrite_true_and_remote_digest_missing() {
        let server = MockServer::start().await;
        head_200(&server, None).await;
        expect_put_carries_digest(&server, LOCAL_DIGEST).await;

        let stage = make_s3_stage(&server.uri());
        let status = upload_to_s3_or_skip(prepared(LOCAL_DIGEST), &stage, "file.csv", true, true)
            .await
            .expect("upload should succeed");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    #[tokio::test]
    async fn always_uploads_when_overwrite_true_and_skip_on_match_disabled() {
        // ODBC preset: even when the remote digest matches, always
        // re-upload and skip the HEAD entirely.
        let server = MockServer::start().await;
        let counter = MethodCounter::default();
        Mock::given(path("/my-bucket/prefix/file.csv"))
            .respond_with(counter.clone())
            .mount(&server)
            .await;

        let stage = make_s3_stage(&server.uri());
        let status = upload_to_s3_or_skip(prepared(LOCAL_DIGEST), &stage, "file.csv", true, false)
            .await
            .expect("upload should succeed");
        assert_eq!(status, UploadStatus::Uploaded);
        assert_eq!(counter.head.load(Ordering::SeqCst), 0, "no HEAD expected");
        assert_eq!(counter.put.load(Ordering::SeqCst), 1, "one PUT expected");
    }

    #[tokio::test]
    async fn skips_when_overwrite_false_and_remote_exists() {
        // Classic existence-skip: digest isn't consulted.
        let server = MockServer::start().await;
        head_200(&server, Some(OTHER_DIGEST)).await;
        // No PUT mock — upload would fail if attempted.
        let stage = make_s3_stage(&server.uri());
        let status = upload_to_s3_or_skip(prepared(LOCAL_DIGEST), &stage, "file.csv", false, true)
            .await
            .expect("skip path should succeed");
        assert_eq!(status, UploadStatus::Skipped);
    }

    #[tokio::test]
    async fn uploads_when_overwrite_false_and_remote_absent() {
        let server = MockServer::start().await;
        head_404(&server).await;
        put_200(&server).await;

        let stage = make_s3_stage(&server.uri());
        let status = upload_to_s3_or_skip(prepared(LOCAL_DIGEST), &stage, "file.csv", false, true)
            .await
            .expect("upload should succeed");
        assert_eq!(status, UploadStatus::Uploaded);
    }

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
}
