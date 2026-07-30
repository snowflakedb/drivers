use super::cloud_http::{self, CloudStreamingDownload, CseDownloadInfo, UploadRetryAdapter};
use super::multipart::{self, MultipartConfig, MultipartParams};
use super::types::{
    ByteSource, CloudCredentials, DownloadResponse, EncryptedFileMetadata, EncryptionData,
    LocationType, MaterialDescription, PreparedUpload, StageInfo, StageInfoRefreshError,
    StageInfoRefresher, UploadStatus, build_encryption_metadata_json, percent_encode_path,
};
use super::{RemoteHead, skip_upload_decision};
use crate::config::retry::RetryPolicy;
use crate::http::retry::{HttpContext, HttpError, execute_with_retry as http_execute_with_retry};
use crate::refresh::{Refresher, execute_with_refresh};
use crate::sensitive::SensitiveString;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_ENGINE};
use bytes::Bytes;
use futures::StreamExt as _;
use futures::TryStreamExt as _;
use reqwest::Method;
use snafu::{IntoError, Location, OptionExt, ResultExt, Snafu};
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use tokio_stream::wrappers::ReceiverStream;

const REQUEST_TIMEOUT_SECS: u64 = 300;

// Azure metadata header names
const AZURE_META_SFC_DIGEST: &str = "x-ms-meta-sfcdigest";
const AZURE_META_ENCRYPTIONDATA: &str = "x-ms-meta-encryptiondata";
const AZURE_META_MATDESC: &str = "x-ms-meta-matdesc";

/// HTTP status code that Azure returns on SAS token expiry or authorization
/// failure. Any 403 triggers a SAS refresh; the body is parsed only for
/// diagnostic logging.
const AZURE_SAS_REFRESH_STATUS: u16 = 403;

/// Known Azure XML error codes that indicate SAS token expiry.
const KNOWN_SAS_EXPIRY_CODES: &[&str] = &["AuthenticationFailed", "InvalidAuthenticationInfo"];

// === SAS-refresh-on-403 infrastructure (mirrors the S3 STS-refresh shape) ===

/// Predicate: does this status code warrant a SAS refresh? Any 403, body-blind.
fn is_expired_sas_error(status_code: u16) -> bool {
    status_code == AZURE_SAS_REFRESH_STATUS
}

/// Extracts `<Code>` from an Azure XML error body. Returns `None` if absent or malformed.
fn parse_azure_error_code(body: &str) -> Option<String> {
    let start = body.find("<Code>")? + "<Code>".len();
    let rel_end = body[start..].find("</Code>")?;
    Some(body[start..start + rel_end].trim().to_string())
}

/// Returns a copy of `stage_info` with `creds` replaced.
fn with_creds(stage_info: &StageInfo, creds: CloudCredentials) -> StageInfo {
    let mut info = stage_info.clone();
    info.creds = creds;
    info
}

/// One Azure transfer attempt's error. `SasExpired` is the recoverable 403;
/// everything else is `Other`.
#[derive(Debug)]
enum AzureAttemptError<E> {
    SasExpired {
        status_code: u16,
        body: String,
        url_redacted: String,
        code: Option<String>,
    },
    Other(E),
}

impl<E> AzureAttemptError<E> {
    fn map_other<F, E2>(self, f: F) -> AzureAttemptError<E2>
    where
        F: FnOnce(E) -> E2,
    {
        match self {
            AzureAttemptError::SasExpired {
                status_code,
                body,
                url_redacted,
                code,
            } => AzureAttemptError::SasExpired {
                status_code,
                body,
                url_redacted,
                code,
            },
            AzureAttemptError::Other(e) => AzureAttemptError::Other(f(e)),
        }
    }
}

/// Maps a 403 to the recoverable `SasExpired` (routes to the refresh layer); any
/// other status to `Other`. The single shared home for the 403→refresh decision.
fn map_http_to_attempt<E>(
    status_code: u16,
    body: String,
    url_redacted: &str,
    make_http_err: impl FnOnce(u16, String) -> E,
) -> AzureAttemptError<E> {
    if is_expired_sas_error(status_code) {
        let code = parse_azure_error_code(&body);
        let known_expiry_code = code
            .as_deref()
            .is_some_and(|c| KNOWN_SAS_EXPIRY_CODES.contains(&c));
        tracing::debug!(
            status = status_code,
            code = code.as_deref().unwrap_or("unknown"),
            known_expiry_code,
            url_redacted = %url_redacted,
            "Azure 403; routing to SAS refresh"
        );
        AzureAttemptError::SasExpired {
            status_code,
            body,
            url_redacted: url_redacted.to_string(),
            code,
        }
    } else {
        AzureAttemptError::Other(make_http_err(status_code, body))
    }
}

/// Azure SAS implementation of [`Refresher`]. Detects whether a refresh
/// actually landed a new snapshot via the cache's monotonic `cached_at` marker.
struct AzureSasRefresher<'a, E, W> {
    refresher: &'a mut dyn StageInfoRefresher,
    last_cached_at: Instant,
    map_refresh_err: W,
    _marker: PhantomData<fn() -> E>,
}

impl<'a, E, W> AzureSasRefresher<'a, E, W>
where
    W: Fn(StageInfoRefreshError) -> E + Send,
{
    fn new_with_marker(
        refresher: &'a mut dyn StageInfoRefresher,
        last_cached_at: Instant,
        map_refresh_err: W,
    ) -> Self {
        Self {
            refresher,
            last_cached_at,
            map_refresh_err,
            _marker: PhantomData,
        }
    }
}

impl<'a, E, W> Refresher<CloudCredentials, AzureAttemptError<E>> for AzureSasRefresher<'a, E, W>
where
    E: Send,
    W: Fn(StageInfoRefreshError) -> E + Send,
{
    fn current(
        &mut self,
    ) -> crate::refresh::RefreshFuture<'_, Result<CloudCredentials, AzureAttemptError<E>>> {
        let creds = self.refresher.cache().snapshot().creds;
        Box::pin(async move { Ok(creds) })
    }

    fn should_refresh(&self, err: &AzureAttemptError<E>) -> bool {
        matches!(err, AzureAttemptError::SasExpired { .. })
    }

    fn refresh(&mut self) -> crate::refresh::RefreshFuture<'_, Result<bool, AzureAttemptError<E>>> {
        Box::pin(async move {
            tracing::info!("Azure hit expired-SAS 403; refreshing stage credentials");
            self.refresher
                .refresh()
                .await
                .map_err(|e| AzureAttemptError::Other((self.map_refresh_err)(e)))?;
            let current = self.refresher.cache().cached_at();
            if current == self.last_cached_at {
                return Ok(false);
            }
            self.last_cached_at = current;
            Ok(true)
        })
    }
}

/// Runs `attempt` once (no refresher) or in a refresh-retry loop (with
/// refresher), folding `AzureAttemptError<E>` back to `E` at the boundary.
///
/// 403 logging is outcome-tiered: a 403 recovered by a refresh is a `debug!`
/// breadcrumb; a terminal failure logs once — a refresh-mechanism failure at the
/// call site → `error!`, a 403 that survives the refresh → `warn!` here with the
/// status and a SAS-redacted URL.
async fn run_azure_with_sas_refresh<F, Fut, T, E>(
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
    initial_creds: &CloudCredentials,
    op: &'static str,
    map_refresh_err: impl Fn(StageInfoRefreshError) -> E + Send,
    map_sas_err: impl Fn(u16, String) -> E,
    attempt: F,
) -> Result<T, E>
where
    F: Fn(CloudCredentials) -> Fut,
    Fut: std::future::Future<Output = Result<T, AzureAttemptError<E>>>,
    E: Send,
{
    fn fold_silent<E>(e: AzureAttemptError<E>, map_sas_err: impl Fn(u16, String) -> E) -> E {
        match e {
            AzureAttemptError::Other(inner) => inner,
            AzureAttemptError::SasExpired {
                status_code, body, ..
            } => map_sas_err(status_code, body),
        }
    }

    match refresher {
        Some(r) => {
            let initial_marker = r.cache().cached_at();
            let mut sas_refresher =
                AzureSasRefresher::new_with_marker(*r, initial_marker, map_refresh_err);
            execute_with_refresh(&mut sas_refresher, attempt)
                .await
                .map_err(|e| match e {
                    AzureAttemptError::Other(inner) => inner,
                    AzureAttemptError::SasExpired {
                        status_code,
                        body,
                        url_redacted,
                        code,
                    } => {
                        tracing::warn!(
                            status = status_code,
                            code = code.as_deref().unwrap_or("unknown"),
                            url_redacted = %url_redacted,
                            "Azure {op} failed terminally with 403 after SAS refresh"
                        );
                        map_sas_err(status_code, body)
                    }
                })
        }
        None => attempt(initial_creds.clone())
            .await
            .map_err(|e| fold_silent(e, &map_sas_err)),
    }
}

/// Per-attempt Azure policy: preserves everything the caller configured —
/// backoff, jitter, `max_elapsed`, `max_attempts`, and any user-set extra
/// retryable statuses — and changes exactly one thing: 403 is removed from the
/// retryable set so an expired-SAS 403 fast-fails to the SAS-refresh layer
/// (`run_azure_with_sas_refresh`) instead of being inline-retried. Mirrors the
/// clone-then-mutate shape of `s3_retry_policy` / `gcs_retry_policy` rather than
/// rebuilding from `RetryPolicy::default()` (which silently dropped user config).
fn azure_403_fastfail_policy(base: &RetryPolicy) -> RetryPolicy {
    let mut policy = base.clone();
    policy.extra_retryable_statuses.remove(&403);
    policy
}

/// Runs ONE Azure PUT attempt with in-line retry for non-403 transients.
/// 403 fast-fails as `AzureAttemptError::SasExpired`.
async fn azure_put_attempt(
    client: &reqwest::Client,
    url: &str,
    sas_token: &str,
    prepared: PreparedUpload,
    base: &RetryPolicy,
) -> Result<(), AzureAttemptError<AzureUploadError>> {
    let source = prepared.source.byte_source();
    let digest = prepared.digest;
    let (encryption_metadata, encryptor) = prepared.cse.map(|c| (c.metadata, c.encryptor)).unzip();

    let encryption_data_str = encryption_metadata
        .as_ref()
        .map(|enc_meta| {
            let encryption_data = build_encryption_metadata_json(enc_meta);
            serde_json::to_string(&encryption_data)
        })
        .transpose()
        .context(azure_upload_error::SerializationSnafu)
        .map_err(AzureAttemptError::Other)?;

    let mat_desc_str = encryption_metadata
        .as_ref()
        .map(|enc_meta| serde_json::to_string(&enc_meta.material_desc))
        .transpose()
        .context(azure_upload_error::SerializationSnafu)
        .map_err(AzureAttemptError::Other)?;

    let content_length = match &encryptor {
        Some(enc) => enc.cipher_len(),
        None => match &source {
            ByteSource::Bytes(b) => b.len() as i64,
            ByteSource::Path(p) => tokio::fs::metadata(p)
                .await
                .context(azure_upload_error::SourceIoSnafu)
                .map_err(AzureAttemptError::Other)?
                .len() as i64,
        },
    };

    let full_url = build_sas_url(url, sas_token);
    let policy = azure_403_fastfail_policy(base);
    let url_redacted = url_host_and_path(&full_url);
    let client = client.clone();

    let adapter = AzurePutAttemptRetry {
        url_redacted: url_redacted.clone(),
    };
    cloud_http::upload_with_retry(
        &policy,
        &adapter,
        &reqwest::Method::PUT,
        &url_redacted,
        async move || {
            let body = cloud_http::body_for(&source, encryptor.as_ref())
                .await
                .context(azure_upload_error::SourceIoSnafu)?;

            // TODO(SNOW-3701467): add an in-transit integrity checksum to match the S3 PUT path.
            let mut req = client
                .put(&full_url)
                .header("x-ms-blob-type", "BlockBlob")
                .header(AZURE_META_SFC_DIGEST, &digest)
                .header(reqwest::header::CONTENT_LENGTH, content_length)
                .body(body);

            if let Some(ref enc_str) = encryption_data_str {
                req = req.header(AZURE_META_ENCRYPTIONDATA, enc_str);
            }
            if let Some(ref md_str) = mat_desc_str {
                req = req.header(AZURE_META_MATDESC, md_str);
            }
            Ok(req)
        },
    )
    .await?;

    tracing::debug!("Azure blob upload successful");
    Ok(())
}

/// Adapter wiring Azure PUT-attempt errors into [`cloud_http::upload_with_retry`].
/// 403 fast-fails as `AzureAttemptError::SasExpired`.
struct AzurePutAttemptRetry {
    url_redacted: String,
}

impl UploadRetryAdapter for AzurePutAttemptRetry {
    type Err = AzureAttemptError<AzureUploadError>;
    type BuildErr = AzureUploadError;

    fn on_build_err(&self, e: AzureUploadError) -> AzureAttemptError<AzureUploadError> {
        AzureAttemptError::Other(e)
    }

    fn on_http_failure(
        &self,
        status_code: u16,
        body: String,
    ) -> AzureAttemptError<AzureUploadError> {
        map_http_to_attempt(
            status_code,
            sanitize_sas(body),
            &self.url_redacted,
            |status_code, body| AzureUploadError::AzureHttp {
                status_code,
                body,
                location: Location::default(),
            },
        )
    }

    fn on_transport(&self, e: reqwest::Error) -> AzureAttemptError<AzureUploadError> {
        AzureAttemptError::Other(AzureUploadError::Http {
            detail: sanitize_sas(e.to_string()),
            location: Location::default(),
        })
    }

    fn on_exhausted(&self, detail: String) -> AzureAttemptError<AzureUploadError> {
        AzureAttemptError::Other(AzureUploadError::RetryExhausted {
            detail: format!("Azure upload {detail}"),
            location: Location::default(),
        })
    }
}

/// Runs ONE Azure GET attempt with in-line retry for non-403 transients.
/// 403 always fast-fails as `AzureAttemptError::SasExpired`.
async fn azure_get_attempt(
    client: &reqwest::Client,
    full_url: &str,
    base: &RetryPolicy,
) -> Result<reqwest::Response, AzureAttemptError<AzureRequestError>> {
    let ctx = HttpContext::new(Method::GET, "azure-transfer");
    let policy = azure_403_fastfail_policy(base);

    let response = http_execute_with_retry(
        || client.get(full_url),
        &ctx,
        &policy,
        |r| async move { Ok(r) },
    )
    .await
    .map_err(|e| AzureAttemptError::Other(map_http_error(e)))?;

    if response.status().is_success() {
        return Ok(response);
    }

    let status_code = response.status().as_u16();
    tracing::debug!(
        status = status_code,
        url = %url_host_and_path(full_url),
        "Azure GET attempt returned non-success"
    );
    let body = sanitize_sas(cloud_http::read_error_body(response).await);
    Err(map_http_to_attempt(
        status_code,
        body,
        &url_host_and_path(full_url),
        |status_code, body| AzureRequestError::AzureHttp { status_code, body },
    ))
}

/// Issues the Azure blob GET under the SAS-refresh-on-403 layer.
/// With a refresher: 403 fast-fails → refresh → retry with new SAS.
/// Without a refresher: run once; 403 surfaces as a terminal error.
async fn azure_get_with_refresh(
    stage_info: &StageInfo,
    filename: &str,
    policy: &RetryPolicy,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<reqwest::Response, AzureDownloadError> {
    let client = create_azure_client(stage_info)?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let attempt = |creds: CloudCredentials| {
        let stage_info = with_creds(stage_info, creds);
        let key = key.clone();
        let client = client.clone();
        let policy = policy.clone();
        async move {
            let (url, sas_token) = resolve_url_and_token(&stage_info, &key)
                .map_err(|e| AzureAttemptError::Other(AzureDownloadError::from(e)))?;
            let full_url = build_sas_url(&url, sas_token.reveal());

            azure_get_attempt(&client, &full_url, &policy)
                .await
                .map_err(|e| e.map_other(AzureDownloadError::from))
        }
    };

    let result = run_azure_with_sas_refresh(
        refresher,
        &stage_info.creds,
        "GET",
        |e| azure_download_error::StageInfoRefreshSnafu.into_error(e),
        |status_code, body| AzureDownloadError::AzureHttp {
            status_code,
            body,
            location: Location::default(),
        },
        attempt,
    )
    .await;

    if let Err(AzureDownloadError::StageInfoRefresh { ref source, .. }) = result {
        tracing::error!(reason = %source, "Azure SAS refresh failed; download aborted");
    }

    result
}

/// Maps an `AzureUploadError` from an in-closure sub-request (HEAD, block PUT,
/// commit) to an attempt error: a 403 becomes the recoverable `SasExpired`
/// (routes to the refresh layer); everything else stays `Other`.
fn sas_expired_or_other(
    e: AzureUploadError,
    url: &str,
    sas_token: &str,
) -> AzureAttemptError<AzureUploadError> {
    match e {
        AzureUploadError::AzureHttp {
            status_code: 403,
            body,
            ..
        } => map_http_to_attempt(
            403,
            body,
            &url_host_and_path(&build_sas_url(url, sas_token)),
            // 403 always maps to SasExpired, so this closure is never invoked.
            |status_code, body| AzureUploadError::AzureHttp {
                status_code,
                body,
                location: Location::default(),
            },
        ),
        // Non-403 (incl. non-HTTP) errors keep their original value + location.
        other => AzureAttemptError::Other(other),
    }
}

/// Whole-file restart on SAS-403 (legacy JDBC/Go/.NET/ODBC parity).
/// Per-block resume is the PR-C follow-up (SNOW-3406384).
// One arg over S3/GCS (Azure adds `skip_upload_on_content_match`); a follow-up
// may bundle {multipart, policy, refresher} into an opts struct.
#[allow(clippy::too_many_arguments)]
pub(super) async fn upload_to_azure_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
    skip_upload_on_content_match: bool,
    multipart: MultipartParams,
    policy: &RetryPolicy,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadStatus, AzureUploadError> {
    let key = format!("{}{filename}", stage_info.key_prefix);
    let head_needed = super::head_needed(overwrite, skip_upload_on_content_match);

    // Build the client once — TLS config is creds-independent; only the SAS in
    // `stage_info.creds` rotates on refresh — and clone it into each attempt.
    let client = create_azure_client(stage_info)?;
    // On-cloud byte count (ciphertext length under CSE) decides single-PUT vs
    // multipart; computed once, outside the per-attempt closure.
    let body_len = multipart::upload_body_len(&prepared)
        .await
        .context(azure_upload_error::SourceIoSnafu)?;

    let attempt = |creds: CloudCredentials| {
        let prepared = prepared.clone();
        let stage_info = with_creds(stage_info, creds);
        let key = key.clone();
        let base = policy.clone();
        let client = client.clone();
        async move {
            let sas_token = match &stage_info.creds {
                CloudCredentials::Azure { sas_token } => sas_token.clone(),
                _ => {
                    return Err(AzureAttemptError::Other(
                        AzureUploadError::MissingAzureCredentials {
                            location: Location::default(),
                        },
                    ));
                }
            };
            let (url, _) = resolve_url_and_token(&stage_info, &key)
                .map_err(|e| AzureAttemptError::Other(AzureUploadError::from(e)))?;
            // 403 fast-fails to the refresh layer for every SAS-bearing request
            // in this attempt (HEAD, single PUT, blocks) — never inline-retried.
            let attempt_policy = azure_403_fastfail_policy(&base);

            // HEAD runs INSIDE the refresh closure so an expired-SAS 403 on the
            // existence/skip probe rotates the SAS and retries, rather than
            // failing closed before the PUT ever runs (mirrors S3/GCS).
            let remote = if head_needed {
                match send_head_to_azure_blob(&client, &url, &sas_token, &attempt_policy).await {
                    Ok(remote) => remote,
                    // Expired-SAS 403 on the MANDATORY existence check (!overwrite):
                    // route to refresh + retry rather than failing closed before
                    // the PUT ever runs. This is the bug this change fixes.
                    Err(
                        e @ AzureUploadError::AzureHttp {
                            status_code: 403, ..
                        },
                    ) if !overwrite => {
                        return Err(sas_expired_or_other(e, &url, sas_token.reveal()));
                    }
                    // Any other !overwrite HEAD failure: fail-CLOSED (can't risk
                    // clobbering an existing blob we couldn't verify).
                    Err(e) if !overwrite => return Err(AzureAttemptError::Other(e)),
                    // skip-match only (overwrite=true): fail-OPEN — a missed skip is
                    // just bandwidth; the PUT refreshes on its own 403 if expired.
                    Err(_) => None,
                }
            } else {
                None
            };

            let remote_head = match &remote {
                Some(h) => RemoteHead::Present {
                    digest: h.digest.as_deref(),
                },
                None => RemoteHead::Absent,
            };
            if let Some(status) = skip_upload_decision(
                LocationType::Azure,
                overwrite,
                skip_upload_on_content_match,
                &remote_head,
                &prepared.digest,
                &key,
            ) {
                return Ok(status);
            }

            if body_len >= multipart.threshold.bytes() {
                azure_multipart_upload(
                    &client,
                    &url,
                    sas_token.reveal(),
                    prepared,
                    body_len,
                    multipart.concurrency,
                    &attempt_policy,
                )
                .await
                .map_err(|e| sas_expired_or_other(e, &url, sas_token.reveal()))?;
            } else {
                azure_put_attempt(&client, &url, sas_token.reveal(), prepared, &base).await?;
            }
            Ok(UploadStatus::Uploaded)
        }
    };

    let result = run_azure_with_sas_refresh(
        refresher,
        &stage_info.creds,
        "PUT",
        |e| azure_upload_error::StageInfoRefreshSnafu.into_error(e),
        |status_code, body| AzureUploadError::AzureHttp {
            status_code,
            body,
            location: Location::default(),
        },
        attempt,
    )
    .await;

    if let Err(AzureUploadError::StageInfoRefresh { ref source, .. }) = result {
        tracing::error!(reason = %source, "Azure SAS refresh failed; upload aborted");
    }

    result
}

/// Downloads a file from Azure Blob Storage and returns data with optional encryption metadata.
/// For SSE stages the metadata headers will be absent and `None` is returned.
///
/// On a 403 the `refresher` (if any) fetches a fresh SAS and the GET retries.
pub async fn download_from_azure(
    stage_info: &StageInfo,
    filename: &str,
    policy: &RetryPolicy,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<DownloadResponse, AzureDownloadError> {
    let response = azure_get_with_refresh(stage_info, filename, policy, refresher).await?;

    // Extract metadata from response headers
    let (digest, file_metadata) = parse_azure_file_metadata(response.headers())?;

    let data = response
        .bytes()
        .await
        .map_err(|e| AzureRequestError::Http {
            detail: sanitize_sas(e.to_string()),
        })?
        .to_vec();
    let cloud_byte_count = data.len() as i64;

    Ok(DownloadResponse {
        data,
        digest,
        file_metadata,
        cloud_byte_count,
    })
}

/// Subset of HEAD response metadata consumed by the upload-or-skip path.
/// `None` digest means the header was absent — treat as "cannot compare".
#[derive(Debug, Clone)]
struct RemoteBlobHeader {
    digest: Option<String>,
}

/// Probes the blob with HEAD. Returns:
/// - `Ok(Some(header))` — 200, blob exists (digest captured).
/// - `Ok(None)` — 404, blob absent (safe to upload).
/// - `Err(_)` — any other outcome after retry exhaustion.
async fn send_head_to_azure_blob(
    client: &reqwest::Client,
    url: &str,
    sas_token: &SensitiveString,
    policy: &RetryPolicy,
) -> Result<Option<RemoteBlobHeader>, AzureUploadError> {
    match azure_request_with_retry(
        || client.head(build_sas_url(url, sas_token.reveal())),
        Method::HEAD,
        policy,
    )
    .await
    {
        Ok(response) => {
            let digest = response
                .headers()
                .get(AZURE_META_SFC_DIGEST)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            Ok(Some(RemoteBlobHeader { digest }))
        }
        Err(AzureRequestError::AzureHttp {
            status_code: 404, ..
        }) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Uploads `prepared` as an Azure block blob: `Put Block` ×N then `Put Block List`.
///
/// No abort step: Azure garbage-collects uncommitted blocks (~7 days) and never
/// charges for or exposes them, so a failed upload just leaves the blob uncommitted.
async fn azure_multipart_upload(
    client: &reqwest::Client,
    url: &str,
    sas_token: &str,
    prepared: PreparedUpload,
    body_len: u64,
    concurrency: usize,
    policy: &RetryPolicy,
) -> Result<(), AzureUploadError> {
    let chunk_size = multipart::compute_part_size(body_len, &MultipartConfig::AZURE)
        .context(azure_upload_error::FileTooLargeSnafu)?;
    // CSE params (cloud metadata + encryptor) are both present or both absent;
    // the metadata + digest ride on the `Put Block List` commit, the encryptor
    // lazily encrypts each block as the part-reader cuts it.
    let source = prepared.source.byte_source();
    let digest = prepared.digest;
    let (encryption_metadata, encryptor) = match prepared.cse {
        Some(c) => (Some(c.metadata), Some(c.encryptor)),
        None => (None, None),
    };
    let (encryption_data_str, mat_desc_str) =
        azure_encryption_header_strs(encryption_metadata.as_ref())?;
    let full_url = build_sas_url(url, sas_token);
    let full_url = full_url.as_str();

    // Blocks read sequentially from the (optionally encrypting) source, staged
    // concurrently.
    let parts_rx =
        multipart::spawn_part_reader(source, encryptor, chunk_size as usize, concurrency);

    let mut block_numbers: Vec<i32> = ReceiverStream::new(parts_rx)
        .map(|part| async move {
            let part = part.context(azure_upload_error::SourceIoSnafu)?;
            let number = part.number;
            azure_put_block(client, full_url, part, policy).await?;
            Ok::<i32, AzureUploadError>(number)
        })
        .buffer_unordered(concurrency)
        .try_collect()
        .await?;

    // A block blob with zero blocks makes Azure reject `Put Block List` (or
    // commit an empty blob); surface a clear error instead. Unreachable on the
    // normal path (multipart requires `body_len >= threshold >= 1`) — this guards
    // a source truncated to 0 bytes between the size stat and the first read.
    // Mirrors the S3 empty-parts guard in `s3_transfer.rs`.
    if block_numbers.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "no blocks produced (source became empty before read)",
        ))
        .context(azure_upload_error::SourceIoSnafu);
    }

    // `Put Block List` must name the blocks in blob order; uploads finished out
    // of order, so sort by the original part number.
    block_numbers.sort_unstable();

    azure_put_block_list(
        client,
        full_url,
        &block_numbers,
        &digest,
        encryption_data_str.as_deref(),
        mat_desc_str.as_deref(),
        policy,
    )
    .await?;

    tracing::debug!(
        "Azure block-blob upload committed ({} blocks)",
        block_numbers.len()
    );
    Ok(())
}

/// Fixed-width, base64 block id for a 1-based part number. Azure requires every
/// block id in an upload to be the same length; eight digits covers the 50 000
/// block ceiling with room to spare.
fn azure_block_id(number: i32) -> String {
    BASE64_ENGINE.encode(format!("block{number:08}"))
}

/// Stages one block via `Put Block` (no metadata; that rides on the commit).
async fn azure_put_block(
    client: &reqwest::Client,
    full_url: &str,
    part: multipart::UploadPart,
    policy: &RetryPolicy,
) -> Result<(), AzureUploadError> {
    let block_id = azure_block_id(part.number);
    let body = part.body;
    let content_length = body.len();
    let client = client.clone();
    let url_owned = full_url.to_string();
    azure_upload_with_retry(
        async move || {
            Ok(client
                .put(&url_owned)
                .query(&[("comp", "block"), ("blockid", block_id.as_str())])
                .header(reqwest::header::CONTENT_LENGTH, content_length)
                .body(reqwest::Body::from(body.clone())))
        },
        &Method::PUT,
        full_url,
        policy,
    )
    .await
}

/// Commits the staged blocks with `Put Block List`, attaching the digest and
/// (for CSE) encryption metadata headers.
async fn azure_put_block_list(
    client: &reqwest::Client,
    full_url: &str,
    block_numbers: &[i32],
    digest: &str,
    encryption_data_str: Option<&str>,
    mat_desc_str: Option<&str>,
    policy: &RetryPolicy,
) -> Result<(), AzureUploadError> {
    let body = build_block_list_xml(block_numbers);
    let client = client.clone();
    let url_owned = full_url.to_string();
    let digest = digest.to_string();
    let encryption_data_str = encryption_data_str.map(str::to_string);
    let mat_desc_str = mat_desc_str.map(str::to_string);
    azure_upload_with_retry(
        async move || {
            let mut req = client
                .put(&url_owned)
                .query(&[("comp", "blocklist")])
                .header(AZURE_META_SFC_DIGEST, &digest)
                .body(body.clone());
            if let Some(enc) = &encryption_data_str {
                req = req.header(AZURE_META_ENCRYPTIONDATA, enc);
            }
            if let Some(md) = &mat_desc_str {
                req = req.header(AZURE_META_MATDESC, md);
            }
            Ok(req)
        },
        &Method::PUT,
        full_url,
        policy,
    )
    .await
}

/// Builds the `Put Block List` request body naming every block (in order) as
/// `<Latest>` (i.e. prefer an uncommitted block over any same-id committed one).
fn build_block_list_xml(block_numbers: &[i32]) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<BlockList>");
    for &number in block_numbers {
        xml.push_str("<Latest>");
        xml.push_str(&azure_block_id(number));
        xml.push_str("</Latest>");
    }
    xml.push_str("</BlockList>");
    xml
}

/// Serializes the CSE encryption-data and material-description metadata header
/// values from the (optional) CSE metadata (both `None` for SSE). Shared by the
/// single `Put Blob` and the multipart `Put Block List` paths.
fn azure_encryption_header_strs(
    encryption_metadata: Option<&EncryptedFileMetadata>,
) -> Result<(Option<String>, Option<String>), AzureUploadError> {
    let encryption_data_str = encryption_metadata
        .map(|enc_meta| serde_json::to_string(&build_encryption_metadata_json(enc_meta)))
        .transpose()
        .context(azure_upload_error::SerializationSnafu)?;
    let mat_desc_str = encryption_metadata
        .map(|enc_meta| serde_json::to_string(&enc_meta.material_desc))
        .transpose()
        .context(azure_upload_error::SerializationSnafu)?;
    Ok((encryption_data_str, mat_desc_str))
}

// --- Retry logic (delegates to http::retry) ---

/// Executes an Azure HTTP request with in-line retry for non-403 transients.
/// Its HEAD/ranged-GET callers pass `azure_403_fastfail_policy`, so a 403 fast-fails
/// out to the SAS-refresh layer (`run_azure_with_sas_refresh`) instead of retrying.
async fn azure_request_with_retry<F>(
    build_request: F,
    method: Method,
    policy: &RetryPolicy,
) -> Result<reqwest::Response, AzureRequestError>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let ctx = HttpContext::new(method, "azure-transfer");

    let response = http_execute_with_retry(build_request, &ctx, policy, |r| async move { Ok(r) })
        .await
        .map_err(map_http_error)?;

    tracing::info!(status = response.status().as_u16(), "HTTP response");

    if response.status().is_success() {
        return Ok(response);
    }

    let status_code = response.status().as_u16();
    // Scrub SAS signatures from the URL that Azure error bodies often echo.
    // TODO(SNOW-3406377): centralise SAS redaction instead of per-call-site scrubbing.
    let body = sanitize_sas(cloud_http::read_error_body(response).await);
    Err(AzureRequestError::AzureHttp { status_code, body })
}

/// Adapter that wires `AzureUploadError` variants into the shared
/// [`cloud_http::upload_with_retry`] loop. Azure has no special-status hook
/// (unlike GCS' 401), but it does run `sanitize_sas` on every transport-error
/// string before surfacing it.
struct AzureUploadRetry;

impl UploadRetryAdapter for AzureUploadRetry {
    type Err = AzureUploadError;
    type BuildErr = AzureUploadError;

    fn on_build_err(&self, e: AzureUploadError) -> AzureUploadError {
        e
    }

    fn on_http_failure(&self, status_code: u16, body: String) -> AzureUploadError {
        // Azure error bodies often echo the request URL, so scrub SAS signatures
        // before stuffing the body into the user-facing error variant.
        azure_upload_error::AzureHttpSnafu {
            status_code,
            body: sanitize_sas(body),
        }
        .build()
    }

    fn on_transport(&self, e: reqwest::Error) -> AzureUploadError {
        azure_upload_error::HttpSnafu {
            detail: sanitize_sas(e.to_string()),
        }
        .build()
    }

    fn on_exhausted(&self, detail: String) -> AzureUploadError {
        azure_upload_error::RetryExhaustedSnafu {
            detail: format!("Azure upload {detail}"),
        }
        .build()
    }
}

/// Executes an Azure upload with retry, accepting a **fallible** request-builder closure.
///
/// Unlike `azure_request_with_retry`, the closure may return `Err(AzureUploadError)`
/// (e.g. if the source file cannot be opened on a retry attempt). A build failure
/// is treated as non-retryable and propagated immediately.
///
/// Takes the injected `&RetryPolicy` (not a bare `max_attempts`) for the same
/// reason as `azure_request_with_retry`: tests can supply zero backoff.
async fn azure_upload_with_retry<F>(
    build_request: F,
    method: &reqwest::Method,
    url: &str,
    policy: &RetryPolicy,
) -> Result<(), AzureUploadError>
where
    F: AsyncFn() -> Result<reqwest::RequestBuilder, AzureUploadError>,
{
    cloud_http::upload_with_retry(policy, &AzureUploadRetry, method, url, build_request).await
}

fn map_http_error(e: HttpError) -> AzureRequestError {
    match e {
        HttpError::Transport { source, .. } => AzureRequestError::Http {
            detail: sanitize_sas(source.to_string()),
        },
        other => AzureRequestError::RetryExhausted {
            detail: sanitize_sas(other.to_string()),
        },
    }
}

// --- Helpers ---

fn create_azure_client(stage_info: &StageInfo) -> Result<reqwest::Client, AzureRequestError> {
    let builder = crate::tls::client::configure_tls_builder(
        reqwest::Client::builder().timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
        &stage_info.tls_config,
        stage_info.crl_worker.clone(),
    )
    .map_err(|e| {
        HttpSnafu {
            detail: e.to_string(),
        }
        .build()
    })?;
    builder.build().map_err(|e| {
        HttpSnafu {
            detail: e.to_string(),
        }
        .build()
    })
}

/// Constructs the Azure Blob Storage URL and extracts the SAS token from stage info.
///
/// URL format: `https://{storageAccount}.{blob_endpoint}/{container}/{blob_path}`
///
/// The endpoint value comes from Snowflake and may vary by environment
/// (commercial, government, China). It is used as-is from the server response,
/// with a `blob.` prefix prepended only if absent.
fn resolve_url_and_token<'a>(
    stage_info: &'a StageInfo,
    key: &str,
) -> Result<(String, &'a SensitiveString), AzureRequestError> {
    let sas_token = match &stage_info.creds {
        CloudCredentials::Azure { sas_token } => sas_token,
        _ => return Err(AzureRequestError::MissingAzureCredentials),
    };

    let url = build_azure_url(stage_info, key)?;
    Ok((url, sas_token))
}

/// Builds the Azure Blob Storage URL for a given object key.
///
/// When `endpoint` contains a URL scheme (`http://` or `https://`), it is used directly
/// as the base URL. This supports Azure-compatible local emulators (e.g. Azurite) and
/// testing with mock servers. Otherwise, the standard Azure URL pattern
/// `https://{storageAccount}.blob.{endpoint}/{container}/{key}` is used.
fn build_azure_url(stage_info: &StageInfo, key: &str) -> Result<String, AzureRequestError> {
    let encoded_key = percent_encode_path(key);

    // If endpoint contains a scheme, use it directly (e.g. Azurite or test servers).
    if let Some(ref ep) = stage_info.endpoint
        && (ep.starts_with("http://") || ep.starts_with("https://"))
    {
        return Ok(format!("{ep}/{}/{encoded_key}", stage_info.bucket));
    }

    // Standard Azure URL: https://{account}.blob.{endpoint}/{bucket}/{key}
    let storage_account = stage_info
        .storage_account
        .as_ref()
        .filter(|sa| !sa.is_empty())
        .ok_or(AzureRequestError::MissingMetadata {
            field: "storage_account".to_string(),
        })?;

    let raw_endpoint = stage_info
        .endpoint
        .as_deref()
        .unwrap_or("blob.core.windows.net");

    // Normalize the endpoint to a bare hostname (strip any URL scheme or path).
    let endpoint = {
        let without_scheme = raw_endpoint
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(raw_endpoint);
        without_scheme
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or(without_scheme)
    };

    // The Snowflake server may provide the endpoint with or without the "blob." prefix.
    // Azure Government uses "blob.core.usgovcloudapi.net", Azure China uses
    // "blob.core.chinacloudapi.cn". We prepend "blob." only when it's missing.
    let blob_endpoint = if endpoint.starts_with("blob.") {
        endpoint.to_string()
    } else {
        format!("blob.{endpoint}")
    };

    Ok(format!(
        "https://{storage_account}.{blob_endpoint}/{}/{encoded_key}",
        stage_info.bucket
    ))
}

/// Appends the SAS token to a URL as a query parameter.
fn build_sas_url(base_url: &str, sas_token: &str) -> String {
    let token = sas_token.strip_prefix('?').unwrap_or(sas_token);
    let separator = if base_url.contains('?') { "&" } else { "?" };
    format!("{base_url}{separator}{token}")
}

fn try_get_header(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<Option<String>, AzureDownloadError> {
    match headers.get(name) {
        Some(value) => {
            let s = value
                .to_str()
                .context(azure_download_error::InvalidHeaderValueSnafu)?;
            Ok(Some(s.to_string()))
        }
        None => Ok(None),
    }
}

/// Parses the `sfcdigest` and (for CSE) the `encryptiondata` / `matdesc`
/// metadata headers from an Azure response. All metadata is absent for SSE
/// stages; when `encryptiondata` is present, `matdesc` must be too. Shared by
/// every Azure download path (buffered, streaming, ranged).
fn parse_azure_file_metadata(
    headers: &reqwest::header::HeaderMap,
) -> Result<(Option<String>, Option<EncryptedFileMetadata>), AzureDownloadError> {
    let digest = try_get_header(headers, AZURE_META_SFC_DIGEST)?;

    let file_metadata = match try_get_header(headers, AZURE_META_ENCRYPTIONDATA)? {
        Some(encryption_data_str) => {
            let enc_data: EncryptionData = serde_json::from_str(&encryption_data_str)
                .context(azure_download_error::DeserializationSnafu)?;
            let mat_desc_str = try_get_header(headers, AZURE_META_MATDESC)?.context(
                azure_download_error::MissingMetadataSnafu {
                    field: AZURE_META_MATDESC,
                },
            )?;
            let material_desc: MaterialDescription = serde_json::from_str(&mat_desc_str)
                .context(azure_download_error::DeserializationSnafu)?;
            Some(EncryptedFileMetadata {
                encrypted_key: enc_data.wrapped_content_key.encrypted_key,
                iv: enc_data.content_encryption_iv,
                material_desc,
            })
        }
        None => None,
    };
    Ok((digest, file_metadata))
}

/// One Azure routing-HEAD attempt (Get Blob Properties) for size + metadata.
/// 403 fast-fails as `AzureAttemptError::SasExpired` so the refresh layer can
/// rotate the SAS before re-driving; non-403 transients retry in-line.
async fn azure_head_attempt(
    client: &reqwest::Client,
    full_url: &str,
    base: &RetryPolicy,
) -> Result<reqwest::Response, AzureAttemptError<AzureDownloadError>> {
    let policy = azure_403_fastfail_policy(base);
    match azure_request_with_retry(|| client.head(full_url), Method::HEAD, &policy).await {
        Ok(response) => Ok(response),
        Err(AzureRequestError::AzureHttp { status_code, body }) => Err(map_http_to_attempt(
            status_code,
            body,
            &url_host_and_path(full_url),
            |status_code, body| AzureDownloadError::AzureHttp {
                status_code,
                body,
                location: Location::default(),
            },
        )),
        Err(other) => Err(AzureAttemptError::Other(AzureDownloadError::from(other))),
    }
}

/// Wraps `azure_range_download` for the refresh layer: a 403 on any range
/// (expired SAS) fast-fails as `SasExpired`, so the whole ranged download
/// re-drives with a fresh SAS (whole-download restart, like the PUT multipart
/// path); other errors pass through as `Other`.
#[allow(clippy::too_many_arguments)]
async fn azure_range_attempt(
    client: &reqwest::Client,
    full_url: &str,
    content_length: u64,
    chunk_size: u64,
    concurrency: usize,
    base: &RetryPolicy,
    unsafe_file_write: bool,
    spill_target: cloud_http::CloudSpillTarget<'_>,
) -> Result<cloud_http::CloudSpilledBody, AzureAttemptError<AzureDownloadError>> {
    let policy = azure_403_fastfail_policy(base);
    azure_range_download(
        client,
        full_url,
        content_length,
        chunk_size,
        concurrency,
        &policy,
        unsafe_file_write,
        spill_target,
    )
    .await
    .map_err(|e| match e {
        AzureDownloadError::AzureHttp {
            status_code: 403,
            body,
            ..
        } => map_http_to_attempt(
            403,
            body,
            &url_host_and_path(full_url),
            // 403 always maps to SasExpired, so this closure is never invoked.
            |status_code, body| AzureDownloadError::AzureHttp {
                status_code,
                body,
                location: Location::default(),
            },
        ),
        other => AzureAttemptError::Other(other),
    })
}

/// Downloads a file from Azure and returns a [`CloudStreamingDownload`] whose
/// `body` yields the ciphertext via a sync `Read`. A HEAD probe (Python /
/// JDBC / ODBC parity) yields the blob size + metadata: blobs at or above
/// `multipart.threshold` are fetched with parallel ranged GETs into a tempfile
/// (read back through `SpilledReader`); smaller ones stream a single GET straight
/// off the network. On a 403 the `refresher` (if any) rotates the SAS and the
/// HEAD + GET retry with fresh credentials.
pub async fn download_from_azure_streaming(
    stage_info: &StageInfo,
    filename: &str,
    multipart: MultipartParams,
    policy: &RetryPolicy,
    unsafe_file_write: bool,
    spill_target: cloud_http::CloudSpillTarget<'_>,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<CloudStreamingDownload, AzureDownloadError> {
    // Build the client once (TLS config is creds-independent); only the SAS in
    // `stage_info.creds` rotates on refresh.
    let client = create_azure_client(stage_info)?;
    let key = format!("{}{filename}", stage_info.key_prefix);

    // The routing HEAD + the ranged/single GET all run INSIDE the refresh
    // closure, so an expired-SAS 403 on any of them rotates the SAS and
    // re-drives the whole download — symmetric with the PUT path.
    let attempt = |creds: CloudCredentials| {
        let stage_info = with_creds(stage_info, creds);
        let key = key.clone();
        let client = client.clone();
        let base = policy.clone();
        async move {
            let (url, sas_token) = resolve_url_and_token(&stage_info, &key)
                .map_err(|e| AzureAttemptError::Other(AzureDownloadError::from(e)))?;
            let full_url = build_sas_url(&url, sas_token.reveal());

            // Routing HEAD for size + metadata; 403 fast-fails to SasExpired.
            let head = azure_head_attempt(&client, &full_url, &base).await?;
            // Read the size from Content-Length rather than reqwest's
            // `content_length()`, unreliable for HEAD (no body). Real Azure always
            // sends Content-Length on Get Blob Properties.
            let content_length = head
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let (digest, file_metadata) =
                parse_azure_file_metadata(head.headers()).map_err(AzureAttemptError::Other)?;
            // Git-stage objects carry encryption headers but no sfcdigest — non-CSE.
            let cse_info = match (file_metadata, digest) {
                (Some(metadata), Some(digest)) => Some(CseDownloadInfo { metadata, digest }),
                (Some(_), None) => {
                    tracing::debug!(
                        "Azure: encryptiondata present but {AZURE_META_SFC_DIGEST} absent \
                         (git-stage object); treating as non-CSE"
                    );
                    None
                }
                (None, _) => None,
            };

            // Route on size: parallel ranged GETs into a spill file above the
            // threshold, a single streamed GET below. Both fast-fail 403.
            let (body, cloud_bytes_read) = if content_length >= multipart.threshold.bytes() {
                let chunk_size =
                    multipart::compute_part_size(content_length, &MultipartConfig::AZURE)
                        .context(azure_download_error::FileTooLargeSnafu)
                        .map_err(AzureAttemptError::Other)?;
                tracing::debug!(
                    "Azure ranged download: content_length={content_length} \
                     chunk_size={chunk_size} concurrency={}",
                    multipart.concurrency
                );
                let spilled = azure_range_attempt(
                    &client,
                    &full_url,
                    content_length,
                    chunk_size,
                    multipart.concurrency,
                    &base,
                    unsafe_file_write,
                    spill_target,
                )
                .await?;
                (
                    cloud_http::CloudDownloadBody::Spilled(spilled),
                    std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                )
            } else {
                let response = azure_get_attempt(&client, &full_url, &base)
                    .await
                    .map_err(|e| e.map_other(AzureDownloadError::from))?;
                let reader = cloud_http::spawn_byte_stream_producer(response);
                let handle = reader.bytes_read_handle();
                (
                    cloud_http::CloudDownloadBody::Streamed(Box::new(reader)),
                    handle,
                )
            };

            Ok(CloudStreamingDownload {
                cloud_byte_count: content_length as i64,
                cse_info,
                cloud_bytes_read,
                body,
            })
        }
    };

    let result = run_azure_with_sas_refresh(
        refresher,
        &stage_info.creds,
        "GET",
        |e| azure_download_error::StageInfoRefreshSnafu.into_error(e),
        |status_code, body| AzureDownloadError::AzureHttp {
            status_code,
            body,
            location: Location::default(),
        },
        attempt,
    )
    .await;

    if let Err(AzureDownloadError::StageInfoRefresh { ref source, .. }) = result {
        tracing::error!(reason = %source, "Azure SAS refresh failed; streaming download aborted");
    }

    result
}

/// Downloads the blob with parallel ranged GETs into a pre-allocated file,
/// returning the assembled [`CloudSpilledBody`](cloud_http::CloudSpilledBody).
/// Ranges are fetched up to `concurrency` at a time and written at their
/// absolute offset, so out-of-order completion is fine. Thin wrapper around
/// the shared [`cloud_http::assemble_ranged_download`] helper.
#[allow(clippy::too_many_arguments)]
async fn azure_range_download(
    client: &reqwest::Client,
    full_url: &str,
    content_length: u64,
    chunk_size: u64,
    concurrency: usize,
    policy: &RetryPolicy,
    unsafe_file_write: bool,
    target: cloud_http::CloudSpillTarget<'_>,
) -> Result<cloud_http::CloudSpilledBody, AzureDownloadError> {
    let mk_temp_err = |detail: String| azure_download_error::TempFileSnafu { detail }.build();

    cloud_http::assemble_ranged_download(
        content_length,
        chunk_size,
        concurrency,
        target,
        unsafe_file_write,
        mk_temp_err,
        mk_temp_err,
        move |range| async move { azure_get_range(client, full_url, &range, policy).await },
    )
    .await
}

/// Ranged GET of `[range.start, range.end]`, returning the body bytes.
async fn azure_get_range(
    client: &reqwest::Client,
    full_url: &str,
    range: &multipart::DownloadRange,
    policy: &RetryPolicy,
) -> Result<Bytes, AzureDownloadError> {
    let range_header = format!("bytes={}-{}", range.start, range.end);
    let response = azure_request_with_retry(
        || {
            client
                .get(full_url)
                .header(reqwest::header::RANGE, &range_header)
        },
        Method::GET,
        policy,
    )
    .await?;
    // A body-read failure after a successful response maps to the same
    // SAS-scrubbed transport error the buffered `download_from_azure` uses.
    let bytes = response.bytes().await.map_err(|e| {
        AzureDownloadError::from(AzureRequestError::Http {
            detail: sanitize_sas(e.to_string()),
        })
    })?;
    // The 206-vs-200 / truncation guard (bytes.len() == expected) lives in the
    // shared cloud_http::assemble_ranged_download, applied uniformly to all clouds.
    Ok(bytes)
}

/// Removes SAS token signature values from a string to prevent credential leakage in logs.
/// Handles multiple `sig=` occurrences (e.g., when error bodies echo URLs more than once).
fn sanitize_sas(input: String) -> String {
    let mut result = String::with_capacity(input.len());
    let mut remaining = input.as_str();
    while let Some(start) = remaining.find("sig=") {
        result.push_str(&remaining[..start]);
        result.push_str("sig=REDACTED");
        let value_start = start + 4;
        let value_end = remaining[value_start..]
            .find('&')
            .map(|i| value_start + i)
            .unwrap_or(remaining.len());
        remaining = &remaining[value_end..];
    }
    result.push_str(remaining);
    result
}

/// Host + path of a URL for logging, dropping the entire query string (SAS
/// `sig`/`sv`/`se`/… included) per `ud-log-url-in-error-host-and-path`.
/// `sanitize_sas` remains for scrubbing unstructured error-body text.
fn url_host_and_path(url: &str) -> String {
    reqwest::Url::parse(url)
        .map(|u| format!("{}{}", u.host_str().unwrap_or("<unknown-host>"), u.path()))
        .unwrap_or_else(|_| "<unparseable-url>".to_string())
}

// --- Error types ---

/// Internal error for shared helpers (retry, client creation, URL resolution).
/// Converted into `AzureUploadError` or `AzureDownloadError` via `From` impls.
#[derive(Debug, Snafu)]
enum AzureRequestError {
    #[snafu(display("Azure HTTP error: {detail}"))]
    Http { detail: String },
    #[snafu(display("Azure request failed: HTTP {status_code}: {body}"))]
    AzureHttp { status_code: u16, body: String },
    #[snafu(display("Missing Azure credentials"))]
    MissingAzureCredentials,
    #[snafu(display("Missing Azure metadata: {field}"))]
    MissingMetadata { field: String },
    #[snafu(display("Azure retry exhausted: {detail}"))]
    RetryExhausted { detail: String },
}

impl From<AzureRequestError> for AzureUploadError {
    fn from(e: AzureRequestError) -> Self {
        match e {
            AzureRequestError::Http { detail } => azure_upload_error::HttpSnafu { detail }.build(),
            AzureRequestError::AzureHttp { status_code, body } => {
                azure_upload_error::AzureHttpSnafu { status_code, body }.build()
            }
            AzureRequestError::MissingAzureCredentials => {
                azure_upload_error::MissingAzureCredentialsSnafu.build()
            }
            AzureRequestError::MissingMetadata { field } => {
                azure_upload_error::MissingMetadataSnafu { field }.build()
            }
            AzureRequestError::RetryExhausted { detail } => {
                azure_upload_error::RetryExhaustedSnafu { detail }.build()
            }
        }
    }
}

impl From<AzureRequestError> for AzureDownloadError {
    fn from(e: AzureRequestError) -> Self {
        match e {
            AzureRequestError::Http { detail } => {
                azure_download_error::HttpSnafu { detail }.build()
            }
            AzureRequestError::AzureHttp { status_code, body } => {
                azure_download_error::AzureHttpSnafu { status_code, body }.build()
            }
            AzureRequestError::MissingAzureCredentials => {
                azure_download_error::MissingAzureCredentialsSnafu.build()
            }
            AzureRequestError::MissingMetadata { field } => {
                azure_download_error::MissingMetadataSnafu { field }.build()
            }
            AzureRequestError::RetryExhausted { detail } => {
                azure_download_error::RetryExhaustedSnafu { detail }.build()
            }
        }
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum AzureUploadError {
    #[snafu(display("Failed to read upload source data"))]
    SourceIo {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure HTTP error: {detail}"))]
    Http {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure request failed: HTTP {status_code}: {body}"))]
    AzureHttp {
        status_code: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize Azure metadata"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing Azure credentials"))]
    MissingAzureCredentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing Azure metadata: {field}"))]
    MissingMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure retry exhausted: {detail}"))]
    RetryExhausted {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("File too large for Azure block-blob upload"))]
    FileTooLarge {
        #[snafu(source(from(multipart::FileTooLargeError, Box::new)))]
        source: Box<multipart::FileTooLargeError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to refresh Azure stage credentials after 403"))]
    StageInfoRefresh {
        #[snafu(source(from(StageInfoRefreshError, Box::new)))]
        source: Box<StageInfoRefreshError>,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum AzureDownloadError {
    #[snafu(display("Azure HTTP error: {detail}"))]
    Http {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure request failed: HTTP {status_code}: {body}"))]
    AzureHttp {
        status_code: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to deserialize Azure metadata"))]
    Deserialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing Azure metadata: {field}"))]
    MissingMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid Azure header value"))]
    InvalidHeaderValue {
        source: reqwest::header::ToStrError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing Azure credentials"))]
    MissingAzureCredentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure retry exhausted: {detail}"))]
    RetryExhausted {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Object too large to download from Azure"))]
    FileTooLarge {
        #[snafu(source(from(multipart::FileTooLargeError, Box::new)))]
        source: Box<multipart::FileTooLargeError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to stage Azure ranged download to a temp file: {detail}"))]
    TempFile {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to refresh Azure stage credentials after 403"))]
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
    use super::super::prepared_upload_with_digest;
    use super::*;
    use crate::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
    use crate::config::retry::Jitter;
    use crate::sensitive::SensitiveString;
    use bytes::Bytes;

    // Zero-backoff test policy lives in `file_manager::internal` so the in-crate
    // and external integration tests share one definition (the base put/get
    // policy; `azure_403_fastfail_policy` derives the per-attempt policy from it).
    // Aliased so call sites read `test_policy(..)`.
    use crate::file_manager::internal::FakeStageInfoRefresher;
    use crate::file_manager::internal::azure_test_retry_policy as test_policy;

    fn make_stage_info(overrides: StageInfoOverrides) -> StageInfo {
        StageInfo {
            location_type: super::super::types::LocationType::Azure,
            bucket: overrides.bucket.unwrap_or("my-container".to_string()),
            key_prefix: overrides.key_prefix.unwrap_or("prefix/".to_string()),
            region: overrides.region.unwrap_or("eastus2".to_string()),
            creds: overrides.creds.unwrap_or(CloudCredentials::Azure {
                sas_token: SensitiveString::from("fake-sas-token"),
            }),
            endpoint: overrides.endpoint,
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
            storage_account: overrides
                .storage_account
                .or(Some("mystorageaccount".to_string())),
        }
    }

    #[derive(Default)]
    struct StageInfoOverrides {
        bucket: Option<String>,
        key_prefix: Option<String>,
        region: Option<String>,
        creds: Option<CloudCredentials>,
        endpoint: Option<String>,
        storage_account: Option<String>,
    }

    // ---------------------------------------------------------------
    // 1. URL construction
    // ---------------------------------------------------------------

    #[test]
    fn url_default_endpoint() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.windows.net/my-container/prefix/file.csv.gz"
        );
    }

    #[test]
    fn url_custom_endpoint_with_blob_prefix() {
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("blob.core.usgovcloudapi.net".to_string()),
            ..Default::default()
        });
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.usgovcloudapi.net/my-container/prefix/file.csv.gz"
        );
    }

    #[test]
    fn url_custom_endpoint_without_blob_prefix() {
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("core.chinacloudapi.cn".to_string()),
            ..Default::default()
        });
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.chinacloudapi.cn/my-container/prefix/file.csv.gz"
        );
    }

    #[test]
    fn url_endpoint_without_trailing_slash() {
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("core.windows.net".to_string()),
            ..Default::default()
        });
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.windows.net/my-container/prefix/file.csv.gz"
        );
    }

    #[test]
    fn url_missing_storage_account() {
        let mut stage = make_stage_info(StageInfoOverrides::default());
        stage.storage_account = None;
        let result = build_azure_url(&stage, "prefix/file.csv.gz");
        assert!(result.is_err());
    }

    #[test]
    fn url_with_nested_path() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_azure_url(&stage, "deep/nested/path/file.csv.gz").unwrap();
        assert!(url.contains("deep/nested/path/file.csv.gz"));
    }

    // ---------------------------------------------------------------
    // 2. SAS token handling
    // ---------------------------------------------------------------

    #[test]
    fn sas_url_appends_token() {
        let url = build_sas_url(
            "https://example.blob.core.windows.net/c/f",
            "sv=2021&sig=abc",
        );
        assert_eq!(
            url,
            "https://example.blob.core.windows.net/c/f?sv=2021&sig=abc"
        );
    }

    #[test]
    fn sas_url_strips_leading_question_mark() {
        let url = build_sas_url(
            "https://example.blob.core.windows.net/c/f",
            "?sv=2021&sig=abc",
        );
        assert_eq!(
            url,
            "https://example.blob.core.windows.net/c/f?sv=2021&sig=abc"
        );
    }

    #[test]
    fn resolve_with_sas_token() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let (url, token) = resolve_url_and_token(&stage, "prefix/file.csv.gz").unwrap();
        assert!(url.starts_with("https://mystorageaccount.blob.core.windows.net/"));
        assert_eq!(token.reveal(), "fake-sas-token");
    }

    #[test]
    fn resolve_with_s3_creds_returns_error() {
        let stage = make_stage_info(StageInfoOverrides {
            creds: Some(CloudCredentials::S3 {
                aws_key_id: "key".to_string(),
                aws_secret_key: SensitiveString::from("secret"),
                aws_token: SensitiveString::from("token"),
            }),
            ..Default::default()
        });
        let result = resolve_url_and_token(&stage, "prefix/file.csv.gz");
        assert!(matches!(
            result,
            Err(AzureRequestError::MissingAzureCredentials)
        ));
    }

    // ---------------------------------------------------------------
    // 3. Retry policy configuration
    // ---------------------------------------------------------------

    fn base_policy() -> RetryPolicy {
        use crate::config::param_store::ParamStore;
        RetryPolicy::put_get(&ParamStore::new())
    }

    #[test]
    fn azure_403_fastfail_policy_excludes_403_from_retryable() {
        // Even if the caller's base policy lists 403 as retryable, the per-attempt
        // policy must exclude it so an expired-SAS 403 fast-fails to the refresh
        // layer instead of being inline-retried.
        let mut base = base_policy();
        base.extra_retryable_statuses.insert(403);
        let policy = azure_403_fastfail_policy(&base);
        assert!(
            !policy.extra_retryable_statuses.contains(&403),
            "403 must be excluded so it fast-fails to the SAS-refresh layer"
        );
    }

    #[test]
    fn azure_403_fastfail_policy_preserves_user_backoff_and_extra_statuses() {
        // Regression guard: the per-attempt policy must carry the caller's
        // backoff, jitter, max_elapsed, max_attempts, and user-configured extra
        // retryable statuses unchanged — only 403 is removed. The previous
        // implementation rebuilt from `RetryPolicy::default()`, silently dropping
        // all of these (mirrors GCS's `gcs_retry_policy_preserves_*` guard).
        let mut base = base_policy();
        base.max_attempts = 25;
        base.extra_retryable_statuses.insert(429);
        let policy = azure_403_fastfail_policy(&base);
        assert_eq!(policy.max_attempts, 25, "max_attempts preserved");
        assert_eq!(
            policy.max_elapsed, base.max_elapsed,
            "max_elapsed preserved"
        );
        assert_eq!(
            policy.backoff.base, base.backoff.base,
            "backoff base preserved"
        );
        assert_eq!(
            policy.backoff.cap, base.backoff.cap,
            "backoff cap preserved"
        );
        assert_eq!(
            policy.backoff.factor, base.backoff.factor,
            "backoff factor preserved"
        );
        assert!(
            matches!(policy.backoff.jitter, Jitter::Decorrelated),
            "jitter preserved (not forced to None)"
        );
        assert!(
            policy.extra_retryable_statuses.contains(&429),
            "user-configured extra retryable status preserved"
        );
    }

    // ---------------------------------------------------------------
    // 4. SAS token sanitization
    // ---------------------------------------------------------------

    #[test]
    fn sanitize_sas_redacts_signature() {
        let input =
            "https://acct.blob.core.windows.net/c/f?sv=2021&sig=secret123&se=2026".to_string();
        let result = sanitize_sas(input);
        assert_eq!(
            result,
            "https://acct.blob.core.windows.net/c/f?sv=2021&sig=REDACTED&se=2026"
        );
    }

    #[test]
    fn sanitize_sas_handles_sig_at_end() {
        let input = "https://acct.blob.core.windows.net/c/f?sv=2021&sig=secret123".to_string();
        let result = sanitize_sas(input);
        assert_eq!(
            result,
            "https://acct.blob.core.windows.net/c/f?sv=2021&sig=REDACTED"
        );
    }

    #[test]
    fn sanitize_sas_no_sig_unchanged() {
        let input = "no signature here".to_string();
        let result = sanitize_sas(input);
        assert_eq!(result, "no signature here");
    }

    #[test]
    fn sanitize_sas_redacts_multiple_occurrences() {
        let input = "url1?sig=secret1&se=2026 url2?sig=secret2&se=2027".to_string();
        let result = sanitize_sas(input);
        assert!(!result.contains("secret1"));
        assert!(!result.contains("secret2"));
        assert!(result.contains("sig=REDACTED"));
    }

    #[test]
    fn url_endpoint_with_scheme_is_used_directly() {
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("http://127.0.0.1:10000".to_string()),
            ..Default::default()
        });
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "http://127.0.0.1:10000/my-container/prefix/file.csv.gz"
        );
    }

    #[test]
    fn url_endpoint_with_https_scheme_is_used_directly() {
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("https://azurite.local:10000".to_string()),
            ..Default::default()
        });
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://azurite.local:10000/my-container/prefix/file.csv.gz"
        );
    }

    // ---------------------------------------------------------------
    // 5. URL with special characters (uses shared percent_encode_path)
    // ---------------------------------------------------------------

    #[test]
    fn url_encodes_special_chars_in_key() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_azure_url(&stage, "dir/my file (1).csv").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.windows.net/my-container/dir/my%20file%20%281%29.csv"
        );
    }

    // ---------------------------------------------------------------
    // 6. Upload status enum
    // ---------------------------------------------------------------

    #[test]
    fn upload_status_display() {
        assert_eq!(UploadStatus::Uploaded.to_string(), "UPLOADED");
        assert_eq!(UploadStatus::Skipped.to_string(), "SKIPPED");
    }

    // ---------------------------------------------------------------
    // 7. Pre-upload HEAD probe and skip-decision
    //
    // Contract: HEAD is issued only when at least one skip branch could
    // fire (`!overwrite || skip_upload_on_content_match`), and the skip
    // is keyed on either remote existence or remote-vs-local digest
    // equality. Six tests cover every row of the truth table.
    // ---------------------------------------------------------------

    use wiremock::matchers::{header, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Builds a `StageInfo` whose `endpoint` is the mock server URI, so
    /// `build_azure_url` routes the SAS-signed URL straight at the mock.
    fn mock_stage(mock_uri: &str) -> StageInfo {
        StageInfo {
            location_type: super::super::types::LocationType::Azure,
            bucket: "test-container".to_string(),
            key_prefix: "prefix/".to_string(),
            region: "eastus2".to_string(),
            creds: CloudCredentials::Azure {
                sas_token: SensitiveString::from("sv=2021-08-06&sig=test-secret-sig&se=2099-01-01"),
            },
            endpoint: Some(mock_uri.to_string()),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
            storage_account: Some("test".to_string()),
        }
    }

    // ---- Refresh-on-403: predicate, marker, and HEAD-recovery coverage ----
    // (Restored from PR #248; the GET-redrive proof lands with the per-range
    // work.)

    fn azure_creds(sig: &str) -> CloudCredentials {
        CloudCredentials::Azure {
            sas_token: SensitiveString::from(format!("sv=2021&sig={sig}&se=2099")),
        }
    }

    fn identity_refresh_map(
        e: crate::file_manager::types::StageInfoRefreshError,
    ) -> AzureUploadError {
        AzureUploadError::StageInfoRefresh {
            source: Box::new(e),
            location: Location::default(),
        }
    }

    #[test]
    fn is_expired_sas_error_fires_on_any_403() {
        assert!(is_expired_sas_error(403));
    }

    #[test]
    fn is_expired_sas_error_ignores_non_403() {
        assert!(!is_expired_sas_error(200));
        assert!(!is_expired_sas_error(404));
        assert!(!is_expired_sas_error(500));
        assert!(!is_expired_sas_error(401));
    }

    #[tokio::test]
    async fn azure_sas_refresher_refresh_returns_false_when_cache_marker_unchanged() {
        // No arm_rotation: refresh() bumps the call counter but stores nothing,
        // so the cache marker does not advance; AzureSasRefresher must report
        // the coalesced no-op as Ok(false).
        let mut fake = FakeStageInfoRefresher::new(azure_creds("SIG1"));
        let initial_marker = fake.cache().cached_at();
        let mut sas_refresher =
            AzureSasRefresher::new_with_marker(&mut fake, initial_marker, identity_refresh_map);

        let result = crate::refresh::Refresher::refresh(&mut sas_refresher).await;

        assert!(
            result.is_ok(),
            "unchanged marker must return Ok; got: {result:?}"
        );
        assert!(
            !result.unwrap(),
            "Ok(false): marker unchanged means no new snapshot"
        );
        assert_eq!(
            fake.refresh_call_count(),
            1,
            "fake called exactly once even though nothing was stored"
        );
    }

    #[tokio::test]
    async fn azure_sas_refresher_refresh_returns_true_when_cache_marker_advances() {
        let mut fake = FakeStageInfoRefresher::new(azure_creds("SIG1"));
        fake.arm_rotation(azure_creds("SIG2"));
        let initial_marker = fake.cache().cached_at();
        let mut sas_refresher =
            AzureSasRefresher::new_with_marker(&mut fake, initial_marker, identity_refresh_map);

        let result = crate::refresh::Refresher::refresh(&mut sas_refresher).await;

        assert!(
            result.is_ok(),
            "rotated marker must return Ok; got: {result:?}"
        );
        assert!(
            result.unwrap(),
            "Ok(true): marker advanced means new snapshot landed"
        );
    }

    /// Regression for the HEAD-bypass bug: a default PUT (!overwrite) whose
    /// pre-upload HEAD probe hits an expired-SAS 403 must REFRESH and retry,
    /// not fail closed before the PUT. Fails on the pre-fix code (HEAD ran
    /// outside the refresh closure → terminal, no refresh); passes after.
    #[tokio::test(flavor = "multi_thread")]
    async fn head_403_with_refresher_refreshes_then_uploads_when_not_overwrite() {
        use wiremock::Request;
        const ORIGINAL_SIG: &str = "sig=ORIGINAL-EXPIRED";
        const REFRESHED_SIG: &str = "sig=REFRESHED-FRESH";
        let original_sas = format!("sv=2021-08-06&{ORIGINAL_SIG}&se=2099-01-01");
        let refreshed_sas = format!("sv=2021-08-06&{REFRESHED_SIG}&se=2099-01-01");

        let server = MockServer::start().await;
        // HEAD: 403 with the expired SAS; 404 (blob absent) once refreshed.
        Mock::given(method("HEAD"))
            .respond_with(move |req: &Request| {
                if req.url.as_str().contains(REFRESHED_SIG) {
                    ResponseTemplate::new(404)
                } else {
                    ResponseTemplate::new(403)
                        .set_body_string("Server failed to authenticate the request.")
                }
            })
            .mount(&server)
            .await;
        // PUT succeeds only with the refreshed SAS.
        Mock::given(method("PUT"))
            .respond_with(move |req: &Request| {
                if req.url.as_str().contains(REFRESHED_SIG) {
                    ResponseTemplate::new(201)
                } else {
                    ResponseTemplate::new(403)
                }
            })
            .mount(&server)
            .await;

        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(server.uri()),
            creds: Some(CloudCredentials::Azure {
                sas_token: SensitiveString::from(original_sas.clone()),
            }),
            ..Default::default()
        });

        let mut fake = FakeStageInfoRefresher::new(stage.creds.clone());
        fake.arm_rotation(CloudCredentials::Azure {
            sas_token: SensitiveString::from(refreshed_sas.clone()),
        });
        let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);

        let status = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut refresher_opt,
        )
        .await
        .expect("default PUT must refresh the expired SAS on the HEAD 403 and upload");
        assert_eq!(status, UploadStatus::Uploaded);
        assert_eq!(
            fake.refresh_call_count(),
            1,
            "exactly one refresh for the single HEAD-403 recovery"
        );

        let requests = server.received_requests().await.unwrap_or_default();
        assert!(
            requests
                .iter()
                .any(|r| r.method.as_str() == "PUT" && r.url.as_str().contains(REFRESHED_SIG)),
            "the PUT after refresh must carry the refreshed SAS"
        );
    }

    /// §13 M1/M2 (Gherkin S7): a refresh-MECHANISM failure logs once at `error!`,
    /// naming the failure reason. S4 (GET) shares this contract on the download path.
    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn put_refresh_mechanism_failure_logs_at_error_naming_reason() {
        let server = MockServer::start().await;
        // Every PUT 403s → triggers a refresh; the refresh itself is armed to fail.
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(403).set_body_string("expired"))
            .mount(&server)
            .await;
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(server.uri()),
            ..Default::default()
        });
        let mut fake = FakeStageInfoRefresher::new(stage.creds.clone());
        fake.arm_failure("GS re-issue rejected in test");
        let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);

        let result = upload_to_azure_or_skip(
            prepared_upload_with_digest("d"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut refresher_opt,
        )
        .await;

        assert!(
            result.is_err(),
            "a refresh-mechanism failure must be terminal"
        );
        assert!(
            logs_contain("Azure SAS refresh failed; upload aborted"),
            "refresh-mechanism failure must log at error! (upload aborted)"
        );
        assert!(
            logs_contain("GS re-issue rejected in test"),
            "the error! log must name the refresh-failure reason"
        );
    }

    /// §13 M1/M2 (Gherkin S5/S8): a 403 that survives the refresh (the fresh SAS
    /// is still rejected) logs once at `warn!` carrying the status and a
    /// SAS-redacted URL (no sig value leaks).
    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn put_terminal_403_after_refresh_logs_at_warn_with_redacted_url() {
        const REFRESHED_SIG: &str = "sig=REFRESHED-STILL-BAD";
        let refreshed_sas = format!("sv=2021&{REFRESHED_SIG}&se=2099");
        let server = MockServer::start().await;
        // Every PUT 403s, even with the refreshed SAS: refresh rotates once, the
        // retry still 403s, and the terminal 403 surfaces after the refresh.
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(403).set_body_string("still denied"))
            .mount(&server)
            .await;
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(server.uri()),
            creds: Some(azure_creds("ORIGINAL-EXPIRED")),
            ..Default::default()
        });
        let mut fake = FakeStageInfoRefresher::new(stage.creds.clone());
        fake.arm_rotation(CloudCredentials::Azure {
            sas_token: SensitiveString::from(refreshed_sas.clone()),
        });
        let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);

        let result = upload_to_azure_or_skip(
            prepared_upload_with_digest("d"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut refresher_opt,
        )
        .await;

        assert!(
            matches!(
                result,
                Err(AzureUploadError::AzureHttp {
                    status_code: 403,
                    ..
                })
            ),
            "a 403 surviving the refresh must be terminal; got {result:?}"
        );
        assert!(
            logs_contain("failed terminally with 403 after SAS refresh"),
            "a terminal 403 must log at warn!"
        );
        assert!(
            !logs_contain("REFRESHED-STILL-BAD") && !logs_contain("ORIGINAL-EXPIRED"),
            "the warn! log's URL must be SAS-redacted (no sig value)"
        );
    }

    /// Symmetric-GET regression: a 403 on the streaming download's routing HEAD
    /// (expired SAS) must refresh and re-drive, not fail closed. Fails on the
    /// pre-fix code (routing HEAD ran outside the refresh layer). The ranged-GET
    /// path shares this closure + `azure_range_attempt`'s 403 mapping; a wire
    /// test for the ranged branch (Range-request mocking) is a follow-up.
    #[tokio::test(flavor = "multi_thread")]
    async fn streaming_get_routing_head_403_with_refresher_recovers() {
        use wiremock::Request;
        const ORIGINAL_SIG: &str = "sig=ORIGINAL-EXPIRED";
        const REFRESHED_SIG: &str = "sig=REFRESHED-FRESH";
        const BLOB_BODY: &[u8] = b"WHOLE-BLOB-BODY";
        let original_sas = format!("sv=2021-08-06&{ORIGINAL_SIG}&se=2099-01-01");
        let refreshed_sas = format!("sv=2021-08-06&{REFRESHED_SIG}&se=2099-01-01");

        let server = MockServer::start().await;
        // Routing HEAD: 403 with the expired SAS; 200 once refreshed.
        Mock::given(method("HEAD"))
            .respond_with(move |req: &Request| {
                if req.url.as_str().contains(REFRESHED_SIG) {
                    ResponseTemplate::new(200).insert_header(AZURE_META_SFC_DIGEST, "test-digest")
                } else {
                    ResponseTemplate::new(403)
                        .set_body_string("Server failed to authenticate the request.")
                }
            })
            .mount(&server)
            .await;
        // Single GET (content-length below threshold): 200 body once refreshed.
        Mock::given(method("GET"))
            .respond_with(move |req: &Request| {
                if req.url.as_str().contains(REFRESHED_SIG) {
                    ResponseTemplate::new(200).set_body_bytes(BLOB_BODY.to_vec())
                } else {
                    ResponseTemplate::new(403)
                }
            })
            .mount(&server)
            .await;

        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(server.uri()),
            creds: Some(CloudCredentials::Azure {
                sas_token: SensitiveString::from(original_sas.clone()),
            }),
            ..Default::default()
        });
        let mut fake = FakeStageInfoRefresher::new(stage.creds.clone());
        fake.arm_rotation(CloudCredentials::Azure {
            sas_token: SensitiveString::from(refreshed_sas.clone()),
        });
        let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);

        let dl = download_from_azure_streaming(
            &stage,
            "file.csv",
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            false,
            cloud_http::CloudSpillTarget::Temp(std::env::temp_dir().as_path()),
            &mut refresher_opt,
        )
        .await
        .expect("streaming GET must refresh on the routing-HEAD 403 and succeed");

        // `into_reader()` on a `Streamed` body drains a `cloud_http::StreamReader`,
        // whose `Read` impl calls `blocking_recv` — only valid off the tokio
        // runtime (see `StreamReader`'s doc comment), so the read happens on a
        // blocking-pool thread here, mirroring how the production download path
        // (`write_cloud_download`) always drives it inside `spawn_blocking`.
        let body = tokio::task::spawn_blocking(move || -> Vec<u8> {
            let mut body = Vec::new();
            std::io::Read::read_to_end(&mut dl.body.into_reader().expect("into_reader"), &mut body)
                .expect("read body after re-drive");
            body
        })
        .await
        .expect("blocking read task must not panic");
        assert_eq!(
            body, BLOB_BODY,
            "body after re-drive must match served bytes"
        );
        assert_eq!(
            fake.refresh_call_count(),
            1,
            "exactly one refresh for the single routing-HEAD 403"
        );
        let reqs = server.received_requests().await.unwrap_or_default();
        assert!(
            reqs.iter()
                .any(|r| r.method.as_str() == "HEAD" && r.url.as_str().contains(REFRESHED_SIG)),
            "the re-driven HEAD must carry the refreshed SAS"
        );
    }

    /// GET with no refresher: a routing-HEAD 403 terminates as `AzureHttp` 403
    /// (no refresh, no inline 403 retry, no hang) — symmetric with the PUT
    /// no-refresher test.
    #[tokio::test(flavor = "multi_thread")]
    async fn streaming_get_403_no_refresher_terminates() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(403)
                    .set_body_string("Server failed to authenticate the request."),
            )
            .mount(&server)
            .await;

        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(server.uri()),
            ..Default::default()
        });
        let result = download_from_azure_streaming(
            &stage,
            "file.csv",
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            false,
            cloud_http::CloudSpillTarget::Temp(std::env::temp_dir().as_path()),
            &mut None,
        )
        .await;
        // `CloudStreamingDownload` is not Debug; assert on the extracted error.
        let err = result.err();
        assert!(
            matches!(
                &err,
                Some(AzureDownloadError::AzureHttp {
                    status_code: 403,
                    ..
                })
            ),
            "routing-HEAD 403 + no refresher must terminate as AzureHttp 403; got: {err:?}"
        );
    }

    /// Scenario 1: existence-only branch — `!overwrite && exists` returns
    /// `Skipped` without issuing a PUT. Mirrors UD's pre-gap behaviour and
    /// guards against regression in the `send_head_to_azure_blob` refactor.
    #[tokio::test(flavor = "multi_thread")]
    async fn skip_when_overwrite_false_and_blob_exists() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("upload-or-skip should succeed");
        assert_eq!(status, UploadStatus::Skipped);
    }

    /// Scenario 2: HEAD elision — `overwrite=true && skip_match=false`
    /// proves UD doesn't waste a round-trip on the path Python wastes on.
    /// `Mock::given(method("HEAD")).expect(0)` is the load-bearing
    /// assertion: any HEAD against the mock fails the test.
    #[tokio::test(flavor = "multi_thread")]
    async fn no_head_issued_when_overwrite_true_and_skip_match_false() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .and(header("x-ms-blob-type", "BlockBlob"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("upload should succeed against the mock");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    /// Scenario 3: content-match branch — when the remote `sfcdigest`
    /// equals the local digest, the upload is skipped. Uses the *real*
    /// `compute_sha256_digest` output rather than a synthetic value so
    /// that a future change to the digest format on either side fails
    /// here, not silently in production.
    #[tokio::test(flavor = "multi_thread")]
    async fn skip_when_overwrite_true_and_skip_match_true_and_digests_match() {
        use super::super::encryption::compute_sha256_digest;

        let source = ByteSource::Bytes(b"hello-azure".to_vec().into());
        let real_digest = compute_sha256_digest(&source).expect("digest computation");

        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header(AZURE_META_SFC_DIGEST, real_digest.as_str()),
            )
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            PreparedUpload {
                source: source.into(),
                digest: real_digest,
                cse: None,
            },
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("upload-or-skip should succeed");
        assert_eq!(status, UploadStatus::Skipped);
    }

    /// Scenario 4: content-mismatch — same flags as scenario 3, but the
    /// remote `sfcdigest` differs from the local one. Different content
    /// cannot be skipped over; upload must proceed.
    #[tokio::test(flavor = "multi_thread")]
    async fn upload_when_overwrite_true_and_skip_match_true_and_digests_differ() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(200).insert_header(AZURE_META_SFC_DIGEST, "remote-digest"),
            )
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("upload should succeed against the mock");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    /// Scenario 5: 404 on HEAD — blob doesn't exist. Even with the flag
    /// on, there is no remote header to compare, so the upload runs.
    #[tokio::test(flavor = "multi_thread")]
    async fn upload_when_skip_match_true_and_head_404() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("upload should succeed against the mock");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    /// Scenario 6: HEAD returns 200 but the `sfcdigest` user-metadata
    /// header is absent — e.g. the blob was uploaded by a tool that
    /// doesn't write Snowflake's custom header. Cannot compare digests,
    /// so the content-match branch must NOT skip.
    #[tokio::test(flavor = "multi_thread")]
    async fn upload_when_skip_match_true_and_head_200_without_sfcdigest_header() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("upload should succeed against the mock");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    // ---------------------------------------------------------------
    // 8. Skip-decision isolation tests live in `file_manager::tests`
    //    (`classify_*`), since the decision is shared across clouds.
    // ---------------------------------------------------------------

    // ---------------------------------------------------------------
    // 9. Parametrized fail-open over HEAD error classes — only 404 is
    //    covered by the existing scenarios. A regression that
    //    misclassifies any of {403, 5xx, transport, malformed-header}
    //    as a successful match would silently preserve stale stage
    //    content (data-correctness, not perf).
    // ---------------------------------------------------------------

    /// Helper: assert that overwrite=true + skip_match=true against the
    /// configured HEAD response results in an `Uploaded` status (PUT runs
    /// exactly once). The matching local digest in the request would skip
    /// IF the HEAD parser misread the error class as a successful 200 with
    /// a matching digest.
    async fn assert_failopen_uploads(head_response: ResponseTemplate) {
        use super::super::encryption::compute_sha256_digest;
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(head_response)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock)
            .await;

        let source = ByteSource::Bytes(b"hello-azure".to_vec().into());
        let real_digest = compute_sha256_digest(&source).expect("digest");
        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            PreparedUpload {
                source: source.into(),
                digest: real_digest,
                cse: None,
            },
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("upload should succeed against the mock");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failopen_403_uploads() {
        assert_failopen_uploads(ResponseTemplate::new(403)).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failopen_500_uploads() {
        assert_failopen_uploads(ResponseTemplate::new(500)).await;
    }

    /// Malformed `x-ms-meta-sfcdigest` — non-ASCII bytes make `to_str()`
    /// fail in `send_head_to_azure_blob`; the digest is dropped to `None`
    /// and the comparison can't match. Must fall through to upload.
    #[tokio::test(flavor = "multi_thread")]
    async fn failopen_malformed_sfcdigest_header_uploads() {
        // Non-ASCII bytes (0xFF) — invalid as an HTTP header value's str view.
        let head =
            ResponseTemplate::new(200).insert_header(AZURE_META_SFC_DIGEST, "\u{00ff}invalid-utf8");
        assert_failopen_uploads(head).await;
    }

    /// Transport error: connect to a server URI that no mock is bound to.
    /// `reqwest`'s `send().await` returns `Err`, mapping to `None` in
    /// `send_head_to_azure_blob` (the documented fail-open path).
    #[tokio::test(flavor = "multi_thread")]
    async fn failopen_transport_error_uploads() {
        use super::super::encryption::compute_sha256_digest;
        // Pick a port unlikely to be bound. We never start a server here.
        let stage = mock_stage("http://127.0.0.1:1");
        let source = ByteSource::Bytes(b"hello-azure".to_vec().into());
        let real_digest = compute_sha256_digest(&source).expect("digest");

        // Note: `azure_request_with_retry` for the PUT will also fail since
        // the same address is unreachable. The point of this test is to
        // exercise the HEAD failure path, not assert PUT success — so we
        // expect an error, but the failure mode must be "PUT was attempted",
        // not "skip fired silently". The result type is the proxy: an Ok
        // here would mean skip fired (data loss); an Err means we tried to
        // PUT and the network failed (correct fail-open behaviour).
        let result = upload_to_azure_or_skip(
            PreparedUpload {
                source: source.into(),
                digest: real_digest,
                cse: None,
            },
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await;
        assert!(
            result.is_err(),
            "transport-error fail-open must reach PUT (which then errors), not silently skip; got: {result:?}"
        );
    }
    // ---------------------------------------------------------------
    // 10. Azure PUT omits Content-Encoding-class headers
    // ---------------------------------------------------------------
    //
    // Asserts the wire-level outcome directly: neither `Content-Encoding`
    // nor `x-ms-blob-content-encoding` reaches Azure on a single-shot PUT.
    // Catches regressions where a reqwest default, middleware, or a future
    // `default_headers(...)` configuration silently re-introduces one of
    // these headers.

    #[tokio::test]
    async fn azure_put_omits_content_encoding_headers() {
        let mock = MockServer::start().await;

        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&mock)
            .await;

        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(mock.uri()),
            ..Default::default()
        });

        let prepared = PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(Bytes::from_static(
                b"hello world",
            )),
            digest: "0".repeat(64),
            cse: None,
        };

        // overwrite=true skips the existence-check HEAD probe so the
        // first request the mock sees is the PUT we want to inspect.
        upload_to_azure_or_skip(
            prepared,
            &stage,
            "file.dat",
            true,
            false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("upload should succeed against the mock");

        let received = mock
            .received_requests()
            .await
            .expect("mock should have captured requests");
        let put = received
            .iter()
            .find(|r| r.method.as_str() == "PUT")
            .expect("a PUT request should have been received");

        // Positive presence checks: required headers must still be sent.
        // Without these, a regression that silently strips ALL headers
        // would also pass the absent-checks below.
        assert!(
            put.headers.get("x-ms-blob-type").is_some(),
            "x-ms-blob-type must be present on Azure PUT"
        );
        assert!(
            put.headers.get(AZURE_META_SFC_DIGEST).is_some(),
            "{AZURE_META_SFC_DIGEST} must be present on Azure PUT"
        );

        // Absence checks: neither Content-Encoding nor its blob-metadata
        // variant may appear. `http::HeaderMap::get` is case-insensitive —
        // one check covers both `content-encoding` and `Content-Encoding`.
        assert!(
            put.headers.get("content-encoding").is_none(),
            "Content-Encoding must be absent on Azure PUT (got {:?})",
            put.headers.get("content-encoding")
        );
        assert!(
            put.headers.get("x-ms-blob-content-encoding").is_none(),
            "x-ms-blob-content-encoding must be absent on Azure PUT (got {:?})",
            put.headers.get("x-ms-blob-content-encoding")
        );
    }

    // ---------------------------------------------------------------
    // 11. HEAD fail-CLOSED + retry
    //
    // HEAD probe runs through `azure_request_with_retry`, retrying
    // transient 5xx / transport / 403 up to `max_attempts`. After
    // exhaustion (or on a non-retryable, non-404 status), the probe
    // surfaces `Err`; `upload_to_azure_or_skip` then dispatches:
    //   - `!overwrite`            => fail-CLOSED (refuse to clobber).
    //   - skip_match (overwrite)  => fail-OPEN (waste a PUT, not data).
    // ---------------------------------------------------------------

    /// Transient 5xx on first HEAD, 200 on second. Existence skip fires.
    #[tokio::test(flavor = "multi_thread")]
    async fn head_retries_on_transient_5xx_then_succeeds() {
        let mock = MockServer::start().await;
        // First HEAD: 503 (matches once, then exhausts via up_to_n_times).
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&mock)
            .await;
        // Subsequent HEADs (after the priority-1 mock exhausts): 200.
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .with_priority(2)
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("retry should recover and existence skip should fire");
        assert_eq!(status, UploadStatus::Skipped);
    }

    /// 403 + !overwrite, no refresher: the HEAD probe fast-fails the expired
    /// SAS to the refresh layer (no inline 403 retry — that is the bug this
    /// change fixes). With no refresher to rotate against, it surfaces
    /// terminally as `AzureHttp` 403 (fail-CLOSED). HEAD runs once; PUT never.
    #[tokio::test(flavor = "multi_thread")]
    async fn head_403_without_refresher_fails_closed_when_not_overwrite() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(403))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let result = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await;
        assert!(
            matches!(
                &result,
                Err(AzureUploadError::AzureHttp {
                    status_code: 403,
                    ..
                })
            ),
            "403 + !overwrite + no refresher must fail-CLOSED terminally as AzureHttp 403 (fast-fail, no inline 403 retry, no PUT); got: {result:?}"
        );
    }

    /// 403 + skip_match (overwrite=true), no refresher: the HEAD probe
    /// fast-fails once (no inline 403 retry) and falls through to PUT
    /// (fail-OPEN — a missed skip is just bandwidth, and the PUT refreshes on
    /// its own 403 if the SAS is expired). HEAD runs once; PUT runs and succeeds.
    #[tokio::test(flavor = "multi_thread")]
    async fn head_403_fails_open_to_put_when_skip_match() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(403))
            .expect(1)
            .mount(&mock)
            .await;
        // The skip_match path falls through to PUT on a HEAD failure (fail-OPEN),
        // rather than surfacing the HEAD error (fail-CLOSED).
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("skip_match 403 HEAD should fail-OPEN to PUT");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    /// Persistent 5xx + !overwrite: same fail-CLOSED outcome as the 403
    /// case, but via the *default* retryable set (no `extra_retryable_statuses`
    /// dependency).
    #[tokio::test(flavor = "multi_thread")]
    async fn head_fails_closed_on_persistent_5xx_when_not_overwrite() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(503))
            .expect(DEFAULT_PUT_GET_MAX_ATTEMPTS as u64)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let result = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await;
        assert!(
            matches!(
                &result,
                Err(AzureUploadError::RetryExhausted { detail, .. }) if detail.contains("503")
            ),
            "persistent 5xx + !overwrite must fail-CLOSED with RetryExhausted carrying \"503\" in detail after retry exhaustion; got: {result:?}"
        );
    }

    /// Non-retryable non-404 (401) + !overwrite: probe returns `Err`
    /// immediately (no retry). HEAD-count = 1 pins the no-retry rule;
    /// PUT `expect(0)` pins fail-CLOSED.
    #[tokio::test(flavor = "multi_thread")]
    async fn head_fails_closed_immediately_on_non_retryable_4xx_when_not_overwrite() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let result = upload_to_azure_or_skip(
            prepared_upload_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            MultipartParams::default(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await;
        assert!(
            matches!(
                &result,
                Err(AzureUploadError::AzureHttp {
                    status_code: 401,
                    ..
                })
            ),
            "non-retryable non-404 status + !overwrite must fail-CLOSED with AzureHttp{{401}} on first attempt; got: {result:?}"
        );
    }

    // ---------------------------------------------------------------
    // Block-blob multipart upload + ranged download (wiremock)
    // ---------------------------------------------------------------

    use wiremock::Request;
    use wiremock::matchers::query_param;

    /// `MultipartParams` with a 1-byte threshold so any non-empty body takes
    /// the block-blob path.
    fn always_multipart() -> MultipartParams {
        MultipartParams {
            threshold: super::super::multipart::MultipartThreshold::from_server(Some(1)),
            concurrency: 4,
        }
    }

    /// A 9 MiB SSE body splits into three Azure blocks (4 + 4 + 1 MiB) at the
    /// 4 MiB default block size: `Put Block` ×3 then one `Put Block List`.
    #[tokio::test(flavor = "multi_thread")]
    async fn azure_block_blob_upload_stages_blocks_then_commits() {
        let mock = MockServer::start().await;

        // Put Block: PUT ?comp=block&blockid=...
        Mock::given(method("PUT"))
            .and(query_param("comp", "block"))
            .respond_with(ResponseTemplate::new(201))
            .expect(3)
            .mount(&mock)
            .await;

        // Put Block List: PUT ?comp=blocklist — must carry the digest metadata.
        Mock::given(method("PUT"))
            .and(query_param("comp", "blocklist"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock)
            .await;

        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(mock.uri()),
            ..Default::default()
        });

        let prepared = PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(Bytes::from(vec![
                3u8;
                9 << 20
            ])),
            digest: "0".repeat(64),
            cse: None,
        };

        upload_to_azure_or_skip(
            prepared,
            &stage,
            "file.dat",
            true,
            false,
            always_multipart(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("block-blob upload should succeed against the mock");

        let received = mock.received_requests().await.unwrap();
        let commit = received
            .iter()
            .find(|r| {
                r.url
                    .query_pairs()
                    .any(|(k, v)| k == "comp" && v == "blocklist")
            })
            .expect("a Put Block List commit must be sent");
        assert!(
            commit.headers.get(AZURE_META_SFC_DIGEST).is_some(),
            "digest metadata must ride on the block-list commit"
        );
    }

    /// A blob above the threshold is fetched with a ranged GET into a tempfile
    /// and re-read byte-for-byte through the returned reader.
    #[tokio::test(flavor = "multi_thread")]
    async fn azure_ranged_download_reassembles_blob() {
        use std::io::Read as _;

        let payload = b"hello ranged azure blob world".to_vec();
        let mock = MockServer::start().await;

        // HEAD (Get Blob Properties) reports the size via Content-Length; the
        // body bytes drive that header (production reads the header, not the
        // body, on HEAD).
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; payload.len()]))
            .mount(&mock)
            .await;

        // The single range returns the whole payload.
        let body = payload.clone();
        Mock::given(method("GET"))
            .respond_with(move |_: &Request| {
                ResponseTemplate::new(206).set_body_bytes(body.clone())
            })
            .mount(&mock)
            .await;

        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(mock.uri()),
            ..Default::default()
        });

        let spill = tempfile::tempdir().unwrap();
        let dl = download_from_azure_streaming(
            &stage,
            "file.dat",
            always_multipart(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            false,
            cloud_http::CloudSpillTarget::Temp(spill.path()),
            &mut None,
        )
        .await
        .expect("ranged download should succeed against the mock");

        assert_eq!(dl.cloud_byte_count, payload.len() as i64);
        let mut got = Vec::new();
        dl.body
            .into_reader()
            .unwrap()
            .read_to_end(&mut got)
            .unwrap();
        assert_eq!(got, payload, "reassembled blob must match the object");
    }

    /// A non-encrypted ranged Azure download assembles straight into the caller's
    /// `.part` file, which the caller renames to the destination on success.
    #[tokio::test(flavor = "multi_thread")]
    async fn azure_ranged_download_assembles_into_part_file() {
        let payload = b"azure ranged straight into dot part".to_vec();
        let mock = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; payload.len()]))
            .mount(&mock)
            .await;
        let body = payload.clone();
        Mock::given(method("GET"))
            .respond_with(move |_: &Request| {
                ResponseTemplate::new(206).set_body_bytes(body.clone())
            })
            .mount(&mock)
            .await;

        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(mock.uri()),
            ..Default::default()
        });
        let dir = tempfile::tempdir().unwrap();
        let part_path = dir.path().join("out.dat.part");
        let dl = download_from_azure_streaming(
            &stage,
            "file.dat",
            always_multipart(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            false,
            cloud_http::CloudSpillTarget::Part(&part_path),
            &mut None,
        )
        .await
        .expect("ranged download should succeed against the mock");

        match dl.body {
            cloud_http::CloudDownloadBody::Spilled(cloud_http::CloudSpilledBody::Part(p)) => {
                assert_eq!(p, part_path, "the assembly file must be the caller's .part");
                assert_eq!(
                    std::fs::read(&p).unwrap(),
                    payload,
                    "the .part must hold the whole reassembled object"
                );
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = std::fs::metadata(&part_path).unwrap().permissions().mode();
                    assert_eq!(mode & 0o777, 0o600, "ranged .part must be owner-only");
                }
            }
            _ => panic!("a non-encrypted ranged download must assemble into `.part`"),
        }
    }

    /// A failed ranged Azure download drains its in-flight writes and removes the
    /// `.part`, so a failure never leaves a partial file behind.
    #[tokio::test(flavor = "multi_thread")]
    async fn azure_ranged_download_failure_removes_part_file() {
        let mock = MockServer::start().await;

        // HEAD advertises 32 bytes, but every ranged GET returns a 4-byte body,
        // tripping the range-length guard and failing the download.
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; 32]))
            .mount(&mock)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(vec![0u8; 4]))
            .mount(&mock)
            .await;

        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some(mock.uri()),
            ..Default::default()
        });
        let dir = tempfile::tempdir().unwrap();
        let part_path = dir.path().join("out.dat.part");
        let result = download_from_azure_streaming(
            &stage,
            "file.dat",
            always_multipart(),
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            false,
            cloud_http::CloudSpillTarget::Part(&part_path),
            &mut None,
        )
        .await;

        assert!(result.is_err(), "a short ranged GET must fail the download");
        assert!(
            !part_path.exists(),
            "a failed ranged download must not leave a `.part` file behind"
        );
    }
}
