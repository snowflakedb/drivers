use super::cloud_http;
use super::encryption::Encryptor;
use super::multipart::{self, MultipartConfig, MultipartParams};
use super::types::{
    ByteSource, CloudCredentials, EncryptedFileMetadata, MaterialDescription, PreparedUpload,
    StageInfo, StageInfoRefreshError, StageInfoRefresher, UploadStatus,
};
use crate::config::retry::RetryPolicy;
use crate::refresh::{Refresher, execute_with_refresh};
use bytes::Bytes;
use futures::StreamExt as _;
use futures::TryStreamExt as _;
use http_body::Frame;
use http_body_util::StreamBody;
use snafu::{IntoError, Location, ResultExt, Snafu};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tempfile::{NamedTempFile, TempPath};
use tokio_stream::wrappers::ReceiverStream;

// AWS SDK imports
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_s3::config::retry::RetryConfig as AwsRetryConfig;
use aws_sdk_s3::config::timeout::TimeoutConfig as AwsTimeoutConfig;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::types::BucketAccelerateStatus;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};
use aws_smithy_types::body::SdkBody;

const SNOWFLAKE_UPLOAD_PROVIDER: &str = "snowflake-upload";
const SNOWFLAKE_DOWNLOAD_PROVIDER: &str = "snowflake-download";
const CONTENT_TYPE_OCTET_STREAM: &str = "application/octet-stream";

/// S3 Transfer Acceleration global endpoint. AWS routes the request to the
/// nearest edge location, which then forwards over the AWS backbone.
const S3_ACCELERATE_ENDPOINT: &str = "https://s3-accelerate.amazonaws.com";

/// Bucket-name prefix Snowflake uses for internal (managed) stages. These
/// never have transfer acceleration configured, so we skip the probe to
/// avoid an extra HTTP call and a possible 403 against limited stage creds.
const INTERNAL_STAGE_BUCKET_PREFIX: &str = "sfc-";

/// Per-attempt HTTP timeout applied to every S3 SDK operation.
///
/// Matches the Azure/GCS transfer timeout (300s). The retry budget
/// (`max_elapsed` in `s3_retry_policy`) must exceed this so at least one
/// full attempt can complete.
const REQUEST_TIMEOUT_SECS: u64 = 300;

/// Uploads a file to S3, skipping if it already exists and `overwrite` is false.
///
/// Files whose on-cloud size (ciphertext length for CSE, source length for SSE)
/// is at or above `multipart.threshold` take the multipart path
/// ([`s3_multipart_upload`]); smaller files take the single `PutObject` path.
///
/// On AWS `ExpiredToken` the `refresher` (if any) is invoked to fetch fresh
/// STS credentials, which it writes into the shared `StageInfoCache`; the
/// upload then retries with the new creds (the whole multipart upload restarts,
/// after aborting the in-flight one). The refresher is responsible for
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
pub(super) async fn upload_to_s3_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
    base_policy: &RetryPolicy,
    multipart: MultipartParams,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadStatus, UploadFileError> {
    let s3_key = format!("{}{filename}", stage_info.key_prefix);
    let policy = s3_retry_policy(base_policy);

    // The on-cloud byte count (ciphertext length for CSE, source length for
    // SSE) decides single-PUT vs multipart. Computed once, outside the
    // per-attempt closure, so a stat isn't re-run on every retry.
    let body_len = multipart::upload_body_len(&prepared).await.map_err(|e| {
        upload_file_error::SourceOpenSnafu {
            detail: e.to_string(),
        }
        .build()
    })?;
    // `>=` matches the Python connector boundary (`upload_size >= threshold`).
    let use_multipart = body_len >= multipart.threshold.bytes();

    let attempt = |creds: CloudCredentials| {
        let prepared = prepared.clone();
        let stage_info = with_creds(stage_info, creds);
        let s3_key = s3_key.clone();
        let policy = policy.clone();
        async move {
            let s3_client = create_s3_client(&stage_info, SNOWFLAKE_UPLOAD_PROVIDER, &policy)
                .await
                .map_err(|e| S3AttemptError::Other(UploadFileError::from(e)))?;

            if !overwrite
                && check_if_file_exists(&s3_client, &stage_info, &s3_key)
                    .await
                    .map_err(S3AttemptError::Other)?
            {
                tracing::info!("File already exists in S3: {:?}", s3_key);
                return Ok(UploadStatus::Skipped);
            }

            if use_multipart {
                s3_multipart_upload(
                    prepared,
                    &s3_client,
                    &stage_info,
                    &s3_key,
                    body_len,
                    multipart.concurrency,
                )
                .await?;
            } else {
                put_object(prepared, &s3_client, &stage_info, &s3_key).await?;
            }
            Ok(UploadStatus::Uploaded)
        }
    };

    run_s3_with_sts_refresh(
        refresher,
        &stage_info.creds,
        |e| upload_file_error::StageInfoRefreshSnafu.into_error(e),
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
                "Access denied when checking if file exists in S3 ({s3_key:?}), proceeding with upload"
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

pin_project_lite::pin_project! {
    /// Wraps a streaming body to advertise an exact `Content-Length`. The AWS
    /// SDK's checksum interceptor rejects a streaming request body of unknown
    /// size (`UnsizedRequestBody`); the file-path `ByteStream` avoids this
    /// because it reports the file length. The encrypting stream's length is
    /// known analytically (`Encryptor::cipher_len`), so we surface it here.
    struct SizedBody<B> {
        #[pin]
        inner: B,
        len: u64,
    }
}

impl<B> http_body::Body for SizedBody<B>
where
    B: http_body::Body<Data = Bytes>,
{
    type Data = Bytes;
    type Error = B::Error;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        self.project().inner.poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::with_exact(self.len)
    }
}

/// Builds a retryable S3 upload body that lazily AES-CBC-encrypts `source`
/// (see `EncryptingReader`); ciphertext is never materialized. Wrapped in
/// [`SdkBody::retryable`] so the SDK's internal retries can replay the body —
/// re-encryption with the fixed key+IV is deterministic, so the advertised
/// `Content-Length` (`SizedBody`) and `sfc-digest` stay valid.
fn encrypting_byte_stream(source: ByteSource, encryptor: Encryptor) -> ByteStream {
    let len = encryptor.cipher_len() as u64;
    let body = SdkBody::retryable(move || {
        // Defer `source.open()` into the encrypting stream's `spawn_blocking`
        // task so the open() syscall runs on the blocking pool, not the tokio
        // runtime thread — a slow or hung open on a networked FS (NFS/EBS, in
        // scope for CSE) must not stall a runtime worker. `SdkBody::retryable`
        // takes a sync `Fn`, so the async-open approach used for GCS/Azure can't
        // apply here; instead the open runs as the first step inside the task,
        // and an open failure surfaces as the stream's first (error) frame —
        // before any body bytes — so the SDK fails this attempt cleanly and
        // retries per policy. `source`/`encryptor` are re-cloned per retry so
        // each rebuilt body re-opens and re-encrypts deterministically.
        let source = source.clone();
        let encryptor = encryptor.clone();
        let stream = cloud_http::encrypting_body_stream(move || source.open(), encryptor)
            .map_ok(Frame::data);
        SdkBody::from_body_1_x(SizedBody {
            inner: StreamBody::new(stream),
            len,
        })
    });
    ByteStream::new(body)
}

/// Issues the S3 `PutObject` call and folds `ExpiredToken` into the
/// `S3AttemptError::StsExpired` arm so the generic refresh helper can catch it.
async fn put_object(
    prepared: PreparedUpload,
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<(), S3AttemptError<UploadFileError>> {
    // CSE params (cloud metadata + encryptor) are both present or both absent.
    let (encryption_metadata, encryptor) = prepared.cse.map(|c| (c.metadata, c.encryptor)).unzip();

    // Ciphertext length for CSE (analytic), so `content_length` can be set
    // before the streaming body is read.
    let content_length = encryptor.as_ref().map(|e| e.cipher_len());

    // `prepared` is held until this fn returns, so a gzip-tempfile guard inside
    // `prepared.source` outlives the SDK send (and its internal retries).
    let body = match (prepared.source.byte_source(), encryptor) {
        // CSE: lazy AES-CBC encrypting stream, retryable so SDK-internal retries
        // can replay the body (re-encryption is deterministic).
        (source, Some(encryptor)) => encrypting_byte_stream(source, encryptor),
        // SSE Path: hand the SDK the file directly (FsBuilder is retryable).
        (ByteSource::Path(ref path), None) => ByteStream::read_from()
            .path(path)
            .build()
            .await
            .map_err(|e| {
                S3AttemptError::Other(
                    upload_file_error::SourceOpenSnafu {
                        detail: e.to_string(),
                    }
                    .build(),
                )
            })?,
        // SSE in-memory (small / passthrough payloads).
        (ByteSource::Bytes(bytes), None) => ByteStream::from(bytes),
    };

    let mut put_object_request = s3_client
        .put_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .body(body)
        .set_content_length(content_length)
        .content_type(CONTENT_TYPE_OCTET_STREAM)
        .metadata("sfc-digest", &prepared.digest);

    if let Some(ref enc_meta) = encryption_metadata {
        let mat_desc = serde_json::to_string(&enc_meta.material_desc)
            .context(upload_file_error::SerializationSnafu)
            .map_err(S3AttemptError::Other)?;
        put_object_request = put_object_request
            .metadata("x-amz-iv", &enc_meta.iv)
            .metadata("x-amz-key", &enc_meta.encrypted_key)
            .metadata("x-amz-matdesc", mat_desc);
    }

    // NB: do NOT `{:?}` the whole request — its `.body(ByteStream)` Debug-
    // expands the full file payload (~12 MB/file). When an in-band telemetry
    // exporter is attached, that string is captured per file and accumulated
    // into the telemetry payload, which then OOMs serializing a multi-GB JSON
    // blob on a multi-file PUT (SNOW-3240509-adjacent; perf 12mx100 exit 137).
    // Log only safe metadata.
    tracing::trace!(bucket = %stage_info.bucket, key = ?s3_key, "Sending S3 PutObject request");

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

/// Uploads `prepared` to S3 with the multipart protocol:
/// `CreateMultipartUpload` → parallel `UploadPart` ×N → `CompleteMultipartUpload`.
///
/// **Abort discipline:** once the upload is created, *any* subsequent failure
/// (a part upload, the completion call, or a read error from the source) aborts
/// the multipart upload before the error propagates, so partially-uploaded
/// parts don't linger as billable orphans. This is the fix for the
/// libsnowflakeclient `// TODO abort existing upload` gap. An abort that itself
/// fails is logged but never masks the original error. Encryption metadata is
/// attached to `CreateMultipartUpload` only — the S3 API rejects per-object
/// metadata on `UploadPart`, matching the Python/JDBC/ODBC connectors.
async fn s3_multipart_upload(
    prepared: PreparedUpload,
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
    body_len: u64,
    concurrency: usize,
) -> Result<(), S3AttemptError<UploadFileError>> {
    let chunk_size = multipart::compute_part_size(body_len, &MultipartConfig::S3)
        .context(upload_file_error::FileTooLargeSnafu)
        .map_err(S3AttemptError::Other)?;

    let upload_id = s3_create_multipart_upload(&prepared, s3_client, stage_info, s3_key).await?;
    tracing::debug!(
        "S3 multipart upload started: key={s3_key:?} upload_id={upload_id:?} \
         body_len={body_len} chunk_size={chunk_size} concurrency={concurrency}"
    );

    // Parts read sequentially from the (optionally encrypting) source, uploaded
    // concurrently. `prepared.source`'s gzip-tempfile guard (if any) is moved
    // into the part-reader's `ByteSource`, so the lazily-read body stays valid.
    let source = prepared.source.byte_source();
    let encryptor = prepared.cse.map(|c| c.encryptor);
    let parts_rx =
        multipart::spawn_part_reader(source, encryptor, chunk_size as usize, concurrency);

    let outcome = upload_parts_and_complete(
        s3_client,
        stage_info,
        s3_key,
        &upload_id,
        parts_rx,
        concurrency,
    )
    .await;

    // Abort on every observable failure so parts don't orphan. When the
    // failure is an expired token, this abort call uses the same expired
    // client and will itself fail (logged, not fatal); the STS-refresh
    // retry then re-creates the upload with fresh creds, and a bucket
    // lifecycle rule reaps the orphaned parts from the aborted attempt.
    //
    // TODO: if cancellation is ever added to the driver, this abort also
    // needs to fire on drop (e.g. via a Drop-based AbortGuard).
    if outcome.is_err() {
        s3_abort_multipart_upload(s3_client, stage_info, s3_key, &upload_id).await;
    }
    outcome
}

/// Issues `CreateMultipartUpload` with the file metadata (digest + CSE
/// headers), returning the upload id. Folds `ExpiredToken` into `StsExpired`.
async fn s3_create_multipart_upload(
    prepared: &PreparedUpload,
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<String, S3AttemptError<UploadFileError>> {
    let mut request = s3_client
        .create_multipart_upload()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .content_type(CONTENT_TYPE_OCTET_STREAM)
        .metadata("sfc-digest", &prepared.digest);

    if let Some(enc_meta) = prepared.cse.as_ref().map(|c| &c.metadata) {
        let mat_desc = serde_json::to_string(&enc_meta.material_desc)
            .context(upload_file_error::SerializationSnafu)
            .map_err(S3AttemptError::Other)?;
        request = request
            .metadata("x-amz-iv", &enc_meta.iv)
            .metadata("x-amz-key", &enc_meta.encrypted_key)
            .metadata("x-amz-matdesc", mat_desc);
    }

    tracing::info!(
        method = "POST",
        bucket = %stage_info.bucket,
        key = ?s3_key,
        "S3 CreateMultipartUpload"
    );
    match request.send().await {
        Ok(out) => {
            tracing::debug!(bucket = %stage_info.bucket, key = ?s3_key, "S3 CreateMultipartUpload succeeded");
            out.upload_id().map(str::to_string).ok_or_else(|| {
                S3AttemptError::Other(
                    upload_file_error::S3MultipartCreateSnafu {
                        detail: "CreateMultipartUpload response had no upload id".to_string(),
                    }
                    .build(),
                )
            })
        }
        Err(sdk_err) => {
            tracing::warn!(
                cause = std::any::type_name_of_val(&sdk_err),
                bucket = %stage_info.bucket,
                key = ?s3_key,
                "S3 CreateMultipartUpload failed"
            );
            Err(map_s3_error(aws_sdk_s3::Error::from(sdk_err), |aws_err| {
                upload_file_error::S3MultipartCreateSnafu {
                    detail: aws_err.to_string(),
                }
                .build()
            }))
        }
    }
}

/// Drives the parallel `UploadPart` phase and, on success, commits with
/// `CompleteMultipartUpload`. Parts upload up to `concurrency` at a time via
/// `buffer_unordered`; the first failure short-circuits (the caller then
/// aborts). Completed parts are sorted by part number before the commit, since
/// `buffer_unordered` yields them out of order.
async fn upload_parts_and_complete(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
    upload_id: &str,
    parts_rx: tokio::sync::mpsc::Receiver<std::io::Result<multipart::UploadPart>>,
    concurrency: usize,
) -> Result<(), S3AttemptError<UploadFileError>> {
    let mut completed: Vec<CompletedPart> = ReceiverStream::new(parts_rx)
        .map(|part| async move {
            let part = part.map_err(|e| {
                S3AttemptError::Other(
                    upload_file_error::SourceReadSnafu {
                        detail: e.to_string(),
                    }
                    .build(),
                )
            })?;
            upload_one_part(s3_client, stage_info, s3_key, upload_id, part).await
        })
        .buffer_unordered(concurrency)
        .try_collect()
        .await?;

    // A multipart upload with zero parts makes S3 reject CompleteMultipartUpload
    // with an opaque `MalformedXML`; surface a clear error instead. Unreachable on
    // the normal path (multipart requires `body_len >= threshold >= 1`) — this
    // guards a source truncated to 0 bytes between the size stat and the first read.
    if completed.is_empty() {
        return Err(S3AttemptError::Other(
            upload_file_error::SourceReadSnafu {
                detail: "no upload parts produced (source became empty before read)".to_string(),
            }
            .build(),
        ));
    }

    // S3 requires the completed-part list in ascending part-number order.
    completed.sort_by_key(|p| p.part_number());

    let completed_upload = CompletedMultipartUpload::builder()
        .set_parts(Some(completed))
        .build();

    tracing::info!(
        method = "POST",
        bucket = %stage_info.bucket,
        key = ?s3_key,
        upload_id = ?upload_id,
        "S3 CompleteMultipartUpload"
    );
    match s3_client
        .complete_multipart_upload()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .upload_id(upload_id)
        .multipart_upload(completed_upload)
        .send()
        .await
    {
        Ok(res) => {
            tracing::debug!("S3 CompleteMultipartUpload succeeded: {:?}", res);
            Ok(())
        }
        Err(sdk_err) => Err(map_s3_error(aws_sdk_s3::Error::from(sdk_err), |aws_err| {
            upload_file_error::S3MultipartCompleteSnafu {
                detail: aws_err.to_string(),
            }
            .build()
        })),
    }
}

/// Uploads a single part and returns its `CompletedPart` (etag + number).
/// Per-part bodies carry no metadata and disable payload signing, mirroring
/// the single-PUT path and the reference connectors.
async fn upload_one_part(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
    upload_id: &str,
    part: multipart::UploadPart,
) -> Result<CompletedPart, S3AttemptError<UploadFileError>> {
    let part_number = part.number;
    let content_length = part.body.len() as i64;
    let body = ByteStream::new(aws_smithy_types::body::SdkBody::from(part.body));

    tracing::info!(
        method = "PUT",
        bucket = %stage_info.bucket,
        key = ?s3_key,
        part_number,
        "S3 UploadPart"
    );
    match s3_client
        .upload_part()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .upload_id(upload_id)
        .part_number(part_number)
        .body(body)
        .set_content_length(Some(content_length))
        .customize()
        .disable_payload_signing()
        .send()
        .await
    {
        Ok(out) => Ok(CompletedPart::builder()
            .set_e_tag(out.e_tag().map(str::to_string))
            .part_number(part_number)
            .build()),
        Err(sdk_err) => Err(map_s3_error(
            aws_sdk_s3::Error::from(sdk_err),
            move |aws_err| {
                upload_file_error::S3UploadPartSnafu {
                    part_number,
                    detail: aws_err.to_string(),
                }
                .build()
            },
        )),
    }
}

/// Best-effort `AbortMultipartUpload`. Logs (never returns) failures so the
/// original error that triggered the abort is the one that propagates.
async fn s3_abort_multipart_upload(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
    upload_id: &str,
) {
    tracing::info!(
        method = "DELETE",
        bucket = %stage_info.bucket,
        key = ?s3_key,
        upload_id = ?upload_id,
        "S3 AbortMultipartUpload"
    );
    match s3_client
        .abort_multipart_upload()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .upload_id(upload_id)
        .send()
        .await
    {
        Ok(_) => {
            tracing::debug!("Aborted S3 multipart upload: key={s3_key:?} upload_id={upload_id:?}")
        }
        Err(e) => {
            tracing::error!(
                cause = std::any::type_name_of_val(&e),
                key = ?s3_key,
                upload_id = ?upload_id,
                "Failed to abort S3 multipart upload; \
                 orphaned parts may incur storage cost until a lifecycle rule reaps them"
            );
            tracing::debug!("S3 abort multipart upload failure detail: {e}");
        }
    }
}

/// Folds an AWS S3 error into an [`S3AttemptError`]: an expired STS token becomes
/// `StsExpired` (so the refresh loop rotates creds and retries), anything else is
/// wrapped by `wrap` into the caller's error type. Shared by the upload and
/// download paths.
fn map_s3_error<E>(
    aws_err: aws_sdk_s3::Error,
    wrap: impl FnOnce(aws_sdk_s3::Error) -> E,
) -> S3AttemptError<E> {
    if is_expired_token_error(&aws_err) {
        S3AttemptError::StsExpired(aws_err)
    } else {
        S3AttemptError::Other(wrap(aws_err))
    }
}

/// Downloaded S3 ciphertext plus the metadata the decrypt step needs. The
/// body is either buffered in memory (single GET, below the multipart
/// threshold) or spilled to a tempfile by parallel ranged GETs (above it),
/// so large downloads never hold the whole blob in heap.
pub(super) struct S3Download {
    pub(super) body: S3DownloadBody,
    pub(super) digest: Option<String>,
    pub(super) file_metadata: Option<EncryptedFileMetadata>,
    /// On-cloud (pre-decryption) byte count, from the HEAD `Content-Length`.
    pub(super) cloud_byte_count: i64,
}

// TODO(SNOW-3406377): the ranged-download assembly loop is duplicated across S3,
// Azure, and GCS. It is extracted into a shared cloud_http::assemble_ranged_download
// helper — and S3's SpilledBody / SpillTarget migrated onto the cloud_http types
// (CloudSpillTarget, CloudSpilledBody) that Azure and GCS already use — in a
// dedicated follow-up PR stacked on this one. See that PR for the extraction.

/// Where the downloaded ciphertext lives. `into_reader` yields a uniform
/// blocking `Read` over either shape for the decrypt/copy step.
pub(super) enum S3DownloadBody {
    InMemory(Bytes),
    Spilled(SpilledBody),
}

/// A ranged download assembled to disk. The two shapes differ only in who owns
/// the file and how it is finalized:
///
/// * `Part` — a non-encrypted download assembled straight into the caller's
///   `<dst>.part` staging file. The bytes are already the final plaintext, so
///   the caller just renames `.part` to the destination (a single same-FS
///   rename). Any leftover after a hard kill is a self-documenting,
///   self-overwriting `.part`, never random debris.
/// * `Temp` — a client-side-encrypted (or git-stage) download assembled into a
///   throwaway RAII temp. CSE bytes are ciphertext that still has to be
///   decrypted into `.part`, so they cannot land in `.part` directly; the temp
///   is unlinked on drop once consumed.
pub(super) enum SpilledBody {
    Part(PathBuf),
    Temp(TempPath),
}

/// Where a ranged download should assemble its bytes. Chosen by the caller
/// (which knows whether the object is client-side-encrypted) and threaded down
/// to [`s3_range_download`]. `Copy` so it can be handed to each STS-refresh
/// retry of the download closure.
#[derive(Clone, Copy)]
pub(super) enum SpillTarget<'a> {
    /// Non-encrypted download: assemble directly into this `<dst>.part` file.
    Part(&'a Path),
    /// Encrypted / git-stage download: assemble ciphertext into a temp in this
    /// directory (kept on the destination's filesystem so the later finalize is
    /// a same-FS rename, not a cross-device copy).
    Temp(&'a Path),
}

impl S3DownloadBody {
    /// Consumes the body into a blocking `Read` over the ciphertext (a
    /// `Cursor` over the in-memory bytes, or a reader over the spilled file).
    pub(super) fn into_reader(self) -> std::io::Result<Box<dyn Read + Send>> {
        match self {
            // `Cursor<Bytes>` reads with no copy of the buffered ciphertext.
            S3DownloadBody::InMemory(bytes) => Ok(Box::new(Cursor::new(bytes))),
            // The decrypt/copy step only reads a spilled body for CSE, which is
            // always a `Temp`; a `Part` body is the final plaintext and is
            // finalized by rename, not read back. Handle both for totality.
            S3DownloadBody::Spilled(SpilledBody::Temp(temp)) => {
                Ok(Box::new(multipart::SpilledReader::open(temp)?))
            }
            S3DownloadBody::Spilled(SpilledBody::Part(path)) => {
                Ok(Box::new(std::fs::File::open(path)?))
            }
        }
    }
}

/// Downloads a file from S3. For SSE stages the encryption metadata headers
/// will be absent and `file_metadata` is `None`. See `upload_to_s3_or_skip`
/// for the `refresher` semantics; refreshed credentials are written into the
/// shared `StageInfoCache` rather than returned.
///
/// A HEAD probe runs first (matching Python/JDBC/ODBC) to learn the object
/// size and metadata: blobs at or above `multipart.threshold` are fetched with
/// parallel ranged GETs into a tempfile; smaller ones take a single buffered
/// GET. `cloud_byte_count` reflects the on-cloud (pre-decryption) byte count
/// from the HEAD `Content-Length`.
pub(super) async fn download_from_s3(
    stage_info: &StageInfo,
    filename: &str,
    base_policy: &RetryPolicy,
    multipart: MultipartParams,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
    spill_target: SpillTarget<'_>,
) -> Result<S3Download, DownloadFileError> {
    let s3_key = format!("{}{filename}", stage_info.key_prefix);
    let policy = s3_retry_policy(base_policy);

    let attempt = |creds: CloudCredentials| {
        let stage_info = with_creds(stage_info, creds);
        let s3_key = s3_key.clone();
        let policy = policy.clone();
        async move {
            let s3_client = create_s3_client(&stage_info, SNOWFLAKE_DOWNLOAD_PROVIDER, &policy)
                .await
                .map_err(|e| S3AttemptError::Other(DownloadFileError::from(e)))?;
            s3_download_attempt(&s3_client, &stage_info, &s3_key, multipart, spill_target).await
        }
    };

    run_s3_with_sts_refresh(
        refresher,
        &stage_info.creds,
        |e| download_file_error::StageInfoRefreshSnafu.into_error(e),
        |aws_err| download_file_error::S3DownloadSnafu.into_error(aws_err),
        attempt,
    )
    .await
}

/// One download attempt: HEAD for size + metadata, then route to a single
/// buffered GET (small) or parallel ranged GETs into a tempfile (large). Run
/// inside the STS-refresh loop, so an `ExpiredToken` at any step retries the
/// whole download with fresh creds.
async fn s3_download_attempt(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
    multipart: MultipartParams,
    spill_target: SpillTarget<'_>,
) -> Result<S3Download, S3AttemptError<DownloadFileError>> {
    let head = s3_head_object(s3_client, stage_info, s3_key).await?;
    let content_length = head.content_length().unwrap_or(0).max(0) as u64;
    let metadata_map = head.metadata().cloned().unwrap_or_default();
    let (digest, file_metadata) =
        parse_s3_file_metadata(&metadata_map).map_err(S3AttemptError::Other)?;

    let body = if content_length >= multipart.threshold.bytes() {
        let chunk_size = multipart::compute_part_size(content_length, &MultipartConfig::S3)
            .context(download_file_error::FileTooLargeSnafu)
            .map_err(S3AttemptError::Other)?;
        tracing::debug!(
            "S3 ranged download: key={s3_key:?} content_length={content_length} \
             chunk_size={chunk_size} concurrency={}",
            multipart.concurrency
        );
        let spilled = s3_range_download(
            s3_client,
            stage_info,
            s3_key,
            content_length,
            chunk_size,
            multipart.concurrency,
            spill_target,
        )
        .await?;
        S3DownloadBody::Spilled(spilled)
    } else {
        S3DownloadBody::InMemory(s3_get_whole(s3_client, stage_info, s3_key).await?)
    };

    Ok(S3Download {
        body,
        digest,
        file_metadata,
        cloud_byte_count: content_length as i64,
    })
}

/// Parses the `sfc-digest` and the CSE metadata headers (`x-amz-matdesc` /
/// `x-amz-key` / `x-amz-iv`) from an S3 user-metadata map. All three CSE
/// headers must be present together or all absent (SSE); a partial set is an
/// error.
fn parse_s3_file_metadata(
    metadata_map: &HashMap<String, String>,
) -> Result<(Option<String>, Option<EncryptedFileMetadata>), DownloadFileError> {
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
    Ok((digest, file_metadata))
}

/// HEAD probe for object size + metadata. Folds `ExpiredToken` into
/// `StsExpired`.
async fn s3_head_object(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<aws_sdk_s3::operation::head_object::HeadObjectOutput, S3AttemptError<DownloadFileError>>
{
    tracing::info!(
        method = "HEAD",
        bucket = %stage_info.bucket,
        key = ?s3_key,
        "S3 HeadObject"
    );
    match s3_client
        .head_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .send()
        .await
    {
        Ok(out) => Ok(out),
        Err(sdk_err) => {
            tracing::warn!(
                cause = std::any::type_name_of_val(&sdk_err),
                bucket = %stage_info.bucket,
                key = ?s3_key,
                "S3 HeadObject failed"
            );
            Err(map_s3_error(aws_sdk_s3::Error::from(sdk_err), |e| {
                download_file_error::S3DownloadSnafu.into_error(e)
            }))
        }
    }
}

/// Collects an S3 GET response body into `Bytes`, mapping a stream error into a
/// `ByteStream` download error. Shared by the single-GET and ranged-GET paths.
async fn collect_s3_body(
    out: aws_sdk_s3::operation::get_object::GetObjectOutput,
) -> Result<Bytes, S3AttemptError<DownloadFileError>> {
    out.body
        .collect()
        .await
        .map(|agg| agg.into_bytes())
        .map_err(|e| S3AttemptError::Other(download_file_error::ByteStreamSnafu.into_error(e)))
}

/// Single buffered GET of the whole object body. Folds `ExpiredToken` into
/// `StsExpired`.
async fn s3_get_whole(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<Bytes, S3AttemptError<DownloadFileError>> {
    tracing::info!(
        method = "GET",
        bucket = %stage_info.bucket,
        key = ?s3_key,
        "S3 GetObject"
    );
    let out = match s3_client
        .get_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .send()
        .await
    {
        Ok(out) => out,
        Err(sdk_err) => {
            tracing::warn!(
                cause = std::any::type_name_of_val(&sdk_err),
                bucket = %stage_info.bucket,
                key = ?s3_key,
                "S3 GetObject failed"
            );
            return Err(map_s3_error(aws_sdk_s3::Error::from(sdk_err), |e| {
                download_file_error::S3DownloadSnafu.into_error(e)
            }));
        }
    };
    collect_s3_body(out).await
}

/// Downloads the object with parallel ranged GETs into a pre-allocated file,
/// returning the assembled [`SpilledBody`]. Ranges are fetched up to
/// `concurrency` at a time and written at their absolute offset (`pwrite`), so
/// out-of-order completion is fine.
///
/// The assembly file is chosen by `target`: a non-encrypted download writes
/// straight into the caller's `<dst>.part` (one rename from done), while an
/// encrypted / git-stage download writes into a throwaway temp (its ciphertext
/// is decrypted into `.part` afterwards).
///
/// On failure the range futures are *drained*, not short-circuited: every
/// in-flight `write_at` finishes and drops its file handle before we return, so
/// the partially-written assembly file can be removed even on Windows (which
/// refuses to unlink an open file). A failed download therefore leaves no
/// leftover; the only way a partial survives is a hard kill (SIGKILL / power
/// loss), and then it is a self-documenting, self-overwriting `<dst>.part`
/// rather than a random temp.
async fn s3_range_download(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
    content_length: u64,
    chunk_size: u64,
    concurrency: usize,
    target: SpillTarget<'_>,
) -> Result<SpilledBody, S3AttemptError<DownloadFileError>> {
    let mk_temp_err = |detail: String| {
        S3AttemptError::Other(download_file_error::TempFileSnafu { detail }.build())
    };

    // Owns the assembly file for the duration of the download: either the
    // caller's `.part` (referenced by path) or a RAII temp.
    enum Assembly {
        Part(PathBuf),
        Temp(NamedTempFile),
    }

    // Create + pre-allocate the assembly file off-thread. spawn_blocking (not
    // block_in_place): safe on current-thread runtimes; block_in_place panics
    // on those. Returns the owner plus a shared, cloneable handle for the
    // concurrent positioned writes.
    let owned_target = match target {
        SpillTarget::Part(p) => (true, p.to_path_buf()),
        SpillTarget::Temp(d) => (false, d.to_path_buf()),
    };
    #[allow(clippy::result_large_err)]
    let (assembly, file) = tokio::task::spawn_blocking(move || {
        let (is_part, path_or_dir) = owned_target;
        if is_part {
            let f = std::fs::File::create(&path_or_dir).map_err(|e| mk_temp_err(e.to_string()))?;
            // Pre-allocate so positioned writes of out-of-order chunks always land.
            f.set_len(content_length)
                .map_err(|e| mk_temp_err(e.to_string()))?;
            let file = Arc::new(f);
            Ok::<_, S3AttemptError<DownloadFileError>>((Assembly::Part(path_or_dir), file))
        } else {
            let named =
                NamedTempFile::new_in(&path_or_dir).map_err(|e| mk_temp_err(e.to_string()))?;
            named
                .as_file()
                .set_len(content_length)
                .map_err(|e| mk_temp_err(e.to_string()))?;
            let file = Arc::new(
                named
                    .as_file()
                    .try_clone()
                    .map_err(|e| mk_temp_err(e.to_string()))?,
            );
            Ok::<_, S3AttemptError<DownloadFileError>>((Assembly::Temp(named), file))
        }
    })
    .await
    .map_err(|e| mk_temp_err(format!("join error in tempfile setup: {e}")))??;

    let ranges = multipart::plan_ranges(content_length, chunk_size);
    // Drain, don't short-circuit: `collect` (not `try_collect`) polls EVERY
    // range future to completion, so all in-flight `write_at` spawn_blocking
    // tasks finish and release their cloned file handles before we return.
    // With no writer holding the file open, the cleanup below can unlink it
    // even on Windows. The first error is surfaced after the drain.
    let results: Vec<Result<(), S3AttemptError<DownloadFileError>>> = futures::stream::iter(ranges)
        .map(|range| {
            let file = Arc::clone(&file);
            async move {
                let bytes = s3_get_range(s3_client, stage_info, s3_key, &range).await?;
                // Guard against endpoints that ignore Range and return the whole
                // object (200 not 206): writing at range.start would corrupt the
                // assembled file by overrunning the pre-allocated length.
                let expected_len = range.end - range.start + 1;
                if bytes.len() as u64 != expected_len {
                    return Err(mk_temp_err(format!(
                        "ranged GET returned {} bytes, expected {expected_len} \
                         (bytes={}-{}); endpoint may not honour Range header",
                        bytes.len(),
                        range.start,
                        range.end
                    )));
                }
                tokio::task::spawn_blocking(move || multipart::write_at(&file, range.start, &bytes))
                    .await
                    .map_err(|e| mk_temp_err(format!("join error writing chunk: {e}")))?
                    .map_err(|e| mk_temp_err(e.to_string()))
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    // Release our write handle so the only reference left is the one owned by
    // `assembly` (Temp) or none (Part) — required before unlinking on Windows.
    drop(file);
    let outcome = results.into_iter().collect::<Result<Vec<()>, _>>();

    match assembly {
        Assembly::Part(path) => match outcome {
            Ok(_) => Ok(SpilledBody::Part(path)),
            Err(e) => {
                // Drained above, so no writer still holds `.part` open; the
                // best-effort remove succeeds even on Windows.
                let _ = std::fs::remove_file(&path);
                Err(e)
            }
        },
        // On success hand out the unlink-on-drop guard; on failure `named`
        // drops here and NamedTempFile unlinks it (drained, so no open writer).
        Assembly::Temp(named) => outcome.map(|_| SpilledBody::Temp(named.into_temp_path())),
    }
}

/// Ranged GET of `[range.start, range.end]`, returning the body bytes. Folds
/// `ExpiredToken` into `StsExpired`.
async fn s3_get_range(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
    range: &multipart::DownloadRange,
) -> Result<Bytes, S3AttemptError<DownloadFileError>> {
    let range_header = format!("bytes={}-{}", range.start, range.end);
    tracing::info!(
        method = "GET",
        bucket = %stage_info.bucket,
        key = ?s3_key,
        range = %range_header,
        "S3 GetObject (ranged)"
    );
    let out = match s3_client
        .get_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .range(range_header)
        .send()
        .await
    {
        Ok(out) => out,
        Err(sdk_err) => {
            tracing::warn!(
                cause = std::any::type_name_of_val(&sdk_err),
                bucket = %stage_info.bucket,
                key = ?s3_key,
                "S3 GetObject (ranged) failed"
            );
            return Err(map_s3_error(aws_sdk_s3::Error::from(sdk_err), |e| {
                download_file_error::S3DownloadSnafu.into_error(e)
            }));
        }
    };
    collect_s3_body(out).await
}

/// Returns a retry policy tuned for S3 file-transfer operations.
///
/// Mirrors the shape and budget of the GCS/Azure policies so that cross-cloud
/// behavior is consistent: exponential backoff from 1s to 16s, and a total
/// retry budget of 600s (2× `REQUEST_TIMEOUT_SECS`) so at least one
/// full-timeout attempt can complete before the budget expires.
///
/// The AWS SDK's standard retry strategy already covers transient transport
/// errors, 5xx server errors, and throttling (429, SlowDown). 403 is left to
/// the SDK's defaults: unlike GCS/Azure (where 403 commonly means "token not
/// yet propagated" / "SAS clock skew"), S3 returns 403 for genuine AccessDenied
/// and retrying is rarely productive — `create_s3_client` is called per
/// operation, so an expired STS token surfaces as a non-retryable 403 and the
/// caller can re-fetch credentials via a new PUT/GET parse.
///
/// Unlike GCS/Azure, S3 has no driver-side retry loop: the AWS SDK owns retry
/// (see `to_aws_retry_config` / `create_s3_client`), so there is no
/// `&RetryPolicy` threaded through a driver loop and no zero-backoff test seam
/// to inject — only policy-shape unit tests below.
pub(crate) fn s3_retry_policy(base: &RetryPolicy) -> RetryPolicy {
    let mut policy = base.clone();
    policy.per_request_timeout = Some(Duration::from_secs(REQUEST_TIMEOUT_SECS));
    policy
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
    let mut builder = AwsTimeoutConfig::builder();
    if let Some(budget) = policy.max_elapsed {
        builder = builder.operation_timeout(budget);
    }
    if let Some(per_attempt) = policy.per_request_timeout {
        builder = builder.operation_attempt_timeout(per_attempt);
    }
    builder.build()
}

async fn create_s3_client(
    stage_info: &StageInfo,
    provider_name: &'static str,
    policy: &RetryPolicy,
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

    let mut loader = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(stage_info.region.clone()))
        .retry_config(to_aws_retry_config(policy))
        .timeout_config(to_aws_timeout_config(policy));
    // Always inject our hyper/rustls client so S3 connections honour the
    // connection's full TLS policy (version window, CRL, custom root store).
    loader = loader.http_client(crate::tls::aws_http_client::tls_configured_aws_http_client(
        &stage_info.tls_config,
        stage_info.crl_worker.clone(),
    ));
    let config = loader.load().await;

    let accelerate = resolve_acceleration(stage_info, &config).await;
    let endpoint_url = resolve_s3_endpoint(stage_info, accelerate);
    Ok(build_s3_client(&config, endpoint_url))
}

/// Builds an `S3Client` from a shared `SdkConfig`, optionally pinning an
/// explicit endpoint. Shared by `create_s3_client` and the probe path so
/// they wire the AWS config the same way.
fn build_s3_client(config: &SdkConfig, endpoint_url: Option<String>) -> S3Client {
    let mut s3_config = aws_sdk_s3::config::Builder::from(config);
    // Keep the SDK's default CRC32 checksum on PUT. It streams as an `aws-chunked`
    // trailer (no buffering), with `SizedBody` advertising the body's exact size so
    // the checksum interceptor accepts the stream instead of rejecting it as
    // `UnsizedRequestBody`. S3 then verifies the checksum on receipt.
    if let Some(ep) = endpoint_url {
        tracing::debug!("Using S3 endpoint: {ep}");
        s3_config = s3_config.endpoint_url(ep);
    }
    S3Client::from_conf(s3_config.build())
}

/// Decides whether to route this stage through the S3 Transfer Acceleration
/// global endpoint. Mirrors the Python connector's behaviour: probe the
/// bucket once, cache the result, never probe internal stages or stages
/// that already pin a custom endpoint (FIPS / VPCE).
async fn resolve_acceleration(stage_info: &StageInfo, config: &SdkConfig) -> bool {
    if should_skip_acceleration_probe(stage_info) {
        return false;
    }
    if let Some(cached) = cached_acceleration(&stage_info.bucket) {
        return cached;
    }
    // Probe with the same endpoint we'd use without acceleration; if the
    // bucket turns out to be accelerated, the caller rebuilds the client
    // with the accelerate endpoint instead.
    let probe_endpoint = resolve_s3_endpoint(stage_info, false);
    let probe_client = build_s3_client(config, probe_endpoint);
    let enabled = probe_acceleration_enabled(&probe_client, &stage_info.bucket).await;
    store_acceleration(&stage_info.bucket, enabled);
    enabled
}

/// Skips the probe for internal (`sfc-*`) stages and for stages that have
/// an explicit endpoint set (FIPS / VPCE / custom): both are incompatible
/// with transfer acceleration and the probe would just waste an HTTP call.
fn should_skip_acceleration_probe(stage_info: &StageInfo) -> bool {
    stage_info.endpoint.is_some() || stage_info.bucket.starts_with(INTERNAL_STAGE_BUCKET_PREFIX)
}

/// Issues `GetBucketAccelerateConfiguration` and returns whether the bucket
/// has acceleration enabled. Any error (network, AccessDenied, malformed
/// response) is treated as "disabled" so the caller falls through to the
/// regular endpoint — acceleration is a throughput optimisation, not a
/// correctness requirement.
async fn probe_acceleration_enabled(client: &S3Client, bucket: &str) -> bool {
    match client
        .get_bucket_accelerate_configuration()
        .bucket(bucket)
        .send()
        .await
    {
        Ok(out) => matches!(out.status(), Some(BucketAccelerateStatus::Enabled)),
        Err(e) => {
            tracing::debug!(
                "S3 transfer-acceleration probe failed for bucket {bucket}: {e}; treating as disabled"
            );
            false
        }
    }
}

/// Process-wide cache of probe results keyed by bucket name. Buckets very
/// rarely flip the acceleration setting, so a permanent cache is acceptable
/// and matches Python's per-storage-client cache in practice.
static ACCELERATION_CACHE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();

fn acceleration_cache() -> &'static Mutex<HashMap<String, bool>> {
    ACCELERATION_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_acceleration(bucket: &str) -> Option<bool> {
    acceleration_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(bucket)
        .copied()
}

fn store_acceleration(bucket: &str, enabled: bool) {
    acceleration_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(bucket.to_string(), enabled);
}

/// Resolves the explicit S3 endpoint URL to hand to the AWS SDK builder, or
/// `None` to let the SDK derive the endpoint from the region.
///
/// Precedence (matches `snowflake-jdbc` and `libsnowflakeclient`, with
/// transfer-acceleration support layered on top — Python connector parity):
/// 1. `stage_info.endpoint` set (FIPS / VPCE / custom): used verbatim, with
///    `https://` prepended if no scheme is present. Wins over acceleration.
/// 2. `accelerate` true: route to `s3-accelerate.amazonaws.com`. The caller
///    has already verified the bucket has acceleration enabled.
/// 3. `stage_info.use_s3_regional_url` set: route to
///    `s3.<region>.amazonaws.com[.cn]`.
/// 4. Otherwise: `None` — the SDK uses its default endpoint resolver, which
///    handles standard regions, GovCloud, and `cn-*` correctly on its own.
///
/// Extracted as a pure function so callers (and tests) can verify the chosen
/// endpoint without going through `aws_sdk_s3::Config`, which doesn't expose
/// the configured URL.
fn resolve_s3_endpoint(stage_info: &StageInfo, accelerate: bool) -> Option<String> {
    if let Some(ep) = stage_info.endpoint.as_deref() {
        let endpoint_url = if ep.starts_with("https://") || ep.starts_with("http://") {
            ep.to_string()
        } else {
            format!("https://{ep}")
        };
        return Some(endpoint_url);
    }
    if accelerate {
        return Some(S3_ACCELERATE_ENDPOINT.to_string());
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
        upload_file_error::MissingS3CredentialsSnafu.build()
    }
}

impl From<S3CredentialError> for DownloadFileError {
    fn from(_: S3CredentialError) -> Self {
        download_file_error::MissingS3CredentialsSnafu.build()
    }
}

/// `pub` because it is a `source` field on the public
/// `FileManagerError::S3Upload`; `pub(super)` trips `private_interfaces`.
#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum UploadFileError {
    #[snafu(display("Failed to open upload source for S3 PUT: {detail}"))]
    SourceOpen {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload file to S3"))]
    S3Upload {
        #[snafu(source(from(aws_sdk_s3::Error, Box::new)))]
        source: Box<aws_sdk_s3::Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("File too large for S3 multipart upload"))]
    FileTooLarge {
        #[snafu(source(from(multipart::FileTooLargeError, Box::new)))]
        source: Box<multipart::FileTooLargeError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read upload source for an S3 multipart part: {detail}"))]
    SourceRead {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to start S3 multipart upload: {detail}"))]
    S3MultipartCreate {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload S3 multipart part {part_number}: {detail}"))]
    S3UploadPart {
        part_number: i32,
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to complete S3 multipart upload: {detail}"))]
    S3MultipartComplete {
        detail: String,
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
    StageInfoRefresh {
        #[snafu(source(from(StageInfoRefreshError, Box::new)))]
        source: Box<StageInfoRefreshError>,
        #[snafu(implicit)]
        location: Location,
    },
}

/// `pub` for the same reason as [`UploadFileError`] — a `source` field on the
/// public `FileManagerError::S3Download`.
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
    #[snafu(display("Object too large to download from S3"))]
    FileTooLarge {
        #[snafu(source(from(multipart::FileTooLargeError, Box::new)))]
        source: Box<multipart::FileTooLargeError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to stage S3 ranged download to a temp file: {detail}"))]
    TempFile {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing S3 credentials"))]
    MissingS3Credentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to refresh S3 stage credentials after ExpiredToken"))]
    StageInfoRefresh {
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
    use crate::config::retry::RetryPolicy;
    use bytes::Bytes;

    fn base_policy() -> RetryPolicy {
        use crate::config::param_store::ParamStore;
        RetryPolicy::put_get(&ParamStore::new())
    }

    fn base_policy_with_attempts(n: u32) -> RetryPolicy {
        let mut p = base_policy();
        p.max_attempts = n;
        p
    }

    #[test]
    fn s3_retry_policy_max_attempts() {
        let policy = s3_retry_policy(&base_policy_with_attempts(25));
        assert_eq!(policy.max_attempts, 25);
        assert_eq!(to_aws_retry_config(&policy).max_attempts(), 25);

        assert_eq!(
            s3_retry_policy(&base_policy_with_attempts(1)).max_attempts,
            1
        );
    }

    #[test]
    fn s3_retry_policy_backoff_bounds() {
        let policy = s3_retry_policy(&base_policy());
        assert_eq!(policy.backoff.base, Duration::from_millis(250));
        assert_eq!(policy.backoff.cap, Duration::from_secs(16));
        assert_eq!(policy.backoff.factor, 2.0);
    }

    #[test]
    fn s3_retry_policy_max_elapsed_exceeds_request_timeout() {
        let policy = s3_retry_policy(&base_policy());
        assert!(
            policy.max_elapsed > Some(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
            "retry budget must exceed a single request timeout"
        );
        assert_eq!(policy.max_elapsed, Some(Duration::from_secs(600)));
    }

    #[test]
    fn s3_retry_policy_has_per_request_timeout() {
        let policy = s3_retry_policy(&base_policy());
        assert_eq!(
            policy.per_request_timeout,
            Some(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
            "per_request_timeout must be set so the SDK cancels stuck attempts"
        );
    }

    #[test]
    fn to_aws_retry_config_translates_policy() {
        let policy = s3_retry_policy(&base_policy());
        let aws = to_aws_retry_config(&policy);
        assert_eq!(aws.max_attempts(), policy.max_attempts);
        assert_eq!(aws.initial_backoff(), policy.backoff.base);
        assert_eq!(aws.max_backoff(), policy.backoff.cap);
    }

    #[test]
    fn to_aws_timeout_config_sets_attempt_and_operation_timeouts() {
        let policy = s3_retry_policy(&base_policy());
        let cfg = to_aws_timeout_config(&policy);
        assert_eq!(cfg.operation_timeout(), policy.max_elapsed);
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
    // Exercises the cases the AWS SDK can't surface for us because
    // `aws_sdk_s3::Config` does not expose the resolved URL: explicit
    // endpoint, regional flag, accelerate, neither, and scheme-less
    // endpoint.

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
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
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
            resolve_s3_endpoint(&stage, false).as_deref(),
            Some("https://my-fips.us-east-1.amazonaws.com")
        );
    }

    #[test]
    fn resolve_endpoint_uses_regional_when_only_flag_set() {
        let stage = s3_stage(None, true);
        assert_eq!(
            resolve_s3_endpoint(&stage, false).as_deref(),
            Some("https://s3.us-east-1.amazonaws.com")
        );
    }

    #[test]
    fn resolve_endpoint_returns_none_when_neither_set() {
        // Falls through to the AWS SDK's default endpoint resolver — we
        // must NOT pre-pin an endpoint, otherwise the SDK can't apply its
        // own `cn-*` / GovCloud handling.
        let stage = s3_stage(None, false);
        assert_eq!(resolve_s3_endpoint(&stage, false), None);
    }

    #[test]
    fn resolve_endpoint_prepends_https_when_scheme_missing() {
        // GS sometimes sends `endPoint` without a scheme (host only).
        // The SDK's `endpoint_url` requires a scheme, so we add `https://`.
        let stage = s3_stage(Some("my-fips.us-east-1.amazonaws.com"), false);
        assert_eq!(
            resolve_s3_endpoint(&stage, false).as_deref(),
            Some("https://my-fips.us-east-1.amazonaws.com")
        );
    }

    #[test]
    fn resolve_endpoint_preserves_http_scheme() {
        // If GS or a test fixture supplies `http://`, we must not double-
        // prefix or upgrade the scheme.
        let stage = s3_stage(Some("http://localhost:9000"), false);
        assert_eq!(
            resolve_s3_endpoint(&stage, false).as_deref(),
            Some("http://localhost:9000")
        );
    }

    // --- Acceleration endpoint resolution ---

    #[test]
    fn resolve_endpoint_uses_accelerate_when_enabled() {
        let stage = s3_stage(None, false);
        assert_eq!(
            resolve_s3_endpoint(&stage, true).as_deref(),
            Some(S3_ACCELERATE_ENDPOINT)
        );
    }

    #[test]
    fn resolve_endpoint_explicit_endpoint_wins_over_accelerate() {
        // Acceleration is incompatible with FIPS / VPCE; the GS-supplied
        // endpoint must still win even if the probe somehow returned true.
        let stage = s3_stage(Some("https://my-fips.us-east-1.amazonaws.com"), false);
        assert_eq!(
            resolve_s3_endpoint(&stage, true).as_deref(),
            Some("https://my-fips.us-east-1.amazonaws.com")
        );
    }

    #[test]
    fn resolve_endpoint_accelerate_wins_over_regional_flag() {
        // When acceleration is enabled, it routes through the global
        // accelerate endpoint regardless of `use_s3_regional_url`.
        let stage = s3_stage(None, true);
        assert_eq!(
            resolve_s3_endpoint(&stage, true).as_deref(),
            Some(S3_ACCELERATE_ENDPOINT)
        );
    }

    #[test]
    fn should_skip_acceleration_probe_for_internal_stage() {
        let mut stage = s3_stage(None, false);
        stage.bucket = "sfc-internal-bucket".to_string();
        assert!(should_skip_acceleration_probe(&stage));
    }

    #[test]
    fn should_skip_acceleration_probe_when_explicit_endpoint_set() {
        // FIPS / VPCE: the probe would be wasted because the endpoint can
        // not be swapped for accelerate even if the bucket has it on.
        let stage = s3_stage(Some("https://my-fips.us-east-1.amazonaws.com"), false);
        assert!(should_skip_acceleration_probe(&stage));
    }

    #[test]
    fn should_not_skip_acceleration_probe_for_external_stage() {
        let stage = s3_stage(None, false);
        assert!(!should_skip_acceleration_probe(&stage));
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
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
            storage_account: None,
        };

        // overwrite=true skips the HEAD probe.
        upload_to_s3_or_skip(
            prepared,
            &stage_info,
            "f.dat",
            true,
            &base_policy(),
            MultipartParams::default(),
            &mut None,
        )
        .await
        .expect("upload should succeed against the mock");
    }

    // SSE body is sent whole (`UNSIGNED-PAYLOAD`). The CSE path's unsigned-payload
    // sentinel (`STREAMING-UNSIGNED-PAYLOAD-TRAILER`) is asserted in
    // `put_object_encrypted_streams_with_crc32_trailer`, which exercises the same
    // upload end-to-end.
    #[tokio::test(flavor = "multi_thread")]
    async fn put_object_sends_unsigned_payload_for_unencrypted_upload() {
        assert_put_sends_unsigned_payload(PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(Bytes::from_static(
                b"hello world",
            )),
            digest: "0".repeat(64),
            cse: None,
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn put_object_streams_multichunk_sse_file_with_crc32_trailer() {
        use std::io::Write;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use wiremock::Request;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&vec![b'x'; 100 * 1024]).unwrap(); // >64 KiB → multiple body frames
        tmp.flush().unwrap();
        let path = tmp.path().to_path_buf();

        let seen_crc32_trailer = Arc::new(AtomicBool::new(false));
        let seen_unsigned_payload = Arc::new(AtomicBool::new(false));
        let crc32_c = seen_crc32_trailer.clone();
        let unsigned_c = seen_unsigned_payload.clone();

        let mock = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(move |req: &Request| {
                crc32_c.store(
                    req.headers
                        .get("x-amz-trailer")
                        .and_then(|v| v.to_str().ok())
                        .is_some_and(|v| v.contains("x-amz-checksum-crc32")),
                    Ordering::SeqCst,
                );
                unsigned_c.store(
                    req.headers
                        .get("x-amz-content-sha256")
                        .and_then(|v| v.to_str().ok())
                        == Some("STREAMING-UNSIGNED-PAYLOAD-TRAILER"),
                    Ordering::SeqCst,
                );
                ResponseTemplate::new(200)
            })
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
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
        };

        tokio::time::timeout(
            std::time::Duration::from_secs(20),
            upload_to_s3_or_skip(
                PreparedUpload {
                    source: crate::file_manager::types::PreparedSource::Path(path),
                    digest: "0".repeat(64),
                    cse: None,
                },
                &stage_info,
                "f.dat",
                true,
                &base_policy(),
                MultipartParams::default(),
                &mut None,
            ),
        )
        .await
        .expect("multi-chunk SSE upload hung — aws-chunked Content-Length regressed")
        .expect("multi-chunk SSE file upload should succeed against the mock");

        assert!(
            seen_unsigned_payload.load(Ordering::SeqCst),
            "the streamed body must be payload-unsigned (STREAMING-UNSIGNED-PAYLOAD-TRAILER)",
        );
        assert!(
            seen_crc32_trailer.load(Ordering::SeqCst),
            "the streamed body must carry a CRC32 trailer for S3 to verify on receipt",
        );
    }

    // Pins the wire contract for the lazy encrypting `SdkBody`: the SDK streams
    // it under `aws-chunked` with a CRC32 trailer (so S3 verifies integrity on
    // receipt) while conveying the exact ciphertext length analytically via
    // `x-amz-decoded-content-length` — i.e. CRC32 without buffering the body.
    #[tokio::test(flavor = "multi_thread")]
    async fn put_object_encrypted_streams_with_crc32_trailer() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
        use wiremock::Request;

        let plaintext = b"client-side-encrypted S3 upload body".to_vec();
        let material = crate::file_manager::types::EncryptionMaterial {
            query_stage_master_key: crate::sensitive::SensitiveString::from(
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [0u8; 32]),
            ),
            query_id: "q".to_string(),
            smk_id: "1".to_string(),
        };
        let (encryptor, encryption_metadata) =
            super::super::encryption::build_encryptor(&material, plaintext.len() as i64).unwrap();
        let expected_len = encryptor.cipher_len() as u64;

        let mock = MockServer::start().await;
        let seen_decoded_len = Arc::new(AtomicU64::new(u64::MAX));
        let seen_crc32_trailer = Arc::new(AtomicBool::new(false));
        let seen_unsigned_payload = Arc::new(AtomicBool::new(false));
        let seen_decoded_len_c = seen_decoded_len.clone();
        let seen_crc32_trailer_c = seen_crc32_trailer.clone();
        let seen_unsigned_payload_c = seen_unsigned_payload.clone();
        Mock::given(method("PUT"))
            .respond_with(move |req: &Request| {
                if let Some(len) = req
                    .headers
                    .get("x-amz-decoded-content-length")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                {
                    seen_decoded_len_c.store(len, Ordering::SeqCst);
                }
                let crc32 = req
                    .headers
                    .get("x-amz-trailer")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.contains("x-amz-checksum-crc32"));
                seen_crc32_trailer_c.store(crc32, Ordering::SeqCst);
                let unsigned = req
                    .headers
                    .get("x-amz-content-sha256")
                    .and_then(|v| v.to_str().ok())
                    == Some("STREAMING-UNSIGNED-PAYLOAD-TRAILER");
                seen_unsigned_payload_c.store(unsigned, Ordering::SeqCst);
                ResponseTemplate::new(200)
            })
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
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
            storage_account: None,
        };

        upload_to_s3_or_skip(
            PreparedUpload {
                source: crate::file_manager::types::PreparedSource::Bytes(plaintext.into()),
                digest: "0".repeat(64),
                cse: Some(crate::file_manager::types::CseParams {
                    metadata: encryption_metadata,
                    encryptor,
                }),
            },
            &stage_info,
            "f.dat",
            true,
            &base_policy(),
            MultipartParams::default(),
            &mut None,
        )
        .await
        .expect("encrypted S3 upload should succeed against the mock");

        assert_eq!(
            seen_decoded_len.load(Ordering::SeqCst),
            expected_len,
            "x-amz-decoded-content-length must equal the analytic ciphertext length",
        );
        assert!(
            seen_crc32_trailer.load(Ordering::SeqCst),
            "the streamed body must carry a CRC32 checksum trailer for S3 to verify on receipt",
        );
        assert!(
            seen_unsigned_payload.load(Ordering::SeqCst),
            "the CSE body must stay payload-unsigned (STREAMING-UNSIGNED-PAYLOAD-TRAILER)",
        );
    }

    // --- Acceleration probe (wiremock) ---

    use wiremock::matchers::query_param;

    async fn build_probe_client_against(uri: &str) -> S3Client {
        let creds = Credentials::new(
            "AKIA-TEST",
            "secret",
            Some("token".to_string()),
            None,
            "test",
        );
        let config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(Region::new("us-east-1"))
            .load()
            .await;
        build_s3_client(&config, Some(uri.to_string()))
    }

    const ACCELERATE_ENABLED_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<AccelerateConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Status>Enabled</Status></AccelerateConfiguration>"#;

    const ACCELERATE_SUSPENDED_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<AccelerateConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Status>Suspended</Status></AccelerateConfiguration>"#;

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_returns_true_when_status_enabled() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("accelerate", ""))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(ACCELERATE_ENABLED_XML, "application/xml"),
            )
            .mount(&mock)
            .await;

        let client = build_probe_client_against(&mock.uri()).await;
        assert!(probe_acceleration_enabled(&client, "probe-enabled-bkt").await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_returns_false_when_status_suspended() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("accelerate", ""))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(ACCELERATE_SUSPENDED_XML, "application/xml"),
            )
            .mount(&mock)
            .await;

        let client = build_probe_client_against(&mock.uri()).await;
        assert!(!probe_acceleration_enabled(&client, "probe-suspended-bkt").await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_returns_false_on_access_denied() {
        // Limited stage credentials may lack `s3:GetAccelerateConfiguration`.
        // Treat that as "not accelerated" rather than failing the upload.
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("accelerate", ""))
            .respond_with(ResponseTemplate::new(403))
            .mount(&mock)
            .await;

        let client = build_probe_client_against(&mock.uri()).await;
        assert!(!probe_acceleration_enabled(&client, "probe-denied-bkt").await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_result_is_cached_per_bucket() {
        // The mock expects exactly one call; a second `cached_acceleration`
        // hit must short-circuit without issuing another probe.
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("accelerate", ""))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(ACCELERATE_ENABLED_XML, "application/xml"),
            )
            .expect(1)
            .mount(&mock)
            .await;

        // Unique per test, so the process-global cache doesn't collide
        // with anything else running in parallel.
        let bucket = "probe-cache-bkt-isolated";

        let client = build_probe_client_against(&mock.uri()).await;
        let probed = probe_acceleration_enabled(&client, bucket).await;
        store_acceleration(bucket, probed);

        // Subsequent resolutions read the cache and never reach the mock.
        assert_eq!(cached_acceleration(bucket), Some(true));
    }

    #[test]
    fn cached_acceleration_returns_none_for_unknown_bucket() {
        // Unique name avoids collisions with any cache entries other tests
        // (or future tests) might leave behind in this process-global map.
        assert_eq!(cached_acceleration("never-stored-bkt-fc8a4f"), None);
    }

    // --- Multipart upload / ranged download (wiremock) ---

    use std::sync::atomic::Ordering as MpOrdering;
    use wiremock::Request;

    const CREATE_MP_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<InitiateMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Bucket>test-bucket</Bucket><Key>prefix/f.dat</Key><UploadId>test-upload-id</UploadId></InitiateMultipartUploadResult>"#;

    const COMPLETE_MP_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<CompleteMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Location>http://example/f.dat</Location><Bucket>test-bucket</Bucket><Key>prefix/f.dat</Key><ETag>"final-etag"</ETag></CompleteMultipartUploadResult>"#;

    fn mp_stage(uri: String) -> StageInfo {
        StageInfo {
            location_type: crate::file_manager::types::LocationType::S3,
            bucket: "test-bucket".to_string(),
            key_prefix: "prefix/".to_string(),
            region: "us-east-1".to_string(),
            creds: s3_creds("AKIA-TEST"),
            endpoint: Some(uri),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            storage_account: None,
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
        }
    }

    /// `MultipartParams` with a 1-byte threshold so any non-empty body takes
    /// the multipart path, at the resolved concurrency.
    fn always_multipart() -> MultipartParams {
        MultipartParams {
            threshold: super::super::multipart::MultipartThreshold::from_server(Some(1)),
            concurrency: 4,
        }
    }

    /// A 20 MiB SSE body splits into three S3 parts (8 + 8 + 4 MiB) at the
    /// default 8 MiB chunk size, exercising the create → parallel UploadPart →
    /// complete sequence end to end.
    #[tokio::test(flavor = "multi_thread")]
    async fn s3_multipart_upload_runs_create_parts_complete() {
        let mock = MockServer::start().await;
        let parts = Arc::new(AtomicUsize::new(0));

        Mock::given(method("POST"))
            .and(query_param("uploads", ""))
            .respond_with(ResponseTemplate::new(200).set_body_raw(CREATE_MP_XML, "application/xml"))
            .expect(1)
            .mount(&mock)
            .await;

        let parts_c = parts.clone();
        Mock::given(method("PUT"))
            .and(query_param("uploadId", "test-upload-id"))
            .respond_with(move |_: &Request| {
                let n = parts_c.fetch_add(1, MpOrdering::SeqCst);
                ResponseTemplate::new(200).insert_header("ETag", format!("\"etag-{n}\"").as_str())
            })
            .mount(&mock)
            .await;

        Mock::given(method("POST"))
            .and(query_param("uploadId", "test-upload-id"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(COMPLETE_MP_XML, "application/xml"),
            )
            .expect(1)
            .mount(&mock)
            .await;

        let prepared = PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(Bytes::from(vec![
                7u8;
                20 << 20
            ])),
            digest: "0".repeat(64),
            cse: None,
        };

        upload_to_s3_or_skip(
            prepared,
            &mp_stage(mock.uri()),
            "f.dat",
            true,
            &base_policy(),
            always_multipart(),
            &mut None,
        )
        .await
        .expect("multipart upload should succeed against the mock");

        assert_eq!(
            parts.load(MpOrdering::SeqCst),
            3,
            "20 MiB / 8 MiB chunk must upload exactly 3 parts"
        );
    }

    /// When a part upload fails, the whole multipart upload must be aborted
    /// (`AbortMultipartUpload`) before the error propagates — the fix for the
    /// libsnowflakeclient `// TODO abort` orphan-cost gap.
    #[tokio::test(flavor = "multi_thread")]
    async fn s3_multipart_upload_aborts_on_part_failure() {
        let mock = MockServer::start().await;
        let aborts = Arc::new(AtomicUsize::new(0));

        Mock::given(method("POST"))
            .and(query_param("uploads", ""))
            .respond_with(ResponseTemplate::new(200).set_body_raw(CREATE_MP_XML, "application/xml"))
            .mount(&mock)
            .await;

        // Every part fails with a non-retryable 400.
        Mock::given(method("PUT"))
            .and(query_param("uploadId", "test-upload-id"))
            .respond_with(ResponseTemplate::new(400).set_body_string("nope"))
            .mount(&mock)
            .await;

        let aborts_c = aborts.clone();
        Mock::given(method("DELETE"))
            .and(query_param("uploadId", "test-upload-id"))
            .respond_with(move |_: &Request| {
                aborts_c.fetch_add(1, MpOrdering::SeqCst);
                ResponseTemplate::new(204)
            })
            .mount(&mock)
            .await;

        let prepared = PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(Bytes::from(vec![
                7u8;
                9 << 20
            ])),
            digest: "0".repeat(64),
            cse: None,
        };

        let result = upload_to_s3_or_skip(
            prepared,
            &mp_stage(mock.uri()),
            "f.dat",
            true,
            &base_policy_with_attempts(1), // single attempt: fail fast, no SDK retry storm
            always_multipart(),
            &mut None,
        )
        .await;

        assert!(result.is_err(), "a failing part must fail the upload");
        assert_eq!(
            aborts.load(MpOrdering::SeqCst),
            1,
            "the multipart upload must be aborted exactly once on failure"
        );
    }

    /// A blob above the threshold is fetched with a ranged GET into a tempfile
    /// and re-read byte-for-byte through `S3DownloadBody::into_reader`.
    #[tokio::test(flavor = "multi_thread")]
    async fn s3_ranged_download_reassembles_object() {
        use std::io::Read as _;

        let payload = b"hello ranged multipart world".to_vec();
        let mock = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-length", payload.len().to_string()),
            )
            .mount(&mock)
            .await;

        // The single range [0, len-1] returns the whole payload.
        let body = payload.clone();
        Mock::given(method("GET"))
            .respond_with(move |_: &Request| {
                ResponseTemplate::new(206).set_body_bytes(body.clone())
            })
            .mount(&mock)
            .await;

        let spill = tempfile::tempdir().unwrap();
        let download = download_from_s3(
            &mp_stage(mock.uri()),
            "f.dat",
            &base_policy(),
            always_multipart(),
            &mut None,
            SpillTarget::Temp(spill.path()),
        )
        .await
        .expect("ranged download should succeed against the mock");

        assert_eq!(download.cloud_byte_count, payload.len() as i64);
        assert!(
            matches!(download.body, S3DownloadBody::Spilled(SpilledBody::Temp(_))),
            "above-threshold download must spill to a tempfile"
        );

        let mut reader = download.body.into_reader().unwrap();
        let mut got = Vec::new();
        reader.read_to_end(&mut got).unwrap();
        assert_eq!(got, payload, "reassembled ciphertext must match the object");
    }

    /// A non-encrypted ranged download assembles straight into the caller's
    /// `.part` file (no intermediate temp), which the caller renames to the
    /// destination on success.
    #[tokio::test(flavor = "multi_thread")]
    async fn s3_ranged_download_assembles_into_part_file() {
        let payload = b"hello ranged straight into dot part".to_vec();
        let mock = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-length", payload.len().to_string()),
            )
            .mount(&mock)
            .await;

        let body = payload.clone();
        Mock::given(method("GET"))
            .respond_with(move |_: &Request| {
                ResponseTemplate::new(206).set_body_bytes(body.clone())
            })
            .mount(&mock)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let part_path = dir.path().join("out.dat.part");
        let download = download_from_s3(
            &mp_stage(mock.uri()),
            "f.dat",
            &base_policy(),
            always_multipart(),
            &mut None,
            SpillTarget::Part(&part_path),
        )
        .await
        .expect("ranged download should succeed against the mock");

        match download.body {
            S3DownloadBody::Spilled(SpilledBody::Part(p)) => {
                assert_eq!(p, part_path, "the assembly file must be the caller's .part");
                assert_eq!(
                    std::fs::read(&p).unwrap(),
                    payload,
                    "the .part must hold the whole reassembled object"
                );
            }
            _ => panic!("a non-encrypted ranged download must assemble into `.part`"),
        }
    }

    /// A failed ranged download drains its in-flight writes and removes the
    /// `.part`, so a failure never leaves a partial file behind.
    #[tokio::test(flavor = "multi_thread")]
    async fn s3_ranged_download_failure_removes_part_file() {
        let mock = MockServer::start().await;

        // HEAD advertises 32 bytes, but every ranged GET returns a 4-byte body,
        // tripping the Range-honouring length guard and failing the download.
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200).insert_header("content-length", "32"))
            .mount(&mock)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(vec![0u8; 4]))
            .mount(&mock)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let part_path = dir.path().join("out.dat.part");
        let result = download_from_s3(
            &mp_stage(mock.uri()),
            "f.dat",
            &base_policy_with_attempts(1), // single attempt: fail fast
            always_multipart(),
            &mut None,
            SpillTarget::Part(&part_path),
        )
        .await;

        assert!(result.is_err(), "a short ranged GET must fail the download");
        assert!(
            !part_path.exists(),
            "a failed ranged download must not leave a `.part` file behind"
        );
    }
}
