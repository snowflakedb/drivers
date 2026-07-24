use super::cloud_http::{self, CloudStreamingDownload, CseDownloadInfo, UploadRetryAdapter};
use super::multipart::{self, MultipartConfig, MultipartParams};
use super::types::{
    ByteSource, CloudCredentials, DownloadResponse, EncryptedFileMetadata, EncryptionData,
    MaterialDescription, PreparedUpload, StageInfo, StageInfoRefreshError, StageInfoRefresher,
    UploadStatus, build_encryption_metadata_json, percent_encode_path,
};
use crate::config::retry::RetryPolicy;
use crate::http::retry::{HttpContext, HttpError, execute_with_retry as http_execute_with_retry};
use crate::log_foreign_error;
use crate::refresh::{Refresher, execute_with_refresh};
use crate::sensitive::SensitiveString;
use bytes::Bytes;
use futures::StreamExt as _;
use reqwest::{Method, StatusCode};
use snafu::{IntoError, Location, OptionExt, ResultExt, Snafu};
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;
use tempfile::NamedTempFile;

const REQUEST_TIMEOUT_SECS: u64 = 300;

// GCS metadata header names
const GCS_META_SFC_DIGEST: &str = "x-goog-meta-sfc-digest";
const GCS_META_ENCRYPTIONDATA: &str = "x-goog-meta-encryptiondata";
const GCS_META_MATDESC: &str = "x-goog-meta-matdesc";

/// Uploads a file to GCS, skipping when either:
///   * the object already exists and `overwrite` is false (existence skip), or
///   * the remote object's stored SHA-256 (`x-goog-meta-sfc-digest`) matches
///     the local payload's SHA-256 — even under `OVERWRITE=TRUE` (digest skip).
///
/// The existence check runs first (cheap, gated on `!overwrite`); the
/// content-match check runs regardless of `overwrite`. The HEAD is always
/// issued so both checks can share its result.
///
/// `refresher` drives the reactive stage-info recovery introduced by gaps
/// 2.1 (URL expiry) and 2.4 (token expiry):
/// - On HTTP 401 (bearer expired): the `GcsTokenRefresher` adapter calls
///   `refresher.refresh()` (coalesced, 10-min window) and retries with the
///   rotated creds. A second consecutive 401 with the same bearer surfaces
///   the existing `GcsUploadError::TokenExpired` — matching libsfclient's
///   `m_lastRefreshTokenSec` gate (`FileTransferAgent.cpp:412`).
/// - On HTTP 400 in presigned mode: an outer loop calls
///   `refresher.refresh_url()` (no coalesce) and retries with the rotated
///   `presignedUrl` from the cache. A second consecutive 400 surfaces the
///   new `GcsUploadError::PresignedUrlExpired` — matching Python's two-strike
///   guard in `gcs_storage_client.py`.
///
/// When `refresher` is `None`, neither recovery fires and the old shape is
/// preserved (400 stays on the wire-level retry list via `gcs_retry_policy`;
/// 401 surfaces as `TokenExpired` exactly as before).
pub async fn upload_to_gcs_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
    multipart: MultipartParams,
    policy: &RetryPolicy,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadStatus, GcsUploadError> {
    let client = create_gcs_client(stage_info)?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let using_presigned_url = stage_info.presigned_url.is_some();
    let has_refresher = refresher.is_some();

    // Tell the refresher which destination file is uploading so a per-file URL
    // refresh re-issues the PUT SQL rewritten for *this* file (multi-file glob
    // PUT). `filename` is the dst object name (incl. compression suffix).
    if let Some(r) = refresher.as_deref_mut() {
        r.notify_current_upload_file(filename.to_string());
    }
    // With a refresher, 400 is removed from the wire-level retry list — we
    // handle it reactively here by rotating the URL. Without a refresher,
    // the injected policy keeps the legacy 400-retry-with-same-URL fallback.
    let wire_policy = if has_refresher {
        without_400(policy)
    } else {
        policy.clone()
    };

    // Two-strike URL-refresh model. `make_attempt` is a factory that captures
    // `base` (the stage-info for this strike) so the attempt body is written
    // once and called twice with different bases. `Option<&mut dyn Trait>`
    // reborrows prevent extracting the two-strike orchestration into a shared
    // async helper across sequential awaits, so it stays inline here.
    let make_attempt = |base: &StageInfo| {
        let base = base.clone();
        let prepared = prepared.clone();
        let key = key.clone();
        let client = client.clone();
        let wire_policy = wire_policy.clone();
        move |snapshot: super::types::StageInfoSnapshot| {
            let stage_info = base.with_snapshot(snapshot);
            let prepared = prepared.clone();
            let key = key.clone();
            let client = client.clone();
            let wire_policy = wire_policy.clone();
            async move {
                let (url, token) = resolve_url_and_token(&stage_info, &key, None)
                    .map_err(map_gcs_request_error_for_attempt)?;

                let head = check_file_exists_gcs(&client, &url, token).await;

                if !overwrite && matches!(head, GcsHeadResult::Found { .. }) {
                    tracing::info!("File already exists in GCS: {key:?}");
                    return Ok(UploadStatus::Skipped);
                }

                // `prepared.digest` is the SHA-256 of the (compressed) plaintext for both
                // SSE and CSE stages (see `encryption.rs`), so it is stable across uploads
                // of identical content and matches the digest stored by this and other
                // drivers. The skip therefore fires whenever the remote content matches,
                // regardless of the encryption mode.
                if let GcsHeadResult::Found {
                    digest: Some(ref d),
                } = head
                    && d == &prepared.digest
                {
                    tracing::info!(
                        "Remote GCS object matches local content digest, skipping upload: {key:?}"
                    );
                    return Ok(UploadStatus::Skipped);
                }

                // Large files on the access-token path take the XML-API
                // resumable chunked upload (bounded in-flight memory + per-chunk
                // retry); the presigned-URL path and small files keep the single
                // PUT. `token.is_some()` already implies the access-token path —
                // `resolve_url_and_token` returns `None` for both presigned modes.
                let body_len = multipart::upload_body_len(&prepared)
                    .await
                    .map_err(|e| GcsRequestError::SourceIo { source: e })
                    .map_err(map_gcs_request_error_for_attempt)?;
                if let Some(tok) = token
                    && multipart.should_chunk(body_len)
                {
                    gcs_resumable_upload(&client, &url, tok, prepared, body_len, &wire_policy)
                        .await
                        .map_err(map_gcs_request_error_for_attempt)?;
                } else {
                    upload_to_gcs(&client, &url, token, prepared, &wire_policy)
                        .await
                        .map_err(map_gcs_request_error_for_attempt)?;
                }
                Ok(UploadStatus::Uploaded)
            }
        }
    };

    let first = run_gcs_with_token_refresh(
        refresher.as_deref_mut(),
        stage_info,
        |e| gcs_upload_error::StageInfoRefreshSnafu.into_error(e),
        make_attempt(stage_info),
    )
    .await;

    let needs_url_refresh = matches!(
        first,
        Err(GcsUploadError::GcsHttp {
            status_code: 400,
            ..
        })
    ) && using_presigned_url
        && has_refresher;

    if !needs_url_refresh {
        return first;
    }

    tracing::warn!("GCS PUT returned 400 in presigned mode; refreshing per-file URL and retrying");
    let refreshed_stage_info = {
        let Some(r) = refresher.as_mut() else {
            // Invariant: needs_url_refresh is only true when has_refresher
            // is true, so refresher must be Some here.
            unreachable!("refresher is Some: needs_url_refresh requires has_refresher");
        };
        match r.refresh_url().await {
            Ok(()) => {}
            Err(StageInfoRefreshError::PresignedUrlRefreshSkipped { .. }) => {
                // The PUT command had no parseable file:// path to rewrite for
                // this file, so a per-file URL refresh wasn't possible. Fail
                // fast rather than risk misrouting to another file's URL.
                tracing::warn!(
                    "GCS PUT 400 in presigned mode; per-file URL refresh not possible — \
                     surfacing PresignedUrlExpired"
                );
                return gcs_upload_error::PresignedUrlExpiredSnafu.fail();
            }
            Err(e) => return Err(gcs_upload_error::StageInfoRefreshSnafu.into_error(e)),
        }
        stage_info.with_snapshot(r.cache().snapshot())
    };

    let second = run_gcs_with_token_refresh(
        refresher.as_deref_mut(),
        &refreshed_stage_info,
        |e| gcs_upload_error::StageInfoRefreshSnafu.into_error(e),
        make_attempt(&refreshed_stage_info),
    )
    .await;

    match second {
        Err(GcsUploadError::GcsHttp {
            status_code: 400, ..
        }) => {
            tracing::warn!(
                "GCS PUT returned 400 again after URL refresh; failing fast with PresignedUrlExpired"
            );
            gcs_upload_error::PresignedUrlExpiredSnafu.fail()
        }
        other => other,
    }
}

/// Downloads a file from GCS and returns data with optional encryption metadata.
/// For SSE stages the metadata headers will be absent and `None` is returned.
///
/// The body length is verified against the GCS `Content-Length` header when
/// both are unambiguous (no `Content-Encoding` rewrite by the HTTP layer,
/// no chunked transfer). When the header is absent or `Content-Encoding` is
/// present the check is skipped.
///
/// `cloud_byte_count` reflects the on-cloud (pre-decryption) byte count of
/// the object — taken from the collected body length, which equals the
/// GCS `Content-Length` for non-streamed responses. This is the wire byte
/// count, not the decrypted/decoded size of the original file.
///
/// If this function ever switches to a streaming body reader, the
/// Content-Length check must move into the byte-counting stream wrapper.
///
/// `per_file_presigned_url` is the URL GS issued for this specific file via
/// `data.presignedUrls[i]` on GCS GET in presigned-only mode. When `Some`,
/// it takes precedence over `stage_info.presigned_url` (Strategy 0 in
/// `resolve_url_and_token`); when `None`, the function falls back to the
/// existing strategies (PUT-side single presigned URL, then bearer token,
/// then `MissingGcsCredentials`).
///
/// `per_file_index` is the file's position in the GET batch — used after a
/// 400-triggered URL refresh to re-pick `presigned_urls[per_file_index]`
/// from the refresher cache (the refreshed snapshot carries a fresh
/// `presignedUrls[]` array from GS).
///
/// `refresher` enables reactive stage-info recovery — see
/// `upload_to_gcs_or_skip` for the 401/400 handling shape. Specifically for
/// GET:
/// - 401 → `refresher.refresh()` (coalesced) → retry with rotated bearer.
/// - 400 in presigned mode → `refresher.refresh_url()` (no coalesce) →
///   re-pick `presigned_urls[per_file_index]` from the new snapshot, retry.
///
/// Returns the successful response (headers available, body not yet consumed);
/// shared by `download_from_gcs` (buffered) and `download_from_gcs_streaming`
/// so both download paths get this 401/400 refresh handling.
async fn gcs_get_with_refresh(
    stage_info: &StageInfo,
    filename: &str,
    per_file_presigned_url: Option<&str>,
    policy: &RetryPolicy,
    per_file_index: usize,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<reqwest::Response, GcsDownloadError> {
    let client = create_gcs_client(stage_info)?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    // Either presigned-URL source enables the 400-handling: the URL may
    // have expired and reissuing it produces a fresh signature. The
    // PUT-side single-slot URL and the per-file GET list are both signed
    // and both subject to the same expiry semantics.
    let using_presigned_url =
        per_file_presigned_url.is_some() || stage_info.presigned_url.is_some();
    let has_refresher = refresher.is_some();
    // With a refresher, 400 is removed from the wire-level retry list — we
    // handle it reactively here. Without a refresher, the injected policy
    // keeps the legacy 400-retry-with-same-URL fallback so today's tests pass.
    let wire_policy = if has_refresher {
        without_400(policy)
    } else {
        policy.clone()
    };
    let initial_per_file_url = per_file_presigned_url.map(str::to_string);

    // Two-strike URL-refresh model. `make_attempt` is a factory that takes
    // `base` (the stage-info for this strike) and `per_file_url`, returning
    // the attempt closure for `run_gcs_with_token_refresh`. Writing the body
    // once eliminates the duplication between first and second strikes.
    // Per-file URL re-pick after a 400 stays outside the closure.
    // `Option<&mut dyn Trait>` reborrows prevent extracting the two-strike
    // orchestration itself into a shared async helper across sequential awaits.
    let make_attempt = |base: &StageInfo, per_file_url: Option<String>| {
        let base = base.clone();
        let key = key.clone();
        let client = client.clone();
        let wire_policy = wire_policy.clone();
        move |snapshot: super::types::StageInfoSnapshot| {
            let stage_info = base.with_snapshot(snapshot);
            let key = key.clone();
            let client = client.clone();
            let per_file_url = per_file_url.clone();
            let wire_policy = wire_policy.clone();
            async move {
                let (url, token) =
                    resolve_url_and_token(&stage_info, &key, per_file_url.as_deref())
                        .map_err(map_gcs_request_error_for_attempt)?;

                gcs_request_with_retry(
                    || {
                        let mut req = client.get(&url);
                        if let Some(ref t) = token {
                            req = req.bearer_auth(t);
                        }
                        req
                    },
                    Method::GET,
                    &wire_policy,
                )
                .await
                .map_err(map_gcs_request_error_for_attempt)
            }
        }
    };

    let first = run_gcs_with_token_refresh(
        refresher.as_deref_mut(),
        stage_info,
        |e| gcs_download_error::StageInfoRefreshSnafu.into_error(e),
        make_attempt(stage_info, initial_per_file_url.clone()),
    )
    .await;

    let needs_url_refresh = matches!(
        first,
        Err(GcsDownloadError::GcsHttp {
            status_code: 400,
            ..
        })
    ) && using_presigned_url
        && has_refresher;

    let response = if !needs_url_refresh {
        first?
    } else {
        // GET-side presigned URL refresh is a deliberate enhancement beyond
        // legacy drivers: Python's `_update_presigned_url` returns early for
        // non-PUT statements, so a GET 400 would simply exhaust retries on the
        // same URL and fail. The two-strike guard here is more conservative —
        // fail rather than misroute — and aligns with the legacy "fail fast"
        // stance.
        tracing::warn!(
            "GCS GET returned 400 in presigned mode; refreshing per-file URL and retrying"
        );
        let (refreshed_stage_info, refreshed_per_file_url) = {
            let Some(r) = refresher.as_mut() else {
                // Invariant: needs_url_refresh is only true when has_refresher
                // is true, so refresher must be Some here.
                unreachable!("refresher is Some: needs_url_refresh requires has_refresher");
            };
            r.refresh_url()
                .await
                .context(gcs_download_error::StageInfoRefreshSnafu)?;
            let snap = r.cache().snapshot();
            let new_url = snap
                .presigned_urls
                .as_ref()
                .and_then(|urls| urls.get(per_file_index))
                .cloned()
                .flatten();
            // If the original request used a per-file presigned URL and the
            // refreshed snapshot does not supply one for this index, refuse the
            // fall-through to the single-slot presigned_url (PUT-side) or
            // bearer token — routing to either would serve the wrong object.
            if initial_per_file_url.is_some() && new_url.is_none() {
                return gcs_download_error::PresignedUrlExpiredSnafu.fail();
            }
            let new_stage_info = stage_info.with_snapshot(snap);
            (new_stage_info, new_url)
        };

        let second = run_gcs_with_token_refresh(
            refresher.as_deref_mut(),
            &refreshed_stage_info,
            |e| gcs_download_error::StageInfoRefreshSnafu.into_error(e),
            make_attempt(&refreshed_stage_info, refreshed_per_file_url),
        )
        .await;

        match second {
            Ok(resp) => resp,
            Err(GcsDownloadError::GcsHttp {
                status_code: 400, ..
            }) => {
                tracing::warn!(
                    "GCS GET returned 400 again after URL refresh; failing fast with PresignedUrlExpired"
                );
                return gcs_download_error::PresignedUrlExpiredSnafu.fail();
            }
            Err(e) => return Err(e),
        }
    };

    Ok(response)
}

/// Downloads a file from GCS into a buffered `DownloadResponse` (full body held
/// in memory). Used by the buffered consumers and the integration/retry tests;
/// `download_from_gcs_streaming` is the no-buffering variant. Both share
/// `gcs_get_with_refresh` for token/URL-refresh-aware response acquisition.
pub async fn download_from_gcs(
    stage_info: &StageInfo,
    filename: &str,
    per_file_presigned_url: Option<&str>,
    policy: &RetryPolicy,
    per_file_index: usize,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<DownloadResponse, GcsDownloadError> {
    let response = gcs_get_with_refresh(
        stage_info,
        filename,
        per_file_presigned_url,
        policy,
        per_file_index,
        refresher,
    )
    .await?;

    let headers = response.headers();
    let digest = try_get_header(headers, GCS_META_SFC_DIGEST)?;

    let expected_length: Option<u64> = match headers.get(reqwest::header::CONTENT_LENGTH) {
        Some(val) => match val.to_str().ok().and_then(|s| s.parse::<u64>().ok()) {
            Some(len) => Some(len),
            None => {
                tracing::warn!(
                    "Malformed Content-Length header on GCS download response, skipping length check"
                );
                None
            }
        },
        None => None,
    };
    let has_content_encoding = headers.get(reqwest::header::CONTENT_ENCODING).is_some();

    let file_metadata = match try_get_header(headers, GCS_META_ENCRYPTIONDATA)? {
        Some(encryption_data_str) => {
            let enc_data: EncryptionData = serde_json::from_str(&encryption_data_str)
                .context(gcs_download_error::DeserializationSnafu)?;

            let mat_desc_str = try_get_header(headers, GCS_META_MATDESC)?.context(
                gcs_download_error::MissingMetadataSnafu {
                    field: GCS_META_MATDESC,
                },
            )?;
            let material_desc: MaterialDescription = serde_json::from_str(&mat_desc_str)
                .context(gcs_download_error::DeserializationSnafu)?;

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
        .map_err(|source| GcsRequestError::Http { source })?
        .to_vec();
    let actual_len = data.len() as u64;

    if let Some(expected) = expected_length {
        if has_content_encoding {
            tracing::debug!(
                "Content-Encoding present on GCS response, skipping Content-Length verification"
            );
        } else if expected != actual_len {
            return gcs_download_error::ContentLengthMismatchSnafu {
                expected,
                actual: actual_len,
            }
            .fail();
        }
    } else {
        tracing::debug!("No Content-Length header on GCS response, skipping length verification");
    }

    let cloud_byte_count = actual_len as i64;

    Ok(DownloadResponse {
        data,
        digest,
        file_metadata,
        cloud_byte_count,
    })
}

/// Outcome of the pre-upload HEAD request against a GCS object.
///
/// `Found { digest }` carries the `x-goog-meta-sfc-digest` user-metadata
/// value (a Base64 SHA-256 string) when present. `digest` is `None` when
/// the header is absent (older objects, libsfclient S3-style uploads, etc.)
/// or when its bytes are not valid UTF-8. Callers must never log the digest
/// value — it is treated as PII-adjacent, matching the redaction discipline
/// elsewhere in this file.
#[derive(Debug, PartialEq, Eq)]
enum GcsHeadResult {
    NotFound,
    Found { digest: Option<String> },
}

/// Issue a HEAD against the GCS object and return `Found { digest }` on
/// 200, or `NotFound` otherwise.
///
/// Any non-200 status (including 403 / unexpected codes) and any
/// transport-level error are treated as `NotFound` — the caller falls
/// through to a PUT. A malformed sfc-digest header yields
/// `Found { digest: None }`; the digest comparison then misses and the
/// upload proceeds.
async fn check_file_exists_gcs(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
) -> GcsHeadResult {
    let mut request = client.head(url);
    if let Some(t) = token {
        request = request.bearer_auth(t);
    }

    match request.send().await {
        Ok(resp) => match resp.status() {
            StatusCode::OK => {
                let digest = match try_get_header(resp.headers(), GCS_META_SFC_DIGEST) {
                    Ok(value) => value,
                    Err(_) => {
                        // Header value is not valid UTF-8 — treat as
                        // "no digest known"; never log the bytes.
                        tracing::warn!(
                            "Non-UTF8 {GCS_META_SFC_DIGEST} header on GCS HEAD response, \
                             ignoring digest"
                        );
                        None
                    }
                };
                GcsHeadResult::Found { digest }
            }
            StatusCode::NOT_FOUND => GcsHeadResult::NotFound,
            StatusCode::FORBIDDEN => {
                tracing::warn!(
                    "Access denied checking file existence in GCS, proceeding with upload"
                );
                GcsHeadResult::NotFound
            }
            status => {
                tracing::warn!(
                    "Unexpected status {status} checking GCS file existence, proceeding with upload"
                );
                GcsHeadResult::NotFound
            }
        },
        Err(e) => {
            log_foreign_error!(
                warn,
                e,
                "Error checking GCS file existence, proceeding with upload"
            );
            GcsHeadResult::NotFound
        }
    }
}

/// Upload data to GCS with retry logic.
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
/// Returns the internal `GcsRequestError` so the attempt-error mapper can
/// dispatch `TokenExpired` into `GcsAttemptError::TokenExpired` (handled by
/// `run_gcs_with_token_refresh`) versus everything else into `Other`.
async fn upload_to_gcs(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    prepared: PreparedUpload,
    policy: &RetryPolicy,
) -> Result<(), GcsRequestError> {
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
        .context(SerializationSnafu)?;

    let mat_desc_str = encryption_metadata
        .as_ref()
        .map(|enc_meta| serde_json::to_string(&enc_meta.material_desc))
        .transpose()
        .context(SerializationSnafu)?;

    // Set Content-Length explicitly on every GCS upload, mirroring Azure: a
    // streaming `reqwest::Body` (a wrapped CSE stream, or a `tokio::fs::File`
    // for an SSE `Path` source) has no length reqwest can infer, so without this
    // it falls back to `Transfer-Encoding: chunked`. CSE uses the analytic
    // ciphertext length; SSE uses the source length (file metadata / buffer len).
    let content_length = match &encryptor {
        Some(enc) => enc.cipher_len(),
        None => match &source {
            ByteSource::Bytes(b) => b.len() as i64,
            ByteSource::Path(p) => {
                tokio::fs::metadata(p).await.context(SourceIoSnafu)?.len() as i64
            }
        },
    };

    // Own everything the per-attempt async closure touches so the closure is
    // self-contained (`'static`): an `AsyncFn` whose returned future borrowed
    // these from this frame couldn't satisfy the `'static` bound the FFI/trait
    // futures require. `reqwest::Client` clone is a cheap `Arc` bump.
    let client = client.clone();
    let url_owned = url.to_string();
    let token = token.map(str::to_string);

    gcs_upload_with_retry(
        async move || {
            // CSE → lazy AES-CBC encrypting stream; SSE Path → fresh
            // tokio::fs::File per retry; SSE Bytes → O(1) Arc clone.
            let body = cloud_http::body_for(&source, encryptor.as_ref())
                .await
                .context(SourceIoSnafu)?;

            // TODO(SNOW-3701467): add an in-transit integrity checksum (GCS verifies
            // `x-goog-hash: crc32c=<base64>` on upload, 400 on mismatch) to match the
            // S3 PUT path. Today this relies only on TLS + the GET-time `sfc-digest`
            // (verified over plaintext, on read), so corruption isn't caught at PUT.
            let mut req = client
                .put(&url_owned)
                .header(GCS_META_SFC_DIGEST, &digest)
                .header("content-encoding", "")
                .header(reqwest::header::CONTENT_LENGTH, content_length)
                .body(body);

            if let Some(ref enc_str) = encryption_data_str {
                req = req.header(GCS_META_ENCRYPTIONDATA, enc_str);
            }
            if let Some(ref md_str) = mat_desc_str {
                req = req.header(GCS_META_MATDESC, md_str);
            }
            if let Some(t) = &token {
                req = req.bearer_auth(t);
            }
            Ok(req)
        },
        &Method::PUT,
        url,
        policy,
    )
    .await?;

    tracing::debug!("GCS upload successful");
    Ok(())
}

/// Uploads `prepared` to GCS via the XML-API **resumable** protocol: a single
/// initiation `POST` (carrying the digest + CSE metadata) mints a session URL,
/// then the (optionally encrypting) source is cut into `chunk_size` chunks and
/// `PUT` to that session URL **sequentially** with `Content-Range` headers. The
/// final chunk's `200/201` ends the session; intermediate chunks return `308`
/// ("Resume Incomplete"). On any terminal error a best-effort `DELETE` releases
/// the half-staged session (GCS also GCs incomplete sessions on its own).
///
/// Used only on the access-token path for files at/above the multipart
/// threshold; the presigned-URL path and smaller files take the single
/// `Put`-object path in [`upload_to_gcs`]. Mirrors the Node.js connector's
/// `uploadFileResumable` (snowflake-connector-nodejs#1427) and the
/// Python/JDBC resumable upload model.
async fn gcs_resumable_upload(
    client: &reqwest::Client,
    object_url: &str,
    token: &str,
    prepared: PreparedUpload,
    body_len: u64,
    policy: &RetryPolicy,
) -> Result<(), GcsRequestError> {
    let chunk_size =
        multipart::compute_part_size(body_len, &MultipartConfig::GCS).map_err(|e| {
            GcsRequestError::FileTooLarge {
                detail: e.to_string(),
            }
        })?;

    // Digest + CSE metadata ride on the initiation POST (the GCS analogue of
    // Azure's metadata-on-commit), not on the per-chunk PUTs. CSE params (cloud
    // metadata + encryptor) are both present or both absent.
    let source = prepared.source.byte_source();
    let digest = prepared.digest;
    let (encryption_metadata, encryptor) = match prepared.cse {
        Some(c) => (Some(c.metadata), Some(c.encryptor)),
        None => (None, None),
    };
    let encryption_data_str = encryption_metadata
        .as_ref()
        .map(|enc_meta| serde_json::to_string(&build_encryption_metadata_json(enc_meta)))
        .transpose()
        .context(SerializationSnafu)?;
    let mat_desc_str = encryption_metadata
        .as_ref()
        .map(|enc_meta| serde_json::to_string(&enc_meta.material_desc))
        .transpose()
        .context(SerializationSnafu)?;

    let session_url = gcs_resumable_initiate(
        client,
        object_url,
        token,
        body_len,
        &digest,
        encryption_data_str.as_deref(),
        mat_desc_str.as_deref(),
        policy,
    )
    .await?;

    // Stream chunks into the session sequentially (a resumable session commits
    // chunks in order). Each chunk body is a materialized `Bytes`, so a
    // transient PUT failure is retried in place. On any terminal failure,
    // best-effort DELETE the session so it doesn't linger as a half-staged blob.
    let result = async {
        let mut rx = multipart::spawn_part_reader(source, encryptor, chunk_size as usize, 1);
        let mut offset: u64 = 0;
        let mut committed = false;
        while let Some(part) = rx.recv().await {
            let part = part.map_err(|source| GcsRequestError::SourceIo { source })?;
            let len = part.body.len() as u64;
            let done = gcs_put_one_chunk(
                client,
                &session_url,
                token,
                part.body,
                ChunkRange {
                    offset,
                    len,
                    total: body_len,
                },
                policy,
            )
            .await?;
            offset += len;
            if done {
                committed = true;
                break;
            }
        }
        if !committed {
            return Err(GcsRequestError::Resumable {
                detail: format!(
                    "resumable upload ended without a terminal 2xx commit after {offset} of {body_len} bytes"
                ),
            });
        }
        if offset != body_len {
            return Err(GcsRequestError::Resumable {
                detail: format!("resumable upload ended after {offset} of {body_len} bytes"),
            });
        }
        Ok(())
    }
    .await;

    // Abort on every observable failure so partial chunks don't orphan. When
    // the failure is a token expiry, this delete call uses the same expired
    // token and will itself fail (logged, not fatal); GCS then expires the
    // abandoned resumable session URI on its own (~1 week), and an incomplete
    // resumable upload never surfaces as a bucket object.
    // <https://cloud.google.com/storage/docs/resumable-uploads>
    //
    // NB: this only fires on Err. If a caller ever drops this future
    // (cancellation), no Err is produced and the session lingers until GCS GCs
    // it. Today no caller cancels PUT mid-flight (the whole operation runs
    // under a single block_on with no cancel token), so this path is currently
    // unreachable. When cancellation is introduced, add a Drop-based guard so
    // the DELETE fires on drop too.
    // TODO(SNOW-3704984)
    match result {
        Ok(()) => {
            tracing::debug!("GCS resumable upload committed ({body_len} bytes)");
            Ok(())
        }
        Err(e) => {
            gcs_resumable_delete(client, &session_url, token).await;
            Err(e)
        }
    }
}

/// Initiates a GCS XML-API resumable session against the bucket-path object URL
/// (`POST` + `x-goog-resumable: start`) and returns the session URL minted in
/// the `Location` response header. 401 maps to `TokenExpired` so the outer
/// refresh loop can rotate creds and retry.
///
/// Every argument is a distinct input (client, target URL, auth token, body
/// length, digest, encryption metadata, retry policy) needed to build and
/// retry the initiation request.
#[allow(clippy::too_many_arguments)]
async fn gcs_resumable_initiate(
    client: &reqwest::Client,
    object_url: &str,
    token: &str,
    body_len: u64,
    digest: &str,
    encryption_data_str: Option<&str>,
    mat_desc_str: Option<&str>,
    policy: &RetryPolicy,
) -> Result<String, GcsRequestError> {
    let resp = gcs_request_with_retry(
        || {
            let mut req = client
                .post(object_url)
                .bearer_auth(token)
                .header(reqwest::header::CONTENT_LENGTH, 0)
                .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                .header("x-goog-resumable", "start")
                .header("x-upload-content-length", body_len)
                .header("content-encoding", "")
                .header(GCS_META_SFC_DIGEST, digest);
            if let Some(enc) = encryption_data_str {
                req = req.header(GCS_META_ENCRYPTIONDATA, enc);
            }
            if let Some(md) = mat_desc_str {
                req = req.header(GCS_META_MATDESC, md);
            }
            req
        },
        Method::POST,
        policy,
    )
    .await?;

    resp.headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .ok_or_else(|| GcsRequestError::Resumable {
            detail: "initiation response carried no Location (session URL) header".to_string(),
        })
}

/// One resumable-upload chunk's byte span: absolute `offset`, chunk `len`, and
/// the object's `total` size. The `Content-Range` wire grammar is rendered
/// from this inside [`gcs_put_one_chunk`], mirroring how `gcs_get_range`
/// renders the `Range` header from a [`multipart::DownloadRange`].
#[derive(Debug, Clone, Copy)]
struct ChunkRange {
    offset: u64,
    len: u64,
    total: u64,
}

/// PUTs one resumable chunk (its byte span given by `range`) to `session_url`,
/// rendering the `Content-Range: bytes start-end/total` header locally.
/// Returns `Ok(true)` when the object is complete (final-chunk `200/201`),
/// `Ok(false)` on `308` ("Resume Incomplete"). Retries transient
/// transport/HTTP failures in place (the chunk body is re-sendable); 401
/// surfaces as `TokenExpired`.
async fn gcs_put_one_chunk(
    client: &reqwest::Client,
    session_url: &str,
    token: &str,
    body: Bytes,
    range: ChunkRange,
    policy: &RetryPolicy,
) -> Result<bool, GcsRequestError> {
    let content_range = format!(
        "bytes {}-{}/{}",
        range.offset,
        range.offset + range.len - 1,
        range.total
    );
    let backoff = &policy.backoff;
    let len = body.len();
    let mut attempt: u32 = 0;
    let mut delay_ms = backoff.base.as_millis() as f64;
    let start = std::time::Instant::now();
    // Only the host + path may be logged (ud-log-every-http-call-at-info); the
    // resumable session URL's query carries the upload_id, so strip it.
    let log_path = session_url.split(['?', '#']).next().unwrap_or("");
    loop {
        attempt += 1;
        if let Some(budget) = policy.max_elapsed {
            let elapsed = start.elapsed();
            if elapsed >= budget {
                return Err(GcsRequestError::RetryExhausted {
                    detail: format!(
                        "resumable chunk PUT deadline exceeded after {elapsed:?} (budget {budget:?})"
                    ),
                });
            }
        }
        tracing::info!(method = %Method::PUT, path = %log_path, attempt, "outbound HTTP call");
        let send = client
            .put(session_url)
            .bearer_auth(token)
            .header(reqwest::header::CONTENT_LENGTH, len)
            .header(reqwest::header::CONTENT_RANGE, &content_range)
            .body(reqwest::Body::from(body.clone()))
            .send()
            .await;
        match send {
            Ok(resp) => {
                let status = resp.status();
                let code = status.as_u16();
                tracing::info!(status = code, "HTTP response");
                if status == StatusCode::UNAUTHORIZED {
                    return Err(GcsRequestError::TokenExpired);
                }
                // 308 "Resume Incomplete" is the normal between-chunks signal;
                // 200/201 ends the session on the final chunk.
                if code == 308 {
                    return Ok(false);
                }
                if status.is_success() {
                    return Ok(true);
                }
                if cloud_http::is_retryable_status(code, &policy.extra_retryable_statuses)
                    && attempt < policy.max_attempts
                {
                    tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
                    delay_ms = cloud_http::next_delay_ms(delay_ms, backoff);
                    continue;
                }
                let body = cloud_http::read_error_body(resp).await;
                return Err(GcsRequestError::GcsHttp {
                    status_code: code,
                    body,
                });
            }
            Err(e) => {
                if attempt < policy.max_attempts {
                    tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
                    delay_ms = cloud_http::next_delay_ms(delay_ms, backoff);
                    continue;
                }
                return Err(GcsRequestError::Http { source: e });
            }
        }
    }
}

/// Best-effort `DELETE` of a resumable session URL to release a half-staged
/// upload after a terminal error. Failures are logged and swallowed — the
/// original upload error is what matters, and GCS GCs abandoned sessions itself.
async fn gcs_resumable_delete(client: &reqwest::Client, session_url: &str, token: &str) {
    // Only the host + path may be logged (ud-log-every-http-call-at-info); the
    // resumable session URL's query carries the upload_id, so strip it.
    let log_path = session_url.split(['?', '#']).next().unwrap_or("");
    tracing::info!(method = %Method::DELETE, path = %log_path, "outbound HTTP call");
    match client.delete(session_url).bearer_auth(token).send().await {
        Ok(resp) => tracing::info!(status = resp.status().as_u16(), "HTTP response"),
        Err(e) => {
            tracing::debug!("GCS resumable session cleanup DELETE failed (best-effort): {e}");
        }
    }
}

// --- Retry logic (delegates to http::retry) ---

/// Returns a retry policy tuned for GCS file-transfer operations.
///
/// GCS treats 403 as retryable (temporary credential issues), and 400 is
/// retryable when using presigned URLs (URL may have expired).
///
/// Single source of truth for the production policy: built at the GCS entry
/// fns (where `using_presigned_url` is known) and passed by `&RetryPolicy`
/// into the transfer fns, so the wire-retry helpers share one policy and
/// tests can inject a zero-backoff variant (`internal::gcs_test_retry_policy`).
///
/// When a refresher is wired in, 400 is removed from the wire-level retry
/// list at the entry fn (see `strip_400`) because the reactive recovery in
/// `upload_to_gcs_or_skip` / `download_from_gcs` handles it by rotating the
/// presigned URL — blind retry against the same dead URL would just burn the
/// retry budget. The legacy no-refresher path keeps 400 retryable to preserve
/// today's behavior for callers that don't pass a refresher.
pub(crate) fn gcs_retry_policy(using_presigned_url: bool, base: &RetryPolicy) -> RetryPolicy {
    let mut policy = base.clone();
    policy.extra_retryable_statuses.insert(403);
    if using_presigned_url {
        policy.extra_retryable_statuses.insert(400);
    }
    policy
}

/// Returns a clone of `policy` with HTTP 400 removed from the retryable set.
///
/// Used by the entry fns when a refresher is present: the reactive URL-refresh
/// recovery owns the 400 case, so blind wire-level retry against the dead URL
/// must not also fire.
fn without_400(policy: &RetryPolicy) -> RetryPolicy {
    let mut p = policy.clone();
    p.extra_retryable_statuses.remove(&400);
    p
}

/// Executes a GCS HTTP request with retry, then checks for GCS-specific status codes.
///
/// Takes the injected `&RetryPolicy` (not a bare `max_attempts`) so the
/// *backoff* is injectable — production passes `gcs_retry_policy(..)` while
/// tests pass a zero-backoff variant (`internal::gcs_test_retry_policy`).
async fn gcs_request_with_retry<F>(
    build_request: F,
    method: Method,
    policy: &RetryPolicy,
) -> Result<reqwest::Response, GcsRequestError>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    // `allow_post_retry` is a no-op for every method except POST/PATCH (see
    // `allow_retry` in `http::retry`), so it's safe to set unconditionally
    // here rather than have every caller remember it — the resumable-session
    // initiation POST (the only POST/PATCH caller today) needs its transient
    // failures retried like every other GCS request.
    let ctx = HttpContext::new(method, "gcs-transfer").allow_post_retry();

    let response = http_execute_with_retry(build_request, &ctx, policy, |r| async move { Ok(r) })
        .await
        .map_err(map_http_error)?;

    if response.status().is_success() {
        return Ok(response);
    }

    // 401: token expired — propagate up so the query layer can re-execute
    if response.status() == StatusCode::UNAUTHORIZED {
        return Err(GcsRequestError::TokenExpired);
    }

    let status_code = response.status().as_u16();
    let body = cloud_http::read_error_body(response).await;
    Err(GcsRequestError::GcsHttp { status_code, body })
}

/// Adapter that wires `GcsRequestError` variants into the shared
/// [`cloud_http::upload_with_retry`] loop. The 401 special-case is GCS-only
/// — Snowflake's GS layer drives token refresh from a `TokenExpired` error,
/// so the upload path must propagate it eagerly (as `GcsRequestError::TokenExpired`)
/// rather than retrying, letting `upload_to_gcs_or_skip` orchestrate the refresh.
struct GcsUploadRetry;

impl UploadRetryAdapter for GcsUploadRetry {
    type Err = GcsRequestError;
    type BuildErr = GcsRequestError;

    fn on_build_err(&self, e: GcsRequestError) -> GcsRequestError {
        e
    }

    fn on_special_status(&self, status: StatusCode) -> Option<GcsRequestError> {
        (status == StatusCode::UNAUTHORIZED).then_some(GcsRequestError::TokenExpired)
    }

    fn on_http_failure(&self, status_code: u16, body: String) -> GcsRequestError {
        GcsRequestError::GcsHttp { status_code, body }
    }

    fn on_transport(&self, e: reqwest::Error) -> GcsRequestError {
        GcsRequestError::Http { source: e }
    }

    fn on_exhausted(&self, detail: String) -> GcsRequestError {
        GcsRequestError::RetryExhausted {
            detail: format!("GCS upload {detail}"),
        }
    }
}

/// Executes a GCS upload with retry, accepting a **fallible** request-builder closure.
///
/// Unlike `gcs_request_with_retry`, the closure may return `Err(GcsRequestError)`
/// (e.g. if the source file cannot be opened on a retry attempt). A build failure
/// is treated as non-retryable and propagated immediately — it indicates a local
/// problem (missing file, permission denied) rather than a transient network error.
///
/// Returns `GcsRequestError` so the caller (`upload_to_gcs`) keeps the same
/// token-refresh dispatch (`map_gcs_request_error_for_attempt`) as the
/// non-streaming path.
///
/// Takes the injected `&RetryPolicy` (not a bare `max_attempts`) for the same
/// reason as `gcs_request_with_retry`: the backoff is injectable for tests.
async fn gcs_upload_with_retry<F>(
    build_request: F,
    method: &Method,
    url: &str,
    policy: &RetryPolicy,
) -> Result<(), GcsRequestError>
where
    F: AsyncFn() -> Result<reqwest::RequestBuilder, GcsRequestError>,
{
    cloud_http::upload_with_retry(policy, &GcsUploadRetry, method, url, build_request).await
}

fn map_http_error(e: HttpError) -> GcsRequestError {
    match e {
        HttpError::Transport { source, .. } => GcsRequestError::Http { source },
        other => GcsRequestError::RetryExhausted {
            detail: other.to_string(),
        },
    }
}

// --- Helpers ---

fn create_gcs_client(stage_info: &StageInfo) -> Result<reqwest::Client, GcsRequestError> {
    let builder = crate::tls::client::configure_tls_builder(
        reqwest::Client::builder().timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
        &stage_info.tls_config,
        stage_info.crl_worker.clone(),
    )
    .map_err(|e| {
        ClientSetupSnafu {
            detail: e.to_string(),
        }
        .build()
    })?
    // Disable reqwest's auto-gzip path so a GCS response carrying
    // `Content-Encoding: gzip` (typically set by external loaders such
    // as `gsutil cp -Z` or BigQuery exports) is handed to the caller
    // verbatim. The driver is moving opaque, possibly CSE-encrypted
    // bytes, and downstream SHA-256 digest / Content-Length checks
    // assume wire bytes == body bytes. Mirrors JDBC's
    // `HttpUtil.disableContentCompression()`
    // (`SnowflakeGCSClient.java:237,:432` via `HttpUtil.java:420`) and
    // the intent of Python's `remove_content_encoding` urllib3 hook
    // (`storage_client.py:54-59`); the upload-side `content-encoding`
    // strip in `upload_to_gcs` is the matching PUT-side defense.
    .no_gzip();
    builder
        .build()
        .map_err(|source| GcsRequestError::Http { source })
}

/// Constructs the GCS URL and extracts the bearer token from stage info.
///
/// URL strategy priority (matching JDBC/ODBC/Python):
/// 0. Per-file presigned URL (GET, `data.presignedUrls[i]`) — use directly,
///    no token. Wins over the stage-info single slot to mirror Python's
///    `meta.presigned_url or stage_info.get("presignedUrl")` order in
///    `gcs_storage_client.py:77`. Reasoning: GS issues this URL for this
///    specific object; the token is generic and may have narrower ACLs.
/// 1. `stage_info.presigned_url` (PUT-side single slot) — use directly,
///    no token. PUT path is unchanged by step 2.2.
/// 2. Custom endpoint — `https://{endpoint}/{bucket}/{key}`
/// 3. Virtual host — `https://{bucket}.storage.googleapis.com/{key}`
/// 4. Regional — `https://storage.{region}.rep.googleapis.com/{bucket}/{key}`
/// 5. Default — `https://storage.googleapis.com/{bucket}/{key}`
fn resolve_url_and_token<'a>(
    stage_info: &'a StageInfo,
    key: &str,
    per_file_presigned_url: Option<&str>,
) -> Result<(String, Option<&'a str>), GcsRequestError> {
    // Strategy 0: per-file presigned URL (GCS GET multi-file path)
    if let Some(presigned) = per_file_presigned_url {
        return Ok((presigned.to_string(), None));
    }

    // Strategy 1: stage-info presigned URL (PUT path)
    if let Some(presigned) = &stage_info.presigned_url {
        return Ok((presigned.clone(), None));
    }

    // Extract token reference — avoids copying into a non-zeroized String
    let token = match &stage_info.creds {
        CloudCredentials::Gcs { gcs_access_token } => {
            gcs_access_token.as_ref().map(|t| t.reveal().as_str())
        }
        _ => return Err(GcsRequestError::MissingGcsCredentials),
    };

    if token.is_none() {
        return Err(GcsRequestError::MissingGcsCredentials);
    }

    let url = build_gcs_url(stage_info, key);
    Ok((url, token))
}

/// Builds the GCS URL based on endpoint/virtual/regional flags.
fn build_gcs_url(stage_info: &StageInfo, key: &str) -> String {
    let encoded_key = percent_encode_path(key);

    // Strategy 2: custom endpoint
    if let Some(ref ep) = stage_info.endpoint
        && !ep.is_empty()
    {
        let base = if ep.starts_with("https://") || ep.starts_with("http://") {
            ep.clone()
        } else {
            format!("https://{ep}")
        };
        return format!("{base}/{}/{encoded_key}", stage_info.bucket);
    }

    // Strategy 3: virtual host
    if stage_info.use_virtual_url {
        return format!(
            "https://{}.storage.googleapis.com/{encoded_key}",
            stage_info.bucket
        );
    }

    // Strategy 4: regional
    if stage_info.use_regional_url {
        return format!(
            "https://storage.{}.rep.googleapis.com/{}/{encoded_key}",
            stage_info.region.to_lowercase(),
            stage_info.bucket
        );
    }

    // Strategy 5: default
    format!(
        "https://storage.googleapis.com/{}/{encoded_key}",
        stage_info.bucket
    )
}

fn try_get_header(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<Option<String>, GcsDownloadError> {
    match headers.get(name) {
        Some(value) => {
            let s = value
                .to_str()
                .context(gcs_download_error::InvalidHeaderValueSnafu)?;
            Ok(Some(s.to_string()))
        }
        None => Ok(None),
    }
}

/// Downloads a file from GCS, streams the response body without buffering the
/// full ciphertext in memory, and returns a [`CloudStreamingDownload`] that the
/// caller can use to read the body via a sync `Read` interface.
///
/// This is the internal streaming path used by `mod.rs`'s `download_single_file`.
/// The public `download_from_gcs` keeps the old `DownloadResponse` shape for
/// the integration-test / retry-test surface.
///
/// The body is streamed from the HTTP response through a tokio-spawned producer
/// task into a `std::sync::mpsc::sync_channel`. `StreamReader` consumes from
/// the channel, implementing `Read` so `decrypt_ciphertext_to_writer` (which is
/// sync) can consume the body without blocking the async runtime.
///
/// `pub` so the cfg-gated `file_manager::internal` re-export in `mod.rs`
/// can surface it to integration tests via `pub use`; the parent module
/// `gcs_transfer` is itself private, so this is not part of the crate's public API.
///
/// Every argument is a distinct input (stage identity, filename, presigned-URL
/// override, retry policy, per-file index for logging, multipart params, the
/// refresh hook, and where to spill the body) mirroring the other cloud
/// download entry points' presigned-url + refresh + multipart surface.
#[allow(clippy::too_many_arguments)]
pub async fn download_from_gcs_streaming(
    stage_info: &StageInfo,
    filename: &str,
    per_file_presigned_url: Option<&str>,
    policy: &RetryPolicy,
    per_file_index: usize,
    multipart: MultipartParams,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
    spill_target: cloud_http::CloudSpillTarget<'_>,
) -> Result<CloudStreamingDownload, GcsDownloadError> {
    // Ranged (multipart) download applies only on the access-token path — a
    // presigned URL is signed for one specific GET and can't serve a HEAD/Range
    // probe. For access-token sessions, probe the size and, when it's at/above
    // the threshold, fetch the object with parallel ranged GETs into a tempfile.
    let using_presigned_url =
        per_file_presigned_url.is_some() || stage_info.presigned_url.is_some();
    if !using_presigned_url
        && let Some(ranged) = gcs_try_ranged_download(
            stage_info,
            filename,
            policy,
            multipart,
            refresher,
            spill_target,
        )
        .await?
    {
        return Ok(ranged);
    }
    // else: presigned session, or object below the threshold / size not
    // probeable — fall through to the single streamed GET below.

    let response = gcs_get_with_refresh(
        stage_info,
        filename,
        per_file_presigned_url,
        policy,
        per_file_index,
        refresher,
    )
    .await?;

    // cloud_byte_count from Content-Length (accurate for non-chunked responses).
    // Falls back to 0 when the header is absent; mod.rs uses the actual written
    // byte count as a fallback for the Python flavor.
    let cloud_byte_count = response.content_length().unwrap_or(0) as i64;

    let cse_info = parse_gcs_cse_info(response.headers())?;

    let reader = cloud_http::spawn_byte_stream_producer(response);
    let cloud_bytes_read = reader.bytes_read_handle();
    Ok(CloudStreamingDownload {
        cloud_byte_count,
        cse_info,
        cloud_bytes_read,
        body: cloud_http::CloudDownloadBody::Streamed(Box::new(reader)),
    })
}

/// Parses the `sfc-digest` and (for CSE) the `encryptiondata` / `matdesc` GCS
/// user-metadata headers into a [`CseDownloadInfo`] — `Some` for a
/// client-side-encrypted object (where the content digest must ride alongside
/// the encryption headers), `None` for SSE / raw objects. Shared by the single
/// streamed GET and the ranged-download path.
fn parse_gcs_cse_info(
    headers: &reqwest::header::HeaderMap,
) -> Result<Option<CseDownloadInfo>, GcsDownloadError> {
    let digest = try_get_header(headers, GCS_META_SFC_DIGEST)?;
    let Some(encryption_data_str) = try_get_header(headers, GCS_META_ENCRYPTIONDATA)? else {
        return Ok(None);
    };
    let enc_data: EncryptionData = serde_json::from_str(&encryption_data_str)
        .context(gcs_download_error::DeserializationSnafu)?;

    let mat_desc_str = try_get_header(headers, GCS_META_MATDESC)?.context(
        gcs_download_error::MissingMetadataSnafu {
            field: GCS_META_MATDESC,
        },
    )?;
    let material_desc: MaterialDescription =
        serde_json::from_str(&mat_desc_str).context(gcs_download_error::DeserializationSnafu)?;

    // A CSE object should carry its content digest alongside the encryption
    // headers. If sfc-digest is absent (e.g. git stage objects on GCS), fall
    // through to raw bytes, matching the S3 behaviour fixed in #117.
    let Some(digest) = digest else {
        tracing::debug!("GCS encryptiondata present but sfc-digest absent; returning raw bytes");
        return Ok(None);
    };

    Ok(Some(CseDownloadInfo {
        metadata: EncryptedFileMetadata {
            encrypted_key: enc_data.wrapped_content_key.encrypted_key,
            iv: enc_data.content_encryption_iv,
            material_desc,
        },
        digest,
    }))
}

/// On the access-token path, probe the object via HEAD and — when it is at/above
/// the multipart threshold — fetch it with parallel ranged GETs into a tempfile,
/// returning a [`CloudStreamingDownload`] backed by that tempfile. Returns
/// `Ok(None)` to mean "fall through to the single streamed GET" (object below
/// the threshold, or the HEAD couldn't determine a usable size). The whole
/// probe+fetch runs inside the token-refresh loop, so a 401 at any step rotates
/// creds and retries from the HEAD.
async fn gcs_try_ranged_download(
    stage_info: &StageInfo,
    filename: &str,
    policy: &RetryPolicy,
    multipart: MultipartParams,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
    spill_target: cloud_http::CloudSpillTarget<'_>,
) -> Result<Option<CloudStreamingDownload>, GcsDownloadError> {
    let client = create_gcs_client(stage_info)?;
    let key = format!("{}{filename}", stage_info.key_prefix);

    let make_attempt = |base: &StageInfo| {
        let base = base.clone();
        let key = key.clone();
        let client = client.clone();
        move |snapshot: super::types::StageInfoSnapshot| {
            let stage_info = base.with_snapshot(snapshot);
            let key = key.clone();
            let client = client.clone();
            async move {
                let (url, token) = resolve_url_and_token(&stage_info, &key, None)
                    .map_err(map_gcs_request_error_for_attempt)?;
                // Access-token path only (the caller already gated on
                // `!using_presigned_url`); defensively fall through otherwise.
                let Some(token) = token else {
                    return Ok(None);
                };
                gcs_ranged_download_attempt(&client, &url, token, multipart, policy, spill_target)
                    .await
            }
        }
    };

    run_gcs_with_token_refresh(
        refresher.as_deref_mut(),
        stage_info,
        |e| gcs_download_error::StageInfoRefreshSnafu.into_error(e),
        make_attempt(stage_info),
    )
    .await
}

/// One ranged-download attempt: HEAD for size + metadata, then parallel ranged
/// GETs into a tempfile. Returns `Ok(None)` to fall through to the single GET
/// (sub-threshold, or HEAD couldn't size the object). 401 surfaces as
/// `GcsAttemptError::TokenExpired` so the refresh loop rotates creds.
async fn gcs_ranged_download_attempt(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    multipart: MultipartParams,
    policy: &RetryPolicy,
    spill_target: cloud_http::CloudSpillTarget<'_>,
) -> Result<Option<CloudStreamingDownload>, GcsAttemptError<GcsDownloadError>> {
    // HEAD probe. A 401 must refresh; any other probe failure degrades to the
    // proven single-GET path (`Ok(None)`).
    let head =
        match gcs_request_with_retry(|| client.head(url).bearer_auth(token), Method::HEAD, policy)
            .await
        {
            Ok(resp) => resp,
            Err(GcsRequestError::TokenExpired) => return Err(GcsAttemptError::TokenExpired),
            Err(_) => return Ok(None),
        };

    let Some(content_length) = head
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
    else {
        return Ok(None);
    };
    if !multipart.should_chunk(content_length) {
        return Ok(None);
    }

    let cse_info = parse_gcs_cse_info(head.headers()).map_err(GcsAttemptError::Other)?;
    let chunk_size =
        multipart::compute_part_size(content_length, &MultipartConfig::GCS).map_err(|e| {
            GcsAttemptError::Other(
                gcs_download_error::FileTooLargeSnafu {
                    detail: e.to_string(),
                }
                .build(),
            )
        })?;

    tracing::debug!(
        "GCS ranged download: content_length={content_length} chunk_size={chunk_size} \
         concurrency={}",
        multipart.concurrency
    );
    let spilled = gcs_range_download(
        client,
        url,
        token,
        content_length,
        chunk_size,
        multipart.concurrency,
        policy,
        spill_target,
    )
    .await
    .map_err(map_gcs_request_error_for_attempt)?;

    // Spilled body assembled in the destination dir: a non-encrypted download
    // wrote straight into `.part` (renamed into place by the caller), while a
    // CSE/git-stage download wrote ciphertext to a temp. Size is known from the
    // HEAD `Content-Length`, so the chunked-TE bytes-pulled fallback
    // (`cloud_bytes_read`) stays 0.
    Ok(Some(CloudStreamingDownload {
        cloud_byte_count: content_length as i64,
        cse_info,
        cloud_bytes_read: Arc::new(AtomicU64::new(0)),
        body: cloud_http::CloudDownloadBody::Spilled(spilled),
    }))
}

/// Downloads the object with parallel ranged GETs into a pre-allocated file,
/// returning the assembled [`CloudSpilledBody`](cloud_http::CloudSpilledBody).
/// Ranges are fetched up to `concurrency` at a time and written at their
/// absolute offset (`pwrite`), so out-of-order completion is fine.
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
/// loss), and then it is a self-documenting, self-overwriting `<dst>.part`.
///
/// Every argument is a distinct input (connection + request identity, the
/// object's size and chosen chunk size, how many ranges to fetch at once, the
/// retry policy, and the spill target); Azure's `azure_range_download` stays
/// under the limit only because its auth rides in the URL/SAS rather than a
/// separate bearer `token` argument.
#[allow(clippy::too_many_arguments)]
async fn gcs_range_download(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    content_length: u64,
    chunk_size: u64,
    concurrency: usize,
    policy: &RetryPolicy,
    target: cloud_http::CloudSpillTarget<'_>,
) -> Result<cloud_http::CloudSpilledBody, GcsRequestError> {
    let mk_temp_err = |detail: String| GcsRequestError::TempFile { detail };

    // Owns the assembly file for the duration of the download: either the
    // caller's `.part` (referenced by path) or a RAII temp.
    enum Assembly {
        Part(std::path::PathBuf),
        Temp(NamedTempFile),
    }

    // Create + pre-allocate the assembly file off-thread. spawn_blocking (not
    // block_in_place): safe on current-thread runtimes; block_in_place panics
    // on those. Returns the owner plus a shared, cloneable handle for the
    // concurrent positioned writes.
    let owned_target = match target {
        cloud_http::CloudSpillTarget::Part(p) => (true, p.to_path_buf()),
        cloud_http::CloudSpillTarget::Temp(d) => (false, d.to_path_buf()),
    };
    #[allow(clippy::result_large_err)]
    let (assembly, file) = tokio::task::spawn_blocking(move || {
        let (is_part, path_or_dir) = owned_target;
        if is_part {
            // TODO(SNOW-3832603): the SSE ranged-download `.part` is created with the
            // process umask, bypassing the owner-only 0o600 hardening that
            // `create_output_file` applies when `unsafe_file_write == false`. This is a
            // shared cross-cloud gap — S3 (`s3_range_download`) and Azure
            // (`azure_range_download`) create their `.part` the same way, and
            // `unsafe_file_write` is not threaded into any streaming ranged-download
            // signature. Thread `unsafe_file_write` through and route Part creation via
            // `create_output_file` so SSE plaintext ranged downloads on S3/Azure/GCS all
            // get owner-only perms.
            let f = std::fs::File::create(&path_or_dir).map_err(|e| mk_temp_err(e.to_string()))?;
            f.set_len(content_length)
                .map_err(|e| mk_temp_err(e.to_string()))?;
            Ok::<_, GcsRequestError>((Assembly::Part(path_or_dir), Arc::new(f)))
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
            Ok((Assembly::Temp(named), file))
        }
    })
    .await
    .map_err(|e| mk_temp_err(format!("join error in tempfile setup: {e}")))??;

    // Drain, don't short-circuit: `collect` (not `try_collect`) polls EVERY
    // range future to completion, so all in-flight `write_at` tasks finish and
    // release their cloned file handles before we return. With no writer holding
    // the file open, the cleanup below can unlink it even on Windows. The first
    // error is surfaced after the drain.
    let ranges = multipart::plan_ranges(content_length, chunk_size);
    let results: Vec<Result<(), GcsRequestError>> = futures::stream::iter(ranges)
        .map(|range| {
            let file = Arc::clone(&file);
            async move {
                let bytes = gcs_get_range(client, url, token, &range, policy).await?;
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
            Ok(_) => Ok(cloud_http::CloudSpilledBody::Part(path)),
            Err(e) => {
                // Drained above, so no writer still holds `.part` open; the
                // best-effort remove succeeds even on Windows.
                let _ = tokio::task::spawn_blocking(move || std::fs::remove_file(&path)).await;
                Err(e)
            }
        },
        // On success hand out the unlink-on-drop guard; on failure `named` drops
        // here and NamedTempFile unlinks it (drained, so no open writer).
        Assembly::Temp(named) => {
            outcome.map(|_| cloud_http::CloudSpilledBody::Temp(named.into_temp_path()))
        }
    }
}

/// Ranged GET of `[range.start, range.end]`, returning the body bytes. 401
/// surfaces as `GcsRequestError::TokenExpired` via `gcs_request_with_retry`.
async fn gcs_get_range(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    range: &multipart::DownloadRange,
    policy: &RetryPolicy,
) -> Result<Bytes, GcsRequestError> {
    let range_header = format!("bytes={}-{}", range.start, range.end);
    let resp = gcs_request_with_retry(
        || {
            client
                .get(url)
                .bearer_auth(token)
                .header(reqwest::header::RANGE, &range_header)
        },
        Method::GET,
        policy,
    )
    .await?;
    let bytes = resp
        .bytes()
        .await
        .map_err(|source| GcsRequestError::Http { source })?;
    // Guard against endpoints that ignore Range and return the whole object
    // (200 not 206): writing at range.start would corrupt the assembled file
    // by overrunning the pre-allocated length.
    let expected_len = range.end - range.start + 1;
    if bytes.len() as u64 != expected_len {
        return Err(GcsRequestError::RangeNotHonored {
            detail: format!(
                "ranged GET returned {} bytes, expected {expected_len} \
                 (bytes={}-{}); endpoint may not honour Range header",
                bytes.len(),
                range.start,
                range.end
            ),
        });
    }
    Ok(bytes)
}

// --- Reactive recovery scaffolding (mirror s3_transfer.rs) ---

/// Internal error type for one attempt of a GCS operation. The `TokenExpired`
/// arm is the recoverable signal `should_refresh` matches on in
/// `GcsTokenRefresher`; everything else lives in `Other`. Mirrors
/// `S3AttemptError` — stays internal so `GcsUploadError` / `GcsDownloadError`
/// retain their public-API shape.
#[derive(Debug)]
enum GcsAttemptError<E> {
    TokenExpired,
    Other(E),
}

/// Maps the internal `GcsRequestError` into a per-attempt error so the token
/// refresh loop can catch 401 separately from everything else. Anything that
/// isn't 401 — including the new reactive 400 (presigned-URL expired) — goes
/// through `Other`, so the outer 400-handling loop in
/// `upload_to_gcs_or_skip` / `download_from_gcs` can match on it.
fn map_gcs_request_error_for_attempt<E: From<GcsRequestError>>(
    err: GcsRequestError,
) -> GcsAttemptError<E> {
    match err {
        GcsRequestError::TokenExpired => GcsAttemptError::TokenExpired,
        other => GcsAttemptError::Other(E::from(other)),
    }
}

/// GCS token-refresh implementation of the generic [`Refresher`] trait.
/// Mirrors `S3StsRefresher` in `s3_transfer.rs`: drives the retry loop in
/// `execute_with_refresh` by reading creds from a `StageInfoRefresher`'s
/// shared cache and asking it to rotate when GCS returns 401.
///
/// Tracks the last bearer token handed out so a refresh that doesn't
/// actually rotate (refresher inside its 10-min coalescing window — bit-for-bit
/// identical to libsfclient's `m_lastRefreshTokenSec` at
/// `FileTransferAgent.cpp:412`) reports `Ok(false)` and the helper propagates
/// the original error rather than spinning. The bearer comparison goes through
/// `SensitiveString`'s `reveal` boundary only here; the comparison result is a
/// plain `bool` and the strings themselves don't escape.
struct GcsTokenRefresher<'a, E, W> {
    refresher: &'a mut dyn StageInfoRefresher,
    last_seen_token: Option<SensitiveString>,
    map_refresh_err: W,
    _marker: PhantomData<fn() -> E>,
}

impl<'a, E, W> GcsTokenRefresher<'a, E, W>
where
    W: Fn(StageInfoRefreshError) -> E,
{
    fn new(refresher: &'a mut dyn StageInfoRefresher, map_refresh_err: W) -> Self {
        let last_seen_token =
            gcs_bearer_token(&refresher.cache().snapshot().creds).map(SensitiveString::from);
        Self {
            refresher,
            last_seen_token,
            map_refresh_err,
            _marker: PhantomData,
        }
    }
}

impl<'a, E, W> Refresher<super::types::StageInfoSnapshot, GcsAttemptError<E>>
    for GcsTokenRefresher<'a, E, W>
where
    E: Send,
    W: Fn(StageInfoRefreshError) -> E + Send,
{
    fn current(
        &mut self,
    ) -> crate::refresh::RefreshFuture<
        '_,
        Result<super::types::StageInfoSnapshot, GcsAttemptError<E>>,
    > {
        let snap = self.refresher.cache().snapshot();
        Box::pin(async move { Ok(snap) })
    }

    fn should_refresh(&self, err: &GcsAttemptError<E>) -> bool {
        matches!(err, GcsAttemptError::TokenExpired)
    }

    fn refresh(&mut self) -> crate::refresh::RefreshFuture<'_, Result<bool, GcsAttemptError<E>>> {
        Box::pin(async move {
            tracing::info!("GCS hit 401; refreshing stage info (creds)");
            self.refresher
                .refresh()
                .await
                .map_err(|e| GcsAttemptError::Other((self.map_refresh_err)(e)))?;
            let new = self.refresher.cache().snapshot();
            let new_token = gcs_bearer_token(&new.creds).map(SensitiveString::from);
            if new_token.as_ref().map(|s| s.reveal().as_str())
                == self.last_seen_token.as_ref().map(|s| s.reveal().as_str())
            {
                // Refresher coalesced or returned the same bearer — retrying
                // would loop, so decline further rotations.
                return Ok(false);
            }
            self.last_seen_token = new_token;
            Ok(true)
        })
    }
}

/// Returns the bearer string from GCS credentials, or None for non-GCS
/// variants. Used as the rotation marker; a different bearer implies a
/// fresh GS rotation. Mirrors `aws_key_id` in `s3_transfer.rs`.
fn gcs_bearer_token(creds: &CloudCredentials) -> Option<&str> {
    match creds {
        CloudCredentials::Gcs {
            gcs_access_token: Some(t),
        } => Some(t.reveal().as_str()),
        _ => None,
    }
}

/// Runs `attempt` once (no refresher) or in a refresh-retry loop (with
/// refresher), folding `GcsAttemptError<E>` back to `E` at the boundary so
/// callers see a uniform error type. With no refresher, a `TokenExpired`
/// outcome surfaces as `E::from(GcsRequestError::TokenExpired)` — identical
/// to today's pre-refresher behavior. Mirrors `run_s3_with_sts_refresh`.
///
/// `initial_stage_info` seeds the snapshot handed to the first `attempt`
/// invocation in the no-refresher branch, so the legacy path keeps reading
/// the caller's original creds and presigned_url.
async fn run_gcs_with_token_refresh<'r, 'd, F, Fut, T, E>(
    refresher: Option<&'r mut (dyn StageInfoRefresher + 'd)>,
    initial_stage_info: &StageInfo,
    map_refresh_err: impl Fn(StageInfoRefreshError) -> E + Send,
    attempt: F,
) -> Result<T, E>
where
    'd: 'r,
    F: Fn(super::types::StageInfoSnapshot) -> Fut,
    Fut: Future<Output = Result<T, GcsAttemptError<E>>>,
    E: Send + From<GcsRequestError>,
{
    let outcome = match refresher {
        Some(r) => {
            let mut token_refresher = GcsTokenRefresher::new(r, map_refresh_err);
            execute_with_refresh(&mut token_refresher, attempt).await
        }
        None => {
            // No refresher: a `TokenExpired` from the attempt has no
            // recovery path; surface it as today's `TokenExpired` (preserved
            // by the post-loop mapper below). Seed the snapshot from the
            // caller's stage_info so `with_snapshot` overlay is a no-op on
            // the legacy path.
            let snapshot = super::types::StageInfoSnapshot {
                creds: initial_stage_info.creds.clone(),
                presigned_url: initial_stage_info.presigned_url.clone(),
                presigned_urls: None,
            };
            attempt(snapshot).await
        }
    };
    outcome.map_err(|e| match e {
        GcsAttemptError::Other(err) => err,
        GcsAttemptError::TokenExpired => E::from(GcsRequestError::TokenExpired),
    })
}

// --- Error types ---

/// Internal error for shared helpers (retry, client creation, URL resolution,
/// upload-time metadata serialization). Converted into `GcsUploadError` or
/// `GcsDownloadError` via `From` impls.
#[derive(Debug, Snafu)]
enum GcsRequestError {
    #[snafu(display("Failed to read upload source data"))]
    SourceIo { source: std::io::Error },
    #[snafu(display("GCS HTTP error"))]
    Http { source: reqwest::Error },
    #[snafu(display("GCS request failed: HTTP {status_code}: {body}"))]
    GcsHttp { status_code: u16, body: String },
    #[snafu(display("GCS access token expired"))]
    TokenExpired,
    #[snafu(display("GCS presigned URL expired"))]
    PresignedUrlExpired,
    #[snafu(display("Missing GCS credentials"))]
    MissingGcsCredentials,
    #[snafu(display("GCS retry exhausted: {detail}"))]
    RetryExhausted { detail: String },
    #[snafu(display("GCS client setup failed: {detail}"))]
    ClientSetup { detail: String },
    #[snafu(display("Failed to serialize GCS metadata"))]
    Serialization { source: serde_json::Error },
    #[snafu(display("Object too large to upload to GCS: {detail}"))]
    FileTooLarge { detail: String },
    #[snafu(display("GCS resumable upload protocol error: {detail}"))]
    Resumable { detail: String },
    #[snafu(display("Failed to stage GCS ranged download to a temp file: {detail}"))]
    TempFile { detail: String },
    #[snafu(display("GCS endpoint did not honor Range header: {detail}"))]
    RangeNotHonored { detail: String },
}

impl From<GcsRequestError> for GcsUploadError {
    fn from(e: GcsRequestError) -> Self {
        match e {
            GcsRequestError::SourceIo { source } => {
                gcs_upload_error::SourceIoSnafu.into_error(source)
            }
            GcsRequestError::Http { source } => gcs_upload_error::HttpSnafu.into_error(source),
            GcsRequestError::GcsHttp { status_code, body } => {
                gcs_upload_error::GcsHttpSnafu { status_code, body }.build()
            }
            GcsRequestError::TokenExpired => gcs_upload_error::TokenExpiredSnafu.build(),
            GcsRequestError::PresignedUrlExpired => {
                gcs_upload_error::PresignedUrlExpiredSnafu.build()
            }
            GcsRequestError::MissingGcsCredentials => {
                gcs_upload_error::MissingGcsCredentialsSnafu.build()
            }
            GcsRequestError::RetryExhausted { detail } => {
                gcs_upload_error::RetryExhaustedSnafu { detail }.build()
            }
            GcsRequestError::ClientSetup { detail } => {
                gcs_upload_error::ClientSetupFailedSnafu { detail }.build()
            }
            GcsRequestError::Serialization { source } => {
                gcs_upload_error::SerializationSnafu.into_error(source)
            }
            GcsRequestError::FileTooLarge { detail } => {
                gcs_upload_error::FileTooLargeSnafu { detail }.build()
            }
            GcsRequestError::Resumable { detail } => {
                gcs_upload_error::ResumableSnafu { detail }.build()
            }
            // TempFile is download-only; map to a generic upload error so the
            // conversion stays total (it cannot actually occur on upload).
            GcsRequestError::TempFile { detail } => {
                gcs_upload_error::RetryExhaustedSnafu { detail }.build()
            }
            // RangeNotHonored is download-only; map to a generic upload error so
            // the conversion stays total (it cannot actually occur on upload).
            GcsRequestError::RangeNotHonored { detail } => {
                gcs_upload_error::RetryExhaustedSnafu { detail }.build()
            }
        }
    }
}

impl From<GcsRequestError> for GcsDownloadError {
    fn from(e: GcsRequestError) -> Self {
        match e {
            // SourceIo is upload-only (reading the PUT body); if it ever fires on
            // the download path it's a logic bug, but we still need a total mapping.
            GcsRequestError::SourceIo { source } => gcs_download_error::RetryExhaustedSnafu {
                detail: format!("unexpected upload-source IO error on download path: {source}"),
            }
            .build(),
            GcsRequestError::Http { source } => gcs_download_error::HttpSnafu.into_error(source),
            GcsRequestError::GcsHttp { status_code, body } => {
                gcs_download_error::GcsHttpSnafu { status_code, body }.build()
            }
            GcsRequestError::TokenExpired => gcs_download_error::TokenExpiredSnafu.build(),
            GcsRequestError::PresignedUrlExpired => {
                gcs_download_error::PresignedUrlExpiredSnafu.build()
            }
            GcsRequestError::MissingGcsCredentials => {
                gcs_download_error::MissingGcsCredentialsSnafu.build()
            }
            GcsRequestError::RetryExhausted { detail } => {
                gcs_download_error::RetryExhaustedSnafu { detail }.build()
            }
            GcsRequestError::ClientSetup { detail } => {
                gcs_download_error::ClientSetupFailedSnafu { detail }.build()
            }
            // Serialization is upload-only; if it ever fires on the download
            // path it's a logic bug, but we still need a total mapping.
            GcsRequestError::Serialization { source } => {
                gcs_download_error::DeserializationSnafu.into_error(source)
            }
            // FileTooLarge can occur on a ranged download (object > 5 TiB);
            // Resumable is upload-only (logic-bug fallback on the download path).
            GcsRequestError::FileTooLarge { detail } => {
                gcs_download_error::FileTooLargeSnafu { detail }.build()
            }
            GcsRequestError::Resumable { detail } => {
                gcs_download_error::RetryExhaustedSnafu { detail }.build()
            }
            GcsRequestError::TempFile { detail } => {
                gcs_download_error::TempFileSnafu { detail }.build()
            }
            GcsRequestError::RangeNotHonored { detail } => {
                gcs_download_error::RangeNotHonoredSnafu { detail }.build()
            }
        }
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum GcsUploadError {
    #[snafu(display("Failed to read upload source data"))]
    SourceIo {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS HTTP error"))]
    Http {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS request failed: HTTP {status_code}: {body}"))]
    GcsHttp {
        status_code: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS access token expired"))]
    TokenExpired {
        #[snafu(implicit)]
        location: Location,
    },
    /// The presigned URL for this file expired and a refresh attempt did not
    /// yield a working replacement (second consecutive 400). Mirrors Python's
    /// two-strike guard in `gcs_storage_client.py`.
    #[snafu(display("GCS presigned URL expired and refresh did not produce a working URL"))]
    PresignedUrlExpired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize GCS metadata"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing GCS credentials"))]
    MissingGcsCredentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS retry exhausted: {detail}"))]
    RetryExhausted {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Object too large to upload to GCS: {detail}"))]
    FileTooLarge {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS resumable upload protocol error: {detail}"))]
    Resumable {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS client setup failed: {detail}"))]
    ClientSetupFailed {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to refresh GCS stage info after recoverable error"))]
    StageInfoRefresh {
        #[snafu(source(from(StageInfoRefreshError, Box::new)))]
        source: Box<StageInfoRefreshError>,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum GcsDownloadError {
    #[snafu(display("GCS HTTP error"))]
    Http {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS request failed: HTTP {status_code}: {body}"))]
    GcsHttp {
        status_code: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS access token expired"))]
    TokenExpired {
        #[snafu(implicit)]
        location: Location,
    },
    /// The presigned URL for this file expired and a refresh attempt did not
    /// yield a working replacement.
    #[snafu(display("GCS presigned URL expired and refresh did not produce a working URL"))]
    PresignedUrlExpired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to deserialize GCS metadata"))]
    Deserialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing GCS metadata: {field}"))]
    MissingMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid GCS header value"))]
    InvalidHeaderValue {
        source: reqwest::header::ToStrError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing GCS credentials"))]
    MissingGcsCredentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS retry exhausted: {detail}"))]
    RetryExhausted {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Object too large to download from GCS: {detail}"))]
    FileTooLarge {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to stage GCS ranged download to a temp file: {detail}"))]
    TempFile {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS client setup failed: {detail}"))]
    ClientSetupFailed {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to refresh GCS stage info after recoverable error"))]
    StageInfoRefresh {
        #[snafu(source(from(StageInfoRefreshError, Box::new)))]
        source: Box<StageInfoRefreshError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "GCS Content-Length mismatch: header announced {expected} bytes, received {actual}"
    ))]
    ContentLengthMismatch {
        expected: u64,
        actual: u64,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS endpoint did not honor Range header: {detail}"))]
    RangeNotHonored {
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
    use crate::file_manager::types::{RefreshFuture, StageInfoCache, StageInfoSnapshot};
    use crate::sensitive::SensitiveString;
    use bytes::Bytes;

    fn base_policy() -> RetryPolicy {
        use crate::config::param_store::ParamStore;
        RetryPolicy::put_get(&ParamStore::new())
    }

    // Zero-backoff test policy lives in `file_manager::internal` so the in-crate
    // and external integration tests share one definition that derives from the
    // production `gcs_retry_policy` (no drift). Aliased so call sites read
    // `test_policy(using_presigned_url, ..)`.
    use crate::file_manager::internal::gcs_test_retry_policy as test_policy;

    fn make_stage_info(overrides: StageInfoOverrides) -> StageInfo {
        StageInfo {
            location_type: super::super::types::LocationType::Gcs,
            bucket: overrides.bucket.unwrap_or("my-bucket".to_string()),
            key_prefix: overrides.key_prefix.unwrap_or("prefix/".to_string()),
            region: overrides.region.unwrap_or("us-central1".to_string()),
            creds: overrides.creds.unwrap_or(CloudCredentials::Gcs {
                gcs_access_token: Some(SensitiveString::from("fake-token")),
            }),
            endpoint: overrides.endpoint,
            presigned_url: overrides.presigned_url,
            use_virtual_url: overrides.use_virtual_url,
            use_regional_url: overrides.use_regional_url,
            use_s3_regional_url: false,
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
            storage_account: None,
        }
    }

    #[derive(Default)]
    struct StageInfoOverrides {
        bucket: Option<String>,
        key_prefix: Option<String>,
        region: Option<String>,
        creds: Option<CloudCredentials>,
        endpoint: Option<String>,
        presigned_url: Option<String>,
        use_virtual_url: bool,
        use_regional_url: bool,
    }

    // ---------------------------------------------------------------
    // 1. URL construction strategies (matches ODBC test_unit_put_get_gcs.cpp)
    // ---------------------------------------------------------------

    #[test]
    fn url_default_strategy() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(url, "https://storage.googleapis.com/my-bucket/file.csv.gz");
    }

    #[test]
    fn url_custom_endpoint() {
        // Matches ODBC test_gcs_override_endpoint
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("testendpoint.googleapis.com".to_string()),
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(
            url,
            "https://testendpoint.googleapis.com/my-bucket/file.csv.gz"
        );
    }

    #[test]
    fn url_custom_endpoint_with_scheme() {
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("https://custom.example.com".to_string()),
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(url, "https://custom.example.com/my-bucket/file.csv.gz");
    }

    #[test]
    fn url_virtual_host() {
        // Matches ODBC test_gcs_use_virtual_url
        let stage = make_stage_info(StageInfoOverrides {
            use_virtual_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(url, "https://my-bucket.storage.googleapis.com/file.csv.gz");
    }

    #[test]
    fn url_regional() {
        // Matches ODBC test_gcs_use_regional_url
        let stage = make_stage_info(StageInfoOverrides {
            region: Some("testregion".to_string()),
            use_regional_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(
            url,
            "https://storage.testregion.rep.googleapis.com/my-bucket/file.csv.gz"
        );
    }

    #[test]
    fn url_me_central2_forces_regional() {
        // Matches ODBC test_gcs_use_me2_region
        // Note: me-central2 forcing is done in query_response.rs TryFrom,
        // so here we just verify the regional URL is built correctly.
        let stage = make_stage_info(StageInfoOverrides {
            region: Some("me-central2".to_string()),
            use_regional_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(
            url,
            "https://storage.me-central2.rep.googleapis.com/my-bucket/file.csv.gz"
        );
    }

    #[test]
    fn url_custom_endpoint_takes_precedence() {
        // Matches ODBC test_gcs_all_endpoint_fields_enabled
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("testendpoint.googleapis.com".to_string()),
            region: Some("testregion".to_string()),
            use_virtual_url: true,
            use_regional_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(
            url,
            "https://testendpoint.googleapis.com/my-bucket/file.csv.gz"
        );
    }

    #[test]
    fn url_empty_endpoint_falls_through() {
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("".to_string()),
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(url, "https://storage.googleapis.com/my-bucket/file.csv.gz");
    }

    // ---------------------------------------------------------------
    // 2. Access token optionality (matches ODBC token vs presigned tests)
    // ---------------------------------------------------------------

    #[test]
    fn resolve_with_bearer_token() {
        // Matches ODBC test_simple_get_gcs_with_token
        let stage = make_stage_info(StageInfoOverrides::default());
        let (url, token) = resolve_url_and_token(&stage, "file.csv.gz", None).unwrap();
        assert_eq!(url, "https://storage.googleapis.com/my-bucket/file.csv.gz");
        assert_eq!(token, Some("fake-token"));
    }

    #[test]
    fn resolve_with_presigned_url() {
        // Matches ODBC test_simple_get_gcs_with_presignedurl. PUT-side
        // single presigned URL slot — preserved as Strategy 1 by step 2.2.
        let stage = make_stage_info(StageInfoOverrides {
            presigned_url: Some("https://faked.presigned.url".to_string()),
            ..Default::default()
        });
        let (url, token) = resolve_url_and_token(&stage, "file.csv.gz", None).unwrap();
        assert_eq!(url, "https://faked.presigned.url");
        assert!(token.is_none(), "presigned URL mode should not use a token");
    }

    #[test]
    fn resolve_per_file_presigned_url_wins_over_stage_info_presigned_url() {
        // Strategy 0 must beat Strategy 1: GS issues `data.presignedUrls[i]`
        // for this specific object on GCS GET, while `stageInfo.presignedUrl`
        // is the PUT-side single slot. See
        // `--gcp--/2.2-server_supplied_presigned_url_list_on_download.md`,
        // §4 "Mixed-mode stages" — matches Python's
        // `meta.presigned_url or stage_info.get("presignedUrl")` ordering in
        // `gcs_storage_client.py:77`.
        let stage = make_stage_info(StageInfoOverrides {
            presigned_url: Some("https://stage-info.presigned.url/put-slot".to_string()),
            ..Default::default()
        });
        let (url, token) = resolve_url_and_token(
            &stage,
            "file.csv.gz",
            Some("https://per-file.presigned.url/get-slot"),
        )
        .unwrap();
        assert_eq!(url, "https://per-file.presigned.url/get-slot");
        assert!(
            token.is_none(),
            "per-file presigned URL mode must not return a token"
        );
    }

    #[test]
    fn resolve_per_file_presigned_url_wins_over_bearer_token() {
        // Mixed mode: GS sometimes emits both `presignedUrls[]` and a token
        // during stage transitions. Per-file URL must still win — the URL is
        // object-scoped and the token is generic.
        let stage = make_stage_info(StageInfoOverrides::default());
        let (url, token) = resolve_url_and_token(
            &stage,
            "file.csv.gz",
            Some("https://per-file.presigned.url/get-slot"),
        )
        .unwrap();
        assert_eq!(url, "https://per-file.presigned.url/get-slot");
        assert!(
            token.is_none(),
            "per-file presigned URL mode must not return a token even when one is available"
        );
    }

    #[test]
    fn resolve_falls_back_to_stage_info_presigned_url_when_per_file_is_none() {
        // PUT path semantics must not regress: when no per-file URL is
        // supplied, `stage_info.presigned_url` is still honoured (Strategy
        // 1 — the original PUT-side single-slot path).
        let stage = make_stage_info(StageInfoOverrides {
            presigned_url: Some("https://stage-info.presigned.url/put-slot".to_string()),
            ..Default::default()
        });
        let (url, token) = resolve_url_and_token(&stage, "file.csv.gz", None).unwrap();
        assert_eq!(url, "https://stage-info.presigned.url/put-slot");
        assert!(token.is_none());
    }

    #[test]
    fn resolve_with_no_token_and_no_presigned_url_returns_error() {
        // When GCS_ACCESS_TOKEN is absent and no presigned URL, should error
        let stage = make_stage_info(StageInfoOverrides {
            creds: Some(CloudCredentials::Gcs {
                gcs_access_token: None,
            }),
            ..Default::default()
        });
        let result = resolve_url_and_token(&stage, "file.csv.gz", None);
        assert!(matches!(
            result,
            Err(GcsRequestError::MissingGcsCredentials)
        ));
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
        let result = resolve_url_and_token(&stage, "file.csv.gz", None);
        assert!(matches!(
            result,
            Err(GcsRequestError::MissingGcsCredentials)
        ));
    }

    // ---------------------------------------------------------------
    // 3. Retry policy configuration
    // ---------------------------------------------------------------

    #[test]
    fn gcs_retry_policy_includes_403() {
        let policy = gcs_retry_policy(false, &base_policy());
        assert!(
            policy.extra_retryable_statuses.contains(&403),
            "403 should be retryable for GCS (matches JDBC/ODBC)"
        );
    }

    #[test]
    fn gcs_retry_policy_includes_400_for_presigned_urls() {
        let policy = gcs_retry_policy(true, &base_policy());
        assert!(
            policy.extra_retryable_statuses.contains(&400),
            "400 should be retryable when using presigned URLs"
        );
    }

    #[test]
    fn gcs_retry_policy_excludes_400_without_presigned_urls() {
        let policy = gcs_retry_policy(false, &base_policy());
        assert!(
            !policy.extra_retryable_statuses.contains(&400),
            "400 should not be retryable without presigned URLs"
        );
    }

    #[test]
    fn gcs_retry_policy_preserves_user_configured_status_codes() {
        use crate::config::param_registry::param_names;
        use crate::config::param_store::ParamStore;
        use crate::config::settings::Setting;

        // A user-configured extra status code (via `retry_extra_status_codes`)
        // must survive the GCS-specific additions rather than being replaced.
        let mut params = ParamStore::new();
        params.insert(
            param_names::RETRY_EXTRA_STATUS_CODES.as_str().to_string(),
            Setting::String("404".to_string()),
        );
        let policy = gcs_retry_policy(true, &RetryPolicy::put_get(&params));

        assert!(
            policy.extra_retryable_statuses.contains(&404),
            "user-configured 404 should survive GCS policy construction"
        );
        assert!(
            policy.extra_retryable_statuses.contains(&403),
            "GCS should still add 403 on top of user-configured codes"
        );
    }

    // ---------------------------------------------------------------
    // 4. URL percent-encoding
    // ---------------------------------------------------------------

    #[test]
    fn percent_encode_preserves_normal_paths() {
        assert_eq!(
            percent_encode_path("prefix/file.csv.gz"),
            "prefix/file.csv.gz"
        );
    }

    #[test]
    fn percent_encode_encodes_spaces_and_special_chars() {
        assert_eq!(percent_encode_path("dir/my file.csv"), "dir/my%20file.csv");
        assert_eq!(percent_encode_path("path/a+b=c"), "path/a%2Bb%3Dc");
    }

    // ---------------------------------------------------------------
    // 5. Upload status enum
    // ---------------------------------------------------------------

    #[test]
    fn upload_status_display() {
        assert_eq!(UploadStatus::Uploaded.to_string(), "UPLOADED");
        assert_eq!(UploadStatus::Skipped.to_string(), "SKIPPED");
    }

    // ---------------------------------------------------------------
    // 6. Retry policy budget
    // ---------------------------------------------------------------

    #[test]
    fn gcs_retry_policy_max_elapsed_exceeds_request_timeout() {
        let policy = gcs_retry_policy(false, &base_policy());
        assert_eq!(
            policy.max_elapsed,
            Some(Duration::from_secs(600)),
            "max_elapsed must exceed REQUEST_TIMEOUT_SECS (300s)"
        );
        assert!(
            policy.max_elapsed > Some(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
            "retry budget must be larger than a single request timeout"
        );
    }

    #[test]
    fn gcs_retry_policy_max_attempts() {
        let mut base = base_policy();
        base.max_attempts = 25;
        assert_eq!(gcs_retry_policy(false, &base).max_attempts, 25);
        base.max_attempts = 1;
        assert_eq!(gcs_retry_policy(false, &base).max_attempts, 1);
    }

    #[test]
    fn gcs_retry_policy_backoff_bounds() {
        let p = gcs_retry_policy(false, &base_policy());
        assert_eq!(p.backoff.base, Duration::from_millis(250));
        assert_eq!(p.backoff.cap, Duration::from_secs(16));
        assert_eq!(p.backoff.factor, 2.0);
        assert!(matches!(p.backoff.jitter, Jitter::Decorrelated));
    }

    #[test]
    fn without_400_drops_400_and_keeps_403() {
        let p = without_400(&gcs_retry_policy(true, &base_policy()));
        assert!(!p.extra_retryable_statuses.contains(&400));
        assert!(p.extra_retryable_statuses.contains(&403));
    }

    // ---------------------------------------------------------------
    // 7. Percent-encoding edge cases
    // ---------------------------------------------------------------

    #[test]
    fn percent_encode_empty_string() {
        assert_eq!(percent_encode_path(""), "");
    }

    #[test]
    fn percent_encode_unreserved_chars_pass_through() {
        // RFC 3986 unreserved: A-Z a-z 0-9 - _ . ~
        let unreserved = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~/";
        assert_eq!(percent_encode_path(unreserved), unreserved);
    }

    #[test]
    fn percent_encode_special_ascii_chars() {
        assert_eq!(percent_encode_path("@"), "%40");
        assert_eq!(percent_encode_path("#"), "%23");
        assert_eq!(percent_encode_path("!"), "%21");
        assert_eq!(percent_encode_path("$"), "%24");
        assert_eq!(percent_encode_path("&"), "%26");
        assert_eq!(percent_encode_path(" "), "%20");
        assert_eq!(percent_encode_path("%"), "%25");
    }

    #[test]
    fn percent_encode_multibyte_unicode() {
        // é is U+00E9, encoded as 0xC3 0xA9 in UTF-8
        assert_eq!(percent_encode_path("café.csv"), "caf%C3%A9.csv");
        // 日本 is multi-byte CJK
        assert_eq!(
            percent_encode_path("日本/data.csv"),
            "%E6%97%A5%E6%9C%AC/data.csv"
        );
    }

    #[test]
    fn percent_encode_preserves_slashes_in_paths() {
        assert_eq!(percent_encode_path("a/b/c/d.csv"), "a/b/c/d.csv");
    }

    // ---------------------------------------------------------------
    // 8. URL construction with special characters
    // ---------------------------------------------------------------

    #[test]
    fn url_default_encodes_special_chars_in_key() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_gcs_url(&stage, "dir/my file (1).csv");
        assert_eq!(
            url,
            "https://storage.googleapis.com/my-bucket/dir/my%20file%20%281%29.csv"
        );
    }

    #[test]
    fn url_virtual_host_encodes_key() {
        let stage = make_stage_info(StageInfoOverrides {
            use_virtual_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "path/café.csv");
        assert_eq!(
            url,
            "https://my-bucket.storage.googleapis.com/path/caf%C3%A9.csv"
        );
    }

    #[test]
    fn url_regional_encodes_key() {
        let stage = make_stage_info(StageInfoOverrides {
            region: Some("us-east1".to_string()),
            use_regional_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "a&b=c.csv");
        assert_eq!(
            url,
            "https://storage.us-east1.rep.googleapis.com/my-bucket/a%26b%3Dc.csv"
        );
    }

    #[test]
    fn url_custom_endpoint_encodes_key() {
        let stage = make_stage_info(StageInfoOverrides {
            endpoint: Some("custom.example.com".to_string()),
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "dir/file name.csv");
        assert_eq!(
            url,
            "https://custom.example.com/my-bucket/dir/file%20name.csv"
        );
    }

    // ---------------------------------------------------------------
    // 9. try_get_header: missing vs invalid header values
    // ---------------------------------------------------------------

    #[test]
    fn try_get_header_missing_returns_ok_none() {
        let headers = reqwest::header::HeaderMap::new();
        let result = try_get_header(&headers, "x-missing").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn try_get_header_valid_returns_ok_some() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-test", "hello".parse().unwrap());
        let result = try_get_header(&headers, "x-test").unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn try_get_header_invalid_utf8_returns_error() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-bad",
            reqwest::header::HeaderValue::from_bytes(&[0x80, 0x81]).unwrap(),
        );
        let result = try_get_header(&headers, "x-bad");
        assert!(result.is_err(), "non-UTF8 header should produce an error");
        assert!(matches!(
            result.unwrap_err(),
            GcsDownloadError::InvalidHeaderValue { .. }
        ));
    }

    // ---------------------------------------------------------------
    // 10. GCS download metadata extraction
    // ---------------------------------------------------------------

    fn build_gcs_download_headers(
        encryption_data: Option<&str>,
        mat_desc: Option<&str>,
        digest: Option<&str>,
    ) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(v) = encryption_data {
            headers.insert(GCS_META_ENCRYPTIONDATA, v.parse().unwrap());
        }
        if let Some(v) = mat_desc {
            headers.insert(GCS_META_MATDESC, v.parse().unwrap());
        }
        if let Some(v) = digest {
            headers.insert(GCS_META_SFC_DIGEST, v.parse().unwrap());
        }
        headers
    }

    const VALID_ENCRYPTION_DATA: &str =
        r#"{"WrappedContentKey":{"EncryptedKey":"dGVzdA=="},"ContentEncryptionIV":"aXYxMjM0NTY="}"#;
    const VALID_MAT_DESC: &str = r#"{"smkId":"1","queryId":"qid","keySize":"128"}"#;

    #[test]
    fn gcs_metadata_sse_no_headers_returns_none() {
        let headers = build_gcs_download_headers(None, None, None);
        let digest = try_get_header(&headers, GCS_META_SFC_DIGEST).unwrap();
        let file_metadata = try_get_header(&headers, GCS_META_ENCRYPTIONDATA).unwrap();
        assert!(digest.is_none());
        assert!(file_metadata.is_none());
    }

    #[test]
    fn gcs_metadata_encrypted_all_headers_returns_metadata() {
        let headers = build_gcs_download_headers(
            Some(VALID_ENCRYPTION_DATA),
            Some(VALID_MAT_DESC),
            Some("sha256digest"),
        );

        let digest = try_get_header(&headers, GCS_META_SFC_DIGEST).unwrap();
        assert_eq!(digest, Some("sha256digest".to_string()));

        let enc_data_str = try_get_header(&headers, GCS_META_ENCRYPTIONDATA)
            .unwrap()
            .unwrap();
        let enc_data: serde_json::Value = serde_json::from_str(&enc_data_str).unwrap();

        let encrypted_key = enc_data["WrappedContentKey"]["EncryptedKey"]
            .as_str()
            .unwrap();
        assert_eq!(encrypted_key, "dGVzdA==");

        let iv = enc_data["ContentEncryptionIV"].as_str().unwrap();
        assert_eq!(iv, "aXYxMjM0NTY=");

        let mat_desc_str = try_get_header(&headers, GCS_META_MATDESC).unwrap().unwrap();
        let material_desc: MaterialDescription = serde_json::from_str(&mat_desc_str).unwrap();
        assert_eq!(material_desc.smk_id, "1");
    }

    #[test]
    fn gcs_metadata_encryptiondata_present_but_matdesc_missing_errors_in_download() {
        let headers = build_gcs_download_headers(Some(VALID_ENCRYPTION_DATA), None, Some("digest"));

        let enc_data_str = try_get_header(&headers, GCS_META_ENCRYPTIONDATA)
            .unwrap()
            .unwrap();
        assert!(!enc_data_str.is_empty());

        let mat_desc_result: Result<Option<String>, _> = try_get_header(&headers, GCS_META_MATDESC);
        assert!(
            mat_desc_result.unwrap().is_none(),
            "matdesc should be None when header is absent"
        );
    }

    #[test]
    fn gcs_metadata_malformed_encryptiondata_returns_deserialization_error() {
        let headers =
            build_gcs_download_headers(Some("not-valid-json"), Some(VALID_MAT_DESC), None);

        let enc_data_str = try_get_header(&headers, GCS_META_ENCRYPTIONDATA)
            .unwrap()
            .unwrap();
        let parse_result: Result<serde_json::Value, _> = serde_json::from_str(&enc_data_str);
        assert!(
            parse_result.is_err(),
            "malformed JSON should fail deserialization"
        );
    }

    // ---------------------------------------------------------------
    // 11. GcsTokenRefresher::should_refresh
    // Direct port of s3_transfer.rs:expired_token_code_is_detected /
    // other_aws_codes_are_not_treated_as_expired_token.
    // Guards that 400 and 403 (handled separately) do NOT trigger the
    // cred-rotation loop, only 401 (TokenExpired) does.
    // ---------------------------------------------------------------

    struct NoopRefresher {
        cache: StageInfoCache,
    }

    impl NoopRefresher {
        fn with_token(token: &str) -> Self {
            Self {
                cache: StageInfoCache::new(StageInfoSnapshot {
                    creds: CloudCredentials::Gcs {
                        gcs_access_token: Some(SensitiveString::from(token)),
                    },
                    presigned_url: None,
                    presigned_urls: None,
                }),
            }
        }
    }

    impl super::super::types::StageInfoRefresher for NoopRefresher {
        fn refresh(&mut self) -> super::super::types::RefreshFuture<'_> {
            Box::pin(async { Ok(()) })
        }

        fn refresh_url(&mut self) -> RefreshFuture<'_> {
            Box::pin(async { Ok(()) })
        }

        fn cache(&self) -> &StageInfoCache {
            &self.cache
        }
    }

    #[test]
    fn gcs_token_refresher_should_refresh_matches_only_token_expired() {
        // The GcsTokenRefresher must treat only 401 (TokenExpired) as a
        // cred-rotation signal. 400 (presigned URL expiry, handled separately
        // by the outer 400-refresh loop) and 403 (access denied, not
        // recoverable by cred rotation) must not trigger the creds refresh
        // loop.
        let mut noop = NoopRefresher::with_token("tok");
        // Use unit type as E to isolate the should_refresh logic from error
        // conversions — the check dispatches only on the enum variant.
        let r: GcsTokenRefresher<'_, (), _> = GcsTokenRefresher::new(&mut noop, |_| ());

        assert!(
            r.should_refresh(&GcsAttemptError::TokenExpired),
            "TokenExpired (401) must trigger cred refresh"
        );
        assert!(
            !r.should_refresh(&GcsAttemptError::Other(())),
            "Other (400, 403, …) must NOT trigger cred refresh"
        );
    }

    // ---------------------------------------------------------------
    // `check_file_exists_gcs` — HEAD result + sfc-digest extraction
    //
    // Parity with Python connector `gcs_storage_client.get_file_header`
    // at `gcs_storage_client.py:338-419` and the skip block at
    // `storage_client.py:213-220`.
    // ---------------------------------------------------------------

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    /// Builds a `StageInfo` whose URL strategy routes through the given
    /// custom endpoint (i.e. the wiremock server URI). Uses bearer-token
    /// auth so the HEAD path matches the production code paths exactly
    /// — the presigned-URL path is a peer, not a substitute (HEAD on a
    /// PUT-only presigned URL is typically rejected by real GCS so the
    /// existence-check is a no-op in that mode).
    fn make_stage_for_mock(endpoint: &str) -> StageInfo {
        make_stage_info(StageInfoOverrides {
            endpoint: Some(endpoint.to_string()),
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn check_file_exists_gcs_returns_exists_with_digest_on_200_with_header() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(ResponseTemplate::new(200).insert_header(GCS_META_SFC_DIGEST, "dGVzdA=="))
            .mount(&server)
            .await;

        let client = create_gcs_client(&make_stage_for_mock(&server.uri())).unwrap();
        let url = format!("{}/my-bucket/prefix/file.csv", server.uri());
        let result = check_file_exists_gcs(&client, &url, Some("token")).await;

        assert_eq!(
            result,
            GcsHeadResult::Found {
                digest: Some("dGVzdA==".to_string()),
            }
        );
    }

    #[tokio::test]
    async fn check_file_exists_gcs_returns_exists_no_digest_on_200_without_header() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = create_gcs_client(&make_stage_for_mock(&server.uri())).unwrap();
        let url = format!("{}/my-bucket/prefix/file.csv", server.uri());
        let result = check_file_exists_gcs(&client, &url, Some("token")).await;

        // Older objects (pre-`sfc-digest`-write era, libsfclient-S3-style
        // uploads, etc.) lack the header; the conservative fall-through
        // is `Found { digest: None }` so the digest comparison misses
        // and the upload proceeds. Matches Python
        // `meta.sha256_digest == file_header.digest` evaluating to
        // `Some(...) == None == false`.
        assert_eq!(result, GcsHeadResult::Found { digest: None });
    }

    #[tokio::test]
    async fn check_file_exists_gcs_returns_default_on_404() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = create_gcs_client(&make_stage_for_mock(&server.uri())).unwrap();
        let url = format!("{}/my-bucket/prefix/file.csv", server.uri());
        let result = check_file_exists_gcs(&client, &url, Some("token")).await;

        assert_eq!(result, GcsHeadResult::NotFound);
    }

    #[tokio::test]
    async fn check_file_exists_gcs_returns_default_on_403() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client = create_gcs_client(&make_stage_for_mock(&server.uri())).unwrap();
        let url = format!("{}/my-bucket/prefix/file.csv", server.uri());
        let result = check_file_exists_gcs(&client, &url, Some("token")).await;

        // 403 indicates limited credentials (e.g. PUT-only); proceed
        // with upload rather than surface a hard error — the worst
        // case is one wasted PUT that GCS would also reject.
        assert_eq!(result, GcsHeadResult::NotFound);
    }

    #[tokio::test]
    async fn check_file_exists_gcs_returns_default_on_unexpected_status() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = create_gcs_client(&make_stage_for_mock(&server.uri())).unwrap();
        let url = format!("{}/my-bucket/prefix/file.csv", server.uri());
        let result = check_file_exists_gcs(&client, &url, Some("token")).await;

        assert_eq!(result, GcsHeadResult::NotFound);
    }

    #[tokio::test]
    async fn check_file_exists_gcs_drops_non_utf8_digest_header_silently() {
        // A non-UTF8 sfc-digest header must NOT poison the upload — we
        // surface `exists=true, digest=None` so the comparison misses
        // and the upload proceeds. Locks in the "never error out on a
        // malformed header" promise documented on `check_file_exists_gcs`.
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(ResponseTemplate::new(200).insert_header(
                GCS_META_SFC_DIGEST,
                reqwest::header::HeaderValue::from_bytes(&[0x80, 0x81]).unwrap(),
            ))
            .mount(&server)
            .await;

        let client = create_gcs_client(&make_stage_for_mock(&server.uri())).unwrap();
        let url = format!("{}/my-bucket/prefix/file.csv", server.uri());
        let result = check_file_exists_gcs(&client, &url, Some("token")).await;

        assert_eq!(result, GcsHeadResult::Found { digest: None });
    }

    // ---------------------------------------------------------------
    // `upload_to_gcs_or_skip` — digest-based skip-on-content-match
    //
    // Mirrors Python connector `storage_client.py:213-220` order:
    //   1. existence skip (gated on `!overwrite`)
    //   2. digest skip (fires even when `overwrite=true`)
    //   3. PUT
    //
    // Each test mounts a `HEAD` mock with a configurable response and a
    // `PUT` mock with `.expect(0)` or `.expect(1)` to assert the skip
    // path was (or wasn't) taken without relying on side effects.
    // ---------------------------------------------------------------

    /// Constructs a SSE-shaped `PreparedUpload` whose `digest` field is
    /// the caller-supplied marker string. The actual `data` bytes are
    /// irrelevant — the skip branch never gets to PUT them.
    fn make_prepared_for_skip(digest: &str) -> PreparedUpload {
        PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(Bytes::from_static(
                b"payload-bytes",
            )),
            digest: digest.to_string(),
            cse: None,
        }
    }

    /// Constructs a CSE-shaped `PreparedUpload` — the only structural
    /// difference is that `cse` is `Some(_)`. The `digest` field drives the
    /// skip comparison; for both SSE and CSE it is the SHA-256 of the
    /// plaintext (see `encryption.rs`), so the skip fires whenever the remote
    /// digest matches.
    fn make_prepared_cse_for_skip(digest: &str) -> PreparedUpload {
        // The skip branch returns before any body is built, so the bytes are
        // never encrypted — but `CseParams` couples the cloud metadata with a
        // real encryptor, so build both from test material.
        let material = crate::file_manager::types::EncryptionMaterial {
            query_stage_master_key: SensitiveString::from(base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                [0u8; 32],
            )),
            query_id: "qid".to_string(),
            smk_id: "1".to_string(),
        };
        let data = Bytes::from_static(b"would-be-ciphertext-bytes");
        let (encryptor, metadata) =
            super::super::encryption::build_encryptor(&material, data.len() as i64).unwrap();
        PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(data),
            digest: digest.to_string(),
            cse: Some(crate::file_manager::types::CseParams {
                metadata,
                encryptor,
            }),
        }
    }

    /// Mount a HEAD responder and a PUT responder with a usage
    /// expectation. Combined call form keeps each test focused on the
    /// behaviour it asserts.
    async fn mount_head_and_put(
        server: &MockServer,
        head_response: ResponseTemplate,
        expected_puts: u64,
    ) {
        Mock::given(method("HEAD"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(head_response)
            .mount(server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/my-bucket/prefix/file.csv"))
            .respond_with(ResponseTemplate::new(200))
            .expect(expected_puts)
            .mount(server)
            .await;
    }

    /// `MultipartParams` with a 1-byte threshold so any non-empty body takes the
    /// resumable/ranged multipart path, at the resolved concurrency.
    fn always_multipart() -> MultipartParams {
        MultipartParams {
            threshold: super::super::multipart::MultipartThreshold::from_server(Some(1)),
            concurrency: 4,
        }
    }

    /// Above the threshold on the access-token path, the upload takes the XML
    /// resumable path: one initiation `POST` (carrying the digest) mints a
    /// session URL, then the body is `PUT` to that session in `Content-Range`
    /// chunks — `308` between chunks, `200` on the last. A 9 MiB body at the
    /// 8 MiB GCS chunk size is two chunks.
    #[tokio::test(flavor = "multi_thread")]
    async fn gcs_resumable_upload_initiates_then_puts_chunks() {
        let server = MockServer::start().await;

        // Existence-check HEAD: 404 so the upload proceeds.
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        // Initiation POST → 201 with the session URL in `Location`.
        let session_path = "/resumable-session/abc";
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(201).insert_header(
                "location",
                format!("{}{session_path}", server.uri()).as_str(),
            ))
            .expect(1)
            .mount(&server)
            .await;
        // Chunk PUTs against the session URL: first 308, then 200.
        let counter = Arc::new(AtomicU64::new(0));
        Mock::given(method("PUT"))
            .and(path(session_path))
            .respond_with(move |_req: &Request| {
                if counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) == 0 {
                    ResponseTemplate::new(308).insert_header("Range", "bytes=0-8388607")
                } else {
                    ResponseTemplate::new(200)
                }
            })
            .mount(&server)
            .await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(Bytes::from(vec![
                7u8;
                9 << 20
            ])),
            digest: "0".repeat(64),
            cse: None,
        };

        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            true,
            always_multipart(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .expect("resumable upload should succeed against the mock");
        assert_eq!(status, UploadStatus::Uploaded);

        let received = server.received_requests().await.unwrap();
        let init = received
            .iter()
            .find(|r| r.method.as_str() == "POST")
            .expect("an initiation POST must be sent");
        assert!(
            init.headers.get("x-goog-resumable").is_some(),
            "initiation POST must carry x-goog-resumable: start"
        );
        assert!(
            init.headers.get(GCS_META_SFC_DIGEST).is_some(),
            "digest metadata must ride on the initiation POST"
        );
        let chunk_puts: Vec<_> = received
            .iter()
            .filter(|r| r.method.as_str() == "PUT")
            .collect();
        assert_eq!(chunk_puts.len(), 2, "9 MiB / 8 MiB chunk = 2 chunk PUTs");
        for put in &chunk_puts {
            assert!(
                put.headers.get(reqwest::header::CONTENT_RANGE).is_some(),
                "every chunk PUT must carry a Content-Range header"
            );
        }
    }

    /// A failed chunk PUT triggers a best-effort `DELETE` of the session URL.
    #[tokio::test(flavor = "multi_thread")]
    async fn gcs_resumable_upload_deletes_session_on_chunk_failure() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let session_path = "/resumable-session/xyz";
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(201).insert_header(
                "location",
                format!("{}{session_path}", server.uri()).as_str(),
            ))
            .mount(&server)
            .await;
        // Chunk PUT fails hard.
        Mock::given(method("PUT"))
            .and(path(session_path))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        // The cleanup DELETE against the session URL.
        Mock::given(method("DELETE"))
            .and(path(session_path))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(Bytes::from(vec![
                1u8;
                1 << 20
            ])),
            digest: "0".repeat(64),
            cse: None,
        };

        // max_attempts=1 so the 500 fails fast (no long retry/backoff).
        let result = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            true,
            always_multipart(),
            &test_policy(false, 1),
            &mut None,
        )
        .await;
        assert!(
            result.is_err(),
            "a 500 on the chunk PUT must fail the upload"
        );
        // The `.expect(1)` on the DELETE mock verifies cleanup fired on drop.
    }

    /// Above the threshold on the access-token path, the download HEADs for size
    /// then fetches via ranged GETs into a tempfile, reassembled byte-for-byte.
    #[tokio::test(flavor = "multi_thread")]
    async fn gcs_ranged_download_reassembles_object() {
        use std::io::Read as _;

        let payload = b"hello ranged gcs object world".to_vec();
        let server = MockServer::start().await;
        // HEAD reports the size via Content-Length (driven by the body length).
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; payload.len()]))
            .mount(&server)
            .await;
        // The single range returns the whole payload (206).
        let body = payload.clone();
        Mock::given(method("GET"))
            .respond_with(move |_: &Request| {
                ResponseTemplate::new(206).set_body_bytes(body.clone())
            })
            .mount(&server)
            .await;

        let stage = make_stage_for_mock(&server.uri());
        let spill = tempfile::tempdir().unwrap();
        let dl = download_from_gcs_streaming(
            &stage,
            "file.csv",
            None,
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            0,
            always_multipart(),
            &mut None,
            cloud_http::CloudSpillTarget::Temp(spill.path()),
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
        assert_eq!(got, payload, "reassembled object must match");
    }

    /// A non-encrypted ranged GCS download assembles straight into the caller's
    /// `.part` file, which the caller renames to the destination on success.
    #[tokio::test(flavor = "multi_thread")]
    async fn gcs_ranged_download_assembles_into_part_file() {
        let payload = b"gcs ranged straight into dot part".to_vec();
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; payload.len()]))
            .mount(&server)
            .await;
        let body = payload.clone();
        Mock::given(method("GET"))
            .respond_with(move |_: &Request| {
                ResponseTemplate::new(206).set_body_bytes(body.clone())
            })
            .mount(&server)
            .await;

        let stage = make_stage_for_mock(&server.uri());
        let dir = tempfile::tempdir().unwrap();
        let part_path = dir.path().join("out.dat.part");
        let dl = download_from_gcs_streaming(
            &stage,
            "file.csv",
            None,
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            0,
            always_multipart(),
            &mut None,
            cloud_http::CloudSpillTarget::Part(&part_path),
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
            }
            _ => panic!("a non-encrypted ranged download must assemble into `.part`"),
        }
    }

    /// A failed ranged GCS download drains its in-flight writes and removes the
    /// `.part`, so a failure never leaves a partial file behind.
    #[tokio::test(flavor = "multi_thread")]
    async fn gcs_ranged_download_failure_removes_part_file() {
        let server = MockServer::start().await;
        // HEAD advertises 32 bytes, but every ranged GET returns a 4-byte body,
        // tripping the range-length guard and failing the download.
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; 32]))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(vec![0u8; 4]))
            .mount(&server)
            .await;

        let stage = make_stage_for_mock(&server.uri());
        let dir = tempfile::tempdir().unwrap();
        let part_path = dir.path().join("out.dat.part");
        let result = download_from_gcs_streaming(
            &stage,
            "file.csv",
            None,
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            0,
            always_multipart(),
            &mut None,
            cloud_http::CloudSpillTarget::Part(&part_path),
        )
        .await;

        assert!(result.is_err(), "a short ranged GET must fail the download");
        assert!(
            !part_path.exists(),
            "a failed ranged download must not leave a `.part` file behind"
        );
    }

    #[tokio::test]
    async fn upload_to_gcs_or_skip_skips_when_digest_matches_under_overwrite_true() {
        // Python parity: `storage_client.py:214-220` — content-match
        // skip fires regardless of the `overwrite` flag (the existence
        // skip above it is gated on `!overwrite`; this branch is not).
        let server = MockServer::start().await;
        let digest = "ZGlnZXN0Lw==";
        mount_head_and_put(
            &server,
            ResponseTemplate::new(200).insert_header(GCS_META_SFC_DIGEST, digest),
            0,
        )
        .await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = make_prepared_for_skip(digest);
        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            true,
            MultipartParams::default(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .unwrap();

        assert_eq!(status, UploadStatus::Skipped);
    }

    #[tokio::test]
    async fn upload_to_gcs_or_skip_skips_when_digest_matches_under_overwrite_false() {
        // Edge: when both the existence skip and the digest skip would
        // fire, the existence skip short-circuits first (cheaper, no
        // header parsing). Either way the outcome is `Skipped` and no
        // PUT is issued — `expect(0)` guards both.
        let server = MockServer::start().await;
        let digest = "ZGlnZXN0Lw==";
        mount_head_and_put(
            &server,
            ResponseTemplate::new(200).insert_header(GCS_META_SFC_DIGEST, digest),
            0,
        )
        .await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = make_prepared_for_skip(digest);
        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            false,
            MultipartParams::default(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .unwrap();

        assert_eq!(status, UploadStatus::Skipped);
    }

    #[tokio::test]
    async fn upload_to_gcs_or_skip_uploads_when_digest_mismatches_under_overwrite_true() {
        let server = MockServer::start().await;
        mount_head_and_put(
            &server,
            ResponseTemplate::new(200).insert_header(GCS_META_SFC_DIGEST, "remote-digest-differs"),
            1,
        )
        .await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = make_prepared_for_skip("local-digest-value");
        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            true,
            MultipartParams::default(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .unwrap();

        assert_eq!(status, UploadStatus::Uploaded);
    }

    #[tokio::test]
    async fn upload_to_gcs_or_skip_uploads_when_remote_digest_missing_under_overwrite_true() {
        // Python parity for older objects without the `sfc-digest`
        // header: `meta.sha256_digest == file_header.digest` evaluates
        // to `Some(_) == None == false`, so the skip does not fire and
        // the upload proceeds.
        let server = MockServer::start().await;
        mount_head_and_put(&server, ResponseTemplate::new(200), 1).await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = make_prepared_for_skip("local-digest-value");
        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            true,
            MultipartParams::default(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .unwrap();

        assert_eq!(status, UploadStatus::Uploaded);
    }

    #[tokio::test]
    async fn upload_to_gcs_or_skip_skips_existence_when_overwrite_false_and_remote_digest_missing()
    {
        // A remote object without a digest header must still trigger the
        // existence-skip when `overwrite=false`. Locks in that the digest
        // branch does not displace the existence branch.
        let server = MockServer::start().await;
        mount_head_and_put(&server, ResponseTemplate::new(200), 0).await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = make_prepared_for_skip("local-digest-value");
        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            false,
            MultipartParams::default(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .unwrap();

        assert_eq!(status, UploadStatus::Skipped);
    }

    #[tokio::test]
    async fn upload_to_gcs_or_skip_uploads_on_404_under_overwrite_false() {
        let server = MockServer::start().await;
        mount_head_and_put(&server, ResponseTemplate::new(404), 1).await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = make_prepared_for_skip("local-digest-value");
        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            false,
            MultipartParams::default(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .unwrap();

        assert_eq!(status, UploadStatus::Uploaded);
    }

    #[tokio::test]
    async fn upload_to_gcs_or_skip_digest_skip_fires_for_cse_when_digests_match() {
        // CSE digest now hashes the plaintext (see `encryption.rs`), so it
        // is stable across uploads and cross-driver interoperable. When the
        // remote `sfc-digest` matches the local plaintext digest, the skip
        // fires even for a CSE object under `OVERWRITE=TRUE` — no PUT.
        let server = MockServer::start().await;
        let digest = "plaintext-sha256";
        mount_head_and_put(
            &server,
            ResponseTemplate::new(200).insert_header(GCS_META_SFC_DIGEST, digest),
            0,
        )
        .await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = make_prepared_cse_for_skip(digest);
        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            true,
            MultipartParams::default(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .unwrap();

        assert_eq!(status, UploadStatus::Skipped);
    }

    #[tokio::test]
    async fn upload_to_gcs_or_skip_uploads_for_cse_when_digests_differ() {
        // Different remote content => the plaintext digests differ, so the
        // skip does not fire and the CSE object is re-uploaded.
        let server = MockServer::start().await;
        mount_head_and_put(
            &server,
            ResponseTemplate::new(200)
                .insert_header(GCS_META_SFC_DIGEST, "remote-plaintext-sha256"),
            1,
        )
        .await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = make_prepared_cse_for_skip("local-plaintext-sha256");
        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            true,
            MultipartParams::default(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .unwrap();

        assert_eq!(status, UploadStatus::Uploaded);
    }

    #[tokio::test]
    async fn upload_to_gcs_or_skip_skip_on_empty_file_digest_match() {
        // SHA-256 of the empty byte string in Base64 — the well-known
        // `47DEQpj…` value. The skip branch must treat the empty-file
        // case like any other; both ends produce the same digest, so
        // the skip fires.
        const EMPTY_SHA256_B64: &str = "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=";

        let server = MockServer::start().await;
        mount_head_and_put(
            &server,
            ResponseTemplate::new(200).insert_header(GCS_META_SFC_DIGEST, EMPTY_SHA256_B64),
            0,
        )
        .await;

        let stage = make_stage_for_mock(&server.uri());
        let prepared = PreparedUpload {
            source: crate::file_manager::types::PreparedSource::Bytes(Bytes::new()),
            digest: EMPTY_SHA256_B64.to_string(),
            cse: None,
        };
        let status = upload_to_gcs_or_skip(
            prepared,
            &stage,
            "file.csv",
            true,
            MultipartParams::default(),
            &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
            &mut None,
        )
        .await
        .unwrap();

        assert_eq!(status, UploadStatus::Skipped);
    }
}
