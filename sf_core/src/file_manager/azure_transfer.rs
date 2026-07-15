use super::cloud_http::{self, CloudStreamingDownload, CseDownloadInfo, UploadRetryAdapter};
use super::types::{
    ByteSource, CloudCredentials, DownloadResponse, EncryptedFileMetadata, EncryptionData,
    MaterialDescription, PreparedUpload, StageInfo, UploadStatus, build_encryption_metadata_json,
    percent_encode_path,
};
use crate::config::retry::RetryPolicy;
use crate::http::retry::{HttpContext, HttpError, execute_with_retry as http_execute_with_retry};
use crate::sensitive::SensitiveString;
use reqwest::Method;
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::time::Duration;

const REQUEST_TIMEOUT_SECS: u64 = 300;

// Azure metadata header names
const AZURE_META_SFC_DIGEST: &str = "x-ms-meta-sfcdigest";
const AZURE_META_ENCRYPTIONDATA: &str = "x-ms-meta-encryptiondata";
const AZURE_META_MATDESC: &str = "x-ms-meta-matdesc";

/// Uploads a file to Azure, skipping when a HEAD probe says either
/// "blob exists and overwrite is off" or "blob content matches the local
/// digest and the caller opted into content-match skipping". HEAD is elided
/// when neither skip branch can fire, saving a round-trip vs. Python.
pub async fn upload_to_azure_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
    skip_upload_on_content_match: bool,
    policy: &RetryPolicy,
) -> Result<UploadStatus, AzureUploadError> {
    let client = create_azure_client(stage_info)?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let (url, sas_token) = resolve_url_and_token(stage_info, &key)?;

    let head_needed = !overwrite || skip_upload_on_content_match;
    let remote = if head_needed {
        match send_head_to_azure_blob(&client, &url, sas_token, policy).await {
            Ok(remote) => remote,
            Err(e) if !overwrite => {
                // Cannot verify whether the blob exists; fail-CLOSED to
                // avoid silently clobbering existing stage content.
                return Err(e);
            }
            Err(_) => {
                // skip_match path: a missed skip is bandwidth waste, not
                // data loss — fail-OPEN and let the PUT proceed.
                None
            }
        }
    } else {
        None
    };

    match classify_pre_upload_skip(
        overwrite,
        skip_upload_on_content_match,
        remote.as_ref(),
        &prepared.digest,
    ) {
        SkipDecision::Existence => {
            tracing::info!("Blob already exists in Azure: {}", key);
            return Ok(UploadStatus::Skipped);
        }
        SkipDecision::ContentMatch => {
            tracing::info!(
                "Blob content matches local digest, skipping upload: {}",
                key
            );
            return Ok(UploadStatus::Skipped);
        }
        SkipDecision::Upload => {}
    }

    upload_to_azure(&client, &url, sas_token, prepared, policy).await?;
    Ok(UploadStatus::Uploaded)
}

/// Outcome of the pre-upload skip check. Extracted so the decision is
/// testable independent of the HEAD elision in `upload_to_azure_or_skip`
/// (the elision can hide a missing guard in the content-match branch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkipDecision {
    /// The blob is visible on stage and the caller didn't request overwrite.
    /// Skip without comparing content — stale stage bytes are preserved.
    Existence,
    /// The caller opted into content-match skipping and the remote digest
    /// equals the local one. Skip the redundant upload; the bytes on stage
    /// are already what we'd have written.
    ContentMatch,
    /// Neither skip applies; upload proceeds.
    Upload,
}

/// Pure decision: which skip branch (if any) fires. Existence-only is checked
/// first so a `!overwrite` caller never reaches the content-match branch — a
/// blob that exists is treated as authoritative regardless of digest. A
/// missing remote digest cannot match, so the content branch falls through
/// to `Upload` in that case.
fn classify_pre_upload_skip(
    overwrite: bool,
    skip_upload_on_content_match: bool,
    remote: Option<&RemoteBlobHeader>,
    local_digest: &str,
) -> SkipDecision {
    if !overwrite && remote.is_some() {
        return SkipDecision::Existence;
    }
    if overwrite
        && skip_upload_on_content_match
        && remote.and_then(|h| h.digest.as_deref()) == Some(local_digest)
    {
        return SkipDecision::ContentMatch;
    }
    SkipDecision::Upload
}

/// Downloads a file from Azure Blob Storage and returns data with optional encryption metadata.
/// For SSE stages the metadata headers will be absent and `None` is returned.
///
/// `cloud_byte_count` reflects the on-cloud (pre-decryption) byte count of
/// the blob — taken from the collected body length, which equals the
/// Azure `Content-Length` (i.e. the stored blob size) for non-streamed
/// responses. This is the wire byte count, not the decrypted/decoded
/// size of the original file.
pub async fn download_from_azure(
    stage_info: &StageInfo,
    filename: &str,
    policy: &RetryPolicy,
) -> Result<DownloadResponse, AzureDownloadError> {
    let client = create_azure_client(stage_info)?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let (url, sas_token) = resolve_url_and_token(stage_info, &key)?;
    let response = azure_request_with_retry(
        || client.get(build_sas_url(&url, sas_token.reveal())),
        Method::GET,
        policy,
    )
    .await?;

    // Extract metadata from response headers
    let headers = response.headers();
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

#[cfg(test)]
impl RemoteBlobHeader {
    fn with_digest(digest: &str) -> Self {
        Self {
            digest: Some(digest.to_string()),
        }
    }
}

/// Probes the blob with HEAD, retrying transient failures via the shared
/// Azure retry helper. Returns:
///
/// - `Ok(Some(header))` on 200 — blob exists, digest header captured.
/// - `Ok(None)` on 404 — blob does not exist, safe to upload.
/// - `Err(_)` on any other outcome after retry exhaustion (persistent
///   5xx / transport / 403, or non-retryable non-404 like 401 / 409).
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

/// Upload data to Azure with retry logic.
///
/// Streams the body without buffering the whole file in memory:
/// - `ByteSource::Path` opens the file on each retry attempt via
///   `tokio::fs::File` and wraps it in a streaming `reqwest::Body` — the
///   file content is never fully resident in memory at the same time.
/// - `ByteSource::Bytes` (the usual case after client-side encryption) uses
///   the already-in-memory ciphertext directly. It is an `Arc`-backed
///   `bytes::Bytes`, so the per-retry clone in `body_for` is an O(1)
///   reference-count bump — no copy of the ciphertext.
///
/// Sets encryption metadata headers only when client-side encryption was used.
///
/// The SAS token is taken as `&SensitiveString` and revealed only at the
/// URL-construction site so the raw secret never enters this function's
/// outer scope.
async fn upload_to_azure(
    client: &reqwest::Client,
    url: &str,
    sas_token: &SensitiveString,
    prepared: PreparedUpload,
    policy: &RetryPolicy,
) -> Result<(), AzureUploadError> {
    // `body_for` re-opens the source per retry (a `Path` re-open or an O(1)
    // `Bytes` refcount clone). `prepared` is held until this fn returns, so a
    // gzip-tempfile guard inside `prepared.source` outlives the upload + every
    // retry. The CSE params (cloud metadata + encryptor) are both present or
    // both absent — unbundle them once.
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
        .context(azure_upload_error::SerializationSnafu)?;

    let mat_desc_str = encryption_metadata
        .as_ref()
        .map(|enc_meta| serde_json::to_string(&enc_meta.material_desc))
        .transpose()
        .context(azure_upload_error::SerializationSnafu)?;

    // Content-Length must be set explicitly on every Azure upload: the body is
    // a streaming `reqwest::Body` (a wrapped CSE stream, or a `tokio::fs::File`
    // for an SSE `Path` source) whose length reqwest can't infer, so without
    // this header it would fall back to `Transfer-Encoding: chunked` — which
    // Azure rejects with `400 UnsupportedHeader`. CSE uses the analytic
    // ciphertext length; SSE uses the source length (file metadata / buffer len).
    let content_length = match &encryptor {
        Some(enc) => enc.cipher_len(),
        None => match &source {
            ByteSource::Bytes(b) => b.len() as i64,
            ByteSource::Path(p) => tokio::fs::metadata(p)
                .await
                .context(azure_upload_error::SourceIoSnafu)?
                .len() as i64,
        },
    };

    // Own everything the per-attempt async closure touches so the closure is
    // self-contained (`'static`): an `AsyncFn` whose returned future borrowed
    // these from this frame couldn't satisfy the `'static` bound the FFI/trait
    // futures require. `reqwest::Client` clone is a cheap `Arc` bump; the SAS
    // token stays a `SensitiveString` and is revealed only inside the closure
    // (per attempt), so the raw secret still never lands in this outer scope.
    let client = client.clone();
    let url = url.to_string();
    let sas_token = sas_token.clone();

    azure_upload_with_retry(
        async move || {
            // CSE → lazy AES-CBC encrypting stream; SSE Path → fresh
            // tokio::fs::File per retry; SSE Bytes → O(1) Arc clone.
            let body = cloud_http::body_for(&source, encryptor.as_ref())
                .await
                .context(azure_upload_error::SourceIoSnafu)?;

            // TODO(SNOW-3701467): add an in-transit integrity checksum (Azure verifies
            // `Content-MD5` / `x-ms-content-crc64`, or per-segment CRC64 via the
            // structured-body format) to match the S3 PUT path. Today this relies only
            // on TLS + the GET-time `sfc-digest` (verified over plaintext, on read).
            let mut req = client
                .put(build_sas_url(&url, sas_token.reveal()))
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
        policy,
    )
    .await?;

    tracing::debug!("Azure blob upload successful");
    Ok(())
}

// --- Retry logic (delegates to http::retry) ---

/// Returns a retry policy tuned for Azure file-transfer operations.
///
/// Azure treats 403 as retryable (SAS token clock skew / replication delays),
/// matching JDBC/ODBC behavior.
pub(crate) fn azure_retry_policy(base: &RetryPolicy) -> RetryPolicy {
    let mut policy = base.clone();
    policy.extra_retryable_statuses.insert(403);
    policy
}

/// Executes an Azure HTTP request with retry, then checks for Azure-specific status codes.
///
/// Unlike GCS, Azure does not have a `TokenExpired` (401) fast-fail path.
/// Azure SAS tokens are URL-embedded and produce 403 on expiry (which is already retried).
/// SAS tokens cannot be refreshed mid-request — a new query execution is needed.
///
/// `policy` is the whole `RetryPolicy` (which already carries `max_attempts`),
/// not a bare `max_attempts`, so the *backoff* is injectable — production
/// passes `azure_retry_policy(..)` while tests pass a zero-backoff variant.
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

    if response.status().is_success() {
        return Ok(response);
    }

    let status_code = response.status().as_u16();
    // Azure error bodies often echo the request URL — scrub SAS signatures.
    // TODO(SNOW-3406377): SAS-signature redaction is done ad-hoc via
    // `sanitize_sas` at each call site that surfaces an Azure error body or
    // transport string. Once a centralized secure-logging / redaction layer
    // exists, route these through it instead of string-scrubbing here, so the
    // redaction policy lives in one place rather than being re-applied (and
    // potentially missed) per call site.
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
    policy: &RetryPolicy,
) -> Result<(), AzureUploadError>
where
    F: AsyncFn() -> Result<reqwest::RequestBuilder, AzureUploadError>,
{
    cloud_http::upload_with_retry(policy, &AzureUploadRetry, build_request).await
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

/// Downloads a file from Azure, streams the response body without buffering the
/// full ciphertext in memory, and returns a [`CloudStreamingDownload`] that the
/// caller uses to read the body via a sync `Read` interface.
///
/// This is the internal streaming path used by `mod.rs`'s `download_single_file`.
/// The public `download_from_azure` keeps the old `DownloadResponse` shape for
/// the integration-test / retry-test surface.
///
/// Marked `pub` so the cfg-gated `file_manager::internal` re-export can surface
/// it to integration tests; the parent module `azure_transfer` is itself private,
/// so this is not part of the crate's public API.
pub async fn download_from_azure_streaming(
    stage_info: &StageInfo,
    filename: &str,
    policy: &RetryPolicy,
) -> Result<CloudStreamingDownload, AzureDownloadError> {
    let client = create_azure_client(stage_info)?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let (url, sas_token) = resolve_url_and_token(stage_info, &key)?;
    let response = azure_request_with_retry(
        || client.get(build_sas_url(&url, sas_token.reveal())),
        Method::GET,
        policy,
    )
    .await?;

    // cloud_byte_count from Content-Length (accurate for non-chunked responses).
    let cloud_byte_count = response.content_length().unwrap_or(0) as i64;

    let headers = response.headers();
    let digest = try_get_header(headers, AZURE_META_SFC_DIGEST)?;

    let cse_info = match try_get_header(headers, AZURE_META_ENCRYPTIONDATA)? {
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

            // A CSE object always carries its content digest alongside the
            // encryption headers — require it here so the decrypt path receives
            // metadata and digest as one inseparable unit.
            let digest = digest.context(azure_download_error::MissingMetadataSnafu {
                field: AZURE_META_SFC_DIGEST,
            })?;

            Some(CseDownloadInfo {
                metadata: EncryptedFileMetadata {
                    encrypted_key: enc_data.wrapped_content_key.encrypted_key,
                    iv: enc_data.content_encryption_iv,
                    material_desc,
                },
                digest,
            })
        }
        None => None,
    };

    Ok(CloudStreamingDownload {
        cloud_byte_count,
        cse_info,
        reader: cloud_http::spawn_byte_stream_producer(response),
    })
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
}

// --- Unit tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
    use crate::config::retry::Jitter;
    use crate::sensitive::SensitiveString;
    use bytes::Bytes;

    // Zero-backoff test policy lives in `file_manager::internal` so the in-crate
    // and external integration tests share one definition that derives from the
    // production `azure_retry_policy` (no drift). Aliased so call sites read
    // `test_policy(..)`.
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
    fn azure_retry_policy_includes_403() {
        let policy = azure_retry_policy(&base_policy());
        assert!(
            policy.extra_retryable_statuses.contains(&403),
            "403 should be retryable (SAS token clock skew / replication delays)"
        );
    }

    #[test]
    fn azure_retry_policy_max_elapsed_exceeds_request_timeout() {
        let policy = azure_retry_policy(&base_policy());
        assert_eq!(
            policy.max_elapsed,
            Duration::from_secs(600),
            "max_elapsed must exceed REQUEST_TIMEOUT_SECS (300s)"
        );
        assert!(
            policy.max_elapsed > Duration::from_secs(REQUEST_TIMEOUT_SECS),
            "retry budget must be larger than a single request timeout"
        );
    }

    #[test]
    fn azure_retry_policy_max_attempts() {
        let mut base = base_policy();
        base.max_attempts = 25;
        assert_eq!(azure_retry_policy(&base).max_attempts, 25);
        base.max_attempts = 1;
        assert_eq!(azure_retry_policy(&base).max_attempts, 1);
    }

    #[test]
    fn azure_retry_policy_backoff_bounds() {
        let p = azure_retry_policy(&base_policy());
        assert_eq!(p.backoff.base, Duration::from_millis(250));
        assert_eq!(p.backoff.cap, Duration::from_secs(16));
        assert_eq!(p.backoff.factor, 2.0);
        assert!(matches!(p.backoff.jitter, Jitter::Decorrelated));
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

    fn prepared_with_digest(digest: &str) -> PreparedUpload {
        PreparedUpload {
            source: ByteSource::Bytes(b"hello-azure".to_vec().into()).into(),
            digest: digest.to_string(),
            cse: None,
        }
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
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ false,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        )
        .await
        .expect("upload should succeed against the mock");
        assert_eq!(status, UploadStatus::Uploaded);
    }

    // ---------------------------------------------------------------
    // 8. Skip-decision in isolation — kills mutants the wiremock scenarios
    //    can't (see SkipDecision / classify_pre_upload_skip docs).
    //
    //    The six wiremock scenarios above couple the `skip_upload_on_content_match &&`
    //    guard with the HEAD-elision optimization (head_needed = !overwrite ||
    //    skip_match). The case that would expose a missing guard —
    //    overwrite=true, skip_match=false, remote-digest matches local — is
    //    UNREACHABLE through `upload_to_azure_or_skip` because HEAD is elided
    //    in that configuration. These direct unit tests bypass the elision so
    //    a guard regression fails here even when the integration scenarios
    //    still pass.
    // ---------------------------------------------------------------

    /// Mutation guard: dropping `skip_upload_on_content_match &&` from the
    /// content branch flips this to `ContentMatch` and the assertion fails.
    #[test]
    fn classify_does_not_fire_content_branch_without_opt_in() {
        let h = RemoteBlobHeader::with_digest("abc");
        let decision = classify_pre_upload_skip(
            /* overwrite */ true,
            /* skip_upload_on_content_match */ false,
            Some(&h),
            "abc",
        );
        assert_eq!(
            decision,
            SkipDecision::Upload,
            "content-match must require the opt-in flag"
        );
    }

    /// Positive control: the same digest match WITH the flag set fires.
    #[test]
    fn classify_fires_content_branch_with_opt_in() {
        let h = RemoteBlobHeader::with_digest("abc");
        let decision = classify_pre_upload_skip(true, true, Some(&h), "abc");
        assert_eq!(decision, SkipDecision::ContentMatch);
    }

    /// Existence wins over content-match when `!overwrite`: a blob that
    /// exists is treated as authoritative, digest comparison is skipped.
    #[test]
    fn classify_existence_wins_under_no_overwrite() {
        let h = RemoteBlobHeader::with_digest("abc");
        let decision = classify_pre_upload_skip(false, true, Some(&h), "abc");
        assert_eq!(decision, SkipDecision::Existence);
    }

    /// `!overwrite` with no remote means upload — blob doesn't exist yet.
    /// Common first-upload path.
    #[test]
    fn classify_uploads_when_remote_absent_under_no_overwrite() {
        let decision = classify_pre_upload_skip(false, false, None, "abc");
        assert_eq!(decision, SkipDecision::Upload);
    }

    /// `overwrite && skip_match && remote present but digest absent` — the
    /// HEAD returned 200 but no `x-ms-meta-sfcdigest` header. Cannot compare,
    /// so upload runs (fail-open at the comparison site).
    #[test]
    fn classify_uploads_when_remote_digest_missing() {
        let h = RemoteBlobHeader { digest: None };
        let decision = classify_pre_upload_skip(true, true, Some(&h), "abc");
        assert_eq!(decision, SkipDecision::Upload);
    }

    /// `overwrite && skip_match && remote digest differs` — the racing
    /// uploader had different content; we must overwrite, not skip.
    #[test]
    fn classify_uploads_when_digests_differ() {
        let h = RemoteBlobHeader::with_digest("xyz");
        let decision = classify_pre_upload_skip(true, true, Some(&h), "abc");
        assert_eq!(decision, SkipDecision::Upload);
    }

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
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        )
        .await
        .expect("retry should recover and existence skip should fire");
        assert_eq!(status, UploadStatus::Skipped);
    }

    /// Persistent 403 + !overwrite: HEAD retries up to max_attempts (6),
    /// then `upload_to_azure_or_skip` fails-CLOSED. PUT must not run.
    #[tokio::test(flavor = "multi_thread")]
    async fn head_fails_closed_on_persistent_403_when_not_overwrite() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(403))
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
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        )
        .await;
        assert!(
            matches!(
                &result,
                Err(AzureUploadError::RetryExhausted { detail, .. }) if detail.contains("403")
            ),
            "persistent 403 + !overwrite must fail-CLOSED with RetryExhausted carrying \"403\" in detail, not silently fall through to PUT; got: {result:?}"
        );
    }

    /// Persistent 403 + skip_match (overwrite=true): HEAD retries up to
    /// max_attempts (6), then fails-OPEN to PUT. HEAD-count = 6 pins
    /// that retry runs even on the skip_match branch.
    #[tokio::test(flavor = "multi_thread")]
    async fn head_fails_open_on_persistent_403_when_skip_match() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(403))
            .expect(DEFAULT_PUT_GET_MAX_ATTEMPTS as u64)
            .mount(&mock)
            .await;
        // Pins M3: the skip_match path must fall through to PUT after HEAD
        // exhaustion (fail-OPEN), not surface the HEAD error (fail-CLOSED).
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&mock)
            .await;

        let stage = mock_stage(&mock.uri());
        let status = upload_to_azure_or_skip(
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ true,
            /* skip_upload_on_content_match */ true,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        )
        .await
        .expect("skip_match path should fail-OPEN to PUT after HEAD retry exhaustion");
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
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
            prepared_with_digest("local-digest"),
            &stage,
            "f.dat",
            /* overwrite */ false,
            /* skip_upload_on_content_match */ false,
            &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
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
}
