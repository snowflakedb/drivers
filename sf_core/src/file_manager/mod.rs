mod azure_transfer;
mod cloud_http;
mod encryption;
mod gcs_transfer;
mod multipart;
mod s3_transfer;
mod spool;

mod path_expansion;
pub mod types;

#[cfg(any(test, feature = "test-utils"))]
pub mod internal {
    pub use super::azure_transfer::download_from_azure_streaming;
    pub use super::cloud_http::{
        CloudSpillTarget, CloudStreamingDownload, CseDownloadInfo, StreamReader,
    };
    pub use super::encryption::{
        EncryptingReader, EncryptionError, Encryptor, build_encryptor, compute_sha256_digest,
        decrypt_ciphertext_to_writer,
    };
    pub use super::gcs_transfer::download_from_gcs_streaming;
    // Real per-cloud part-size formula, so live/e2e tests derive the expected
    // part/range count from it instead of mirroring constants.
    pub use super::multipart::{MultipartConfig, compute_part_size};
    pub use crate::compression::compress_to_tempfile;

    use super::{
        CloudCredentials, RefreshFuture, StageInfoCache, StageInfoRefreshError, StageInfoRefresher,
        StageInfoSnapshot,
    };
    use crate::utils::sync::MutexRecoverExt;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};

    /// Builds a base put/get retry policy with the given `max_attempts`
    /// (zero backoff for instant test runs).
    fn base_policy(max_attempts: u32) -> crate::config::retry::RetryPolicy {
        use crate::config::retry::{BackoffConfig, Jitter, RetryPolicy};
        use std::time::Duration;
        let mut p = RetryPolicy::put_get(&test_params(max_attempts));
        p.backoff = BackoffConfig {
            base: Duration::ZERO,
            factor: 1.0,
            cap: Duration::ZERO,
            jitter: Jitter::None,
        };
        p
    }

    /// Zero-backoff base put/get policy for Azure transfer tests. The per-attempt
    /// policy is derived from this via `azure_403_fastfail_policy` (which removes
    /// 403 from the retryable set); tests pass this as the transfer `base`.
    pub fn azure_test_retry_policy(max_attempts: u32) -> crate::config::retry::RetryPolicy {
        base_policy(max_attempts)
    }

    /// Zero-backoff variant of the production GCS retry policy, for tests.
    pub fn gcs_test_retry_policy(
        using_presigned_url: bool,
        max_attempts: u32,
    ) -> crate::config::retry::RetryPolicy {
        super::gcs_transfer::gcs_retry_policy(using_presigned_url, &base_policy(max_attempts))
    }

    /// Builds a [`ParamStore`] with only `put_get_max_attempts` set.
    pub fn test_params(max_attempts: u32) -> crate::config::param_store::ParamStore {
        use crate::config::param_registry::param_names;
        use crate::config::settings::Setting;
        let mut params = crate::config::param_store::ParamStore::new();
        params.insert(
            param_names::PUT_GET_MAX_ATTEMPTS.as_str().to_string(),
            Setting::Int(max_attempts as i64),
        );
        params
    }

    /// Configurable in-memory [`StageInfoRefresher`] fake for cloud
    /// file-transfer refresh-on-error tests. Wraps a real [`StageInfoCache`]
    /// (production type), counts `refresh()` calls, optionally rotates the
    /// cached credential on the next refresh, and can be armed to fail.
    ///
    /// `Arc`-backed, so a `clone()` shares the cache and counter across
    /// concurrent tasks — mirroring production's `Arc<RwLock<StageInfoCache>>`.
    /// Shared by the Azure file-transfer + file_manager unit tests. S3
    /// (`s3_transfer.rs`) and GCS (`gcs_retry.rs`) still keep their own richer
    /// fakes (GCS needs a multi-rotation queue + `refresh_url` history); these
    /// should be merged into a common base fake in the future.
    ///
    /// It does NOT model the production coalescing window in
    /// `SnowflakeStageInfoRefresher` (a real `Instant` gate) — unit-test that
    /// against the real type, not this fake.
    #[derive(Clone)]
    pub struct FakeStageInfoRefresher {
        cache: StageInfoCache,
        refresh_calls: Arc<AtomicU32>,
        next_creds: Arc<Mutex<Option<CloudCredentials>>>,
        fail_msg: Arc<Mutex<Option<String>>>,
    }

    impl FakeStageInfoRefresher {
        /// Seeds the fake with `initial` credentials, no pending rotation or failure.
        pub fn new(initial: CloudCredentials) -> Self {
            Self {
                cache: StageInfoCache::new_with_creds(initial),
                refresh_calls: Arc::new(AtomicU32::new(0)),
                next_creds: Arc::new(Mutex::new(None)),
                fail_msg: Arc::new(Mutex::new(None)),
            }
        }

        /// Arms the next `refresh()` to rotate the cache to `creds`. Left
        /// unarmed, `refresh()` stores nothing (simulates a coalesced peer).
        pub fn arm_rotation(&self, creds: CloudCredentials) {
            *self.next_creds.lock_recover() = Some(creds);
        }

        /// Arms the next `refresh()` to fail with `ServerRejected`.
        pub fn arm_failure(&self, msg: &str) {
            *self.fail_msg.lock_recover() = Some(msg.to_string());
        }

        /// Number of times `refresh()` has been invoked.
        pub fn refresh_call_count(&self) -> u32 {
            self.refresh_calls.load(Ordering::SeqCst)
        }
    }

    impl StageInfoRefresher for FakeStageInfoRefresher {
        fn refresh(&self, _observed: std::time::Instant) -> RefreshFuture<'_> {
            let calls = self.refresh_calls.clone();
            let next = self.next_creds.clone();
            let fail = self.fail_msg.clone();
            let cache = self.cache.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                if let Some(msg) = fail.lock_recover().take() {
                    return Err(StageInfoRefreshError::ServerRejected {
                        message: msg,
                        location: snafu::Location::default(),
                    });
                }
                if let Some(c) = next.lock_recover().take() {
                    cache.store(StageInfoSnapshot::creds_only(c));
                    // Real coordinator's contract: return the post-store generation.
                    return Ok(cache.cached_at());
                }
                // Unarmed: return the CURRENT generation, not `observed` — mirrors
                // the coordinator's coalesce fast-path so a concurrent caller sees a
                // peer's rotation (`cached_at() > observed`) instead of stalling.
                Ok(cache.cached_at())
            })
        }

        fn refresh_url(&self, _current_upload_file: Option<&str>) -> RefreshFuture<'_> {
            // Creds live in the cache, not per-file presigned URLs, on the paths
            // these fakes cover; refresh_url is never exercised.
            Box::pin(async { Ok(std::time::Instant::now()) })
        }

        fn cache(&self) -> &StageInfoCache {
            &self.cache
        }
    }
}

pub use self::types::*;
pub use azure_transfer::{AzureDownloadError, AzureUploadError, download_from_azure};
pub use gcs_transfer::{
    GcsDownloadError, GcsUploadError, download_from_gcs, upload_to_gcs_or_skip,
};
pub use multipart::{FileTooLargeError, MultipartParams, MultipartThreshold};
// Mirrors the Azure/GCS `pub use` above: `FileManagerError::S3Upload`/
// `S3Download` carry a `pub source: UploadFileError`/`DownloadFileError`, so
// the type must be reachable for external callers (including tests) to
// pattern-match on it.
pub use s3_transfer::{DownloadFileError, UploadFileError};
pub(crate) use spool::{SPOOL_MEM_THRESHOLD, SpooledBuffer};

use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::apis::operation_ctx::with_cleanup_scope_opt;
use crate::compression::{CompressionError, compress_to_tempfile};
use crate::compression_types::{CompressionType, CompressionTypeError, try_guess_compression_type};
use crate::config::retry::RetryPolicy;
use azure_transfer::{azure_get_streaming, download_from_azure_streaming, upload_to_azure_or_skip};
use cloud_http::{
    CloudDownloadBody, CloudSpillTarget, CloudSpilledBody, CseDownloadInfo,
    spawn_s3_byte_stream_producer,
};
use encryption::{
    EncryptionError, build_encryptor, compute_sha256_digest, decrypt_ciphertext_to_writer,
};
use flate2::write::GzDecoder;
use gcs_transfer::{download_from_gcs_streaming, gcs_get_streaming, gcs_retry_policy};
use path_expansion::{PathExpansionError, expand_filenames};
use s3_transfer::{
    S3Download, S3DownloadBody, S3StreamingDownload, download_from_s3, download_from_s3_streaming,
    upload_to_s3_or_skip,
};
use snafu::{Location, ResultExt, Snafu};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Message string emitted in the PUT result's `message` column when the
/// upload outcome is `Skipped` under `PutGetResultsetFlavor::Odbc`. Mirrors
/// `#define MESSAGE_SKIPPED "File with same name already exists. SKIPPED"`
/// from legacy libsnowflakeclient's `FileTransferExecutionResult.cpp`. The
/// `Python` flavor leaves the `message` column empty for skipped uploads,
/// matching the historical universal-driver behaviour.
const ODBC_PUT_MESSAGE_SKIPPED: &str = "File with same name already exists. SKIPPED";

/// Legacy JDBC 200004 text for the PUT `message` column.
fn jdbc_unsupported_compression_message(type_name: &str) -> String {
    format!("Copy command does not support compression type {type_name}.")
}

/// Result of the pre-upload HEAD probe, in cloud-agnostic terms. Each cloud
/// projects its own HEAD response (or a treated-as-absent error) down to this
/// shape, so the shared skip decision never depends on a cloud SDK type. The
/// two-variant shape also makes the illegal state "absent but has a digest"
/// unrepresentable.
pub(crate) enum RemoteHead<'a> {
    /// The object is not present (404, or an error the cloud's policy treats
    /// as absent — e.g. S3's fail-open-on-403).
    Absent,
    /// The object exists. `digest` carries the stored `sfc-digest` metadata
    /// value iff the HEAD response had a parseable one; `None` when the object
    /// predates digest tagging or the header was malformed.
    Present { digest: Option<&'a str> },
}

/// Outcome of the pre-upload skip check, shared by all three cloud upload
/// paths. Extracted so the decision is testable independent of each cloud's
/// HEAD-elision optimization (the elision can hide a missing guard in the
/// content-match branch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipDecision {
    /// The remote object is visible and the caller didn't request overwrite.
    /// Skip without comparing content — stale stage bytes are preserved.
    Existence,
    /// The caller opted into content-match skipping and the remote digest
    /// equals the local one. Skip the redundant upload; the bytes on stage
    /// are already what we'd have written.
    ContentMatch,
    /// Neither skip applies; upload proceeds.
    Upload,
}

/// Whether a pre-upload HEAD is worth issuing. This MUST stay the disjunction
/// of every condition under which [`classify_pre_upload_skip`] can return a
/// non-`Upload` decision on a *present* object: existence-skip needs the HEAD
/// under `!overwrite`, and content-match needs it under
/// `skip_upload_on_content_match`. Add a third skip trigger and this predicate
/// has to gain the matching term in lockstep, or a cloud will silently elide a
/// HEAD it needs. Kept beside the classifier so "when to probe" lives next to
/// "what the probe means"; every cloud's upload path calls it to decide whether
/// to run its HEAD.
pub(crate) fn head_needed(overwrite: bool, skip_upload_on_content_match: bool) -> bool {
    !overwrite || skip_upload_on_content_match
}

/// Pure decision: which skip branch (if any) fires. Existence-only is checked
/// first so a `!overwrite` caller never reaches the content-match branch — a
/// remote object that exists is treated as authoritative regardless of
/// digest. A missing remote digest cannot match, so the content branch falls
/// through to `Upload` in that case.
///
/// `remote` is a cloud-agnostic [`RemoteHead`], not a cloud SDK response type,
/// so the decision stays free of any cloud's HEAD-response shape; each cloud's
/// call site builds a `RemoteHead` from its own probe result.
pub(crate) fn classify_pre_upload_skip(
    overwrite: bool,
    skip_upload_on_content_match: bool,
    remote: &RemoteHead<'_>,
    local_digest: &str,
) -> SkipDecision {
    if !overwrite && matches!(remote, RemoteHead::Present { .. }) {
        return SkipDecision::Existence;
    }
    if overwrite
        && skip_upload_on_content_match
        && matches!(remote, RemoteHead::Present { digest: Some(d) } if *d == local_digest)
    {
        return SkipDecision::ContentMatch;
    }
    SkipDecision::Upload
}

/// Shared pre-upload skip step: classify, and if a skip fires, log it
/// (tagged with `cloud`) and return `Skipped`. `None` means proceed to
/// upload. Each cloud still owns its own HEAD probe and error policy; only
/// this pure decision + log + return step is shared, removing the per-cloud
/// triplication of the classify/match/log block.
pub(crate) fn skip_upload_decision(
    cloud: LocationType,
    overwrite: bool,
    skip_upload_on_content_match: bool,
    remote: &RemoteHead<'_>,
    local_digest: &str,
    key: &str,
) -> Option<UploadStatus> {
    match classify_pre_upload_skip(
        overwrite,
        skip_upload_on_content_match,
        remote,
        local_digest,
    ) {
        SkipDecision::Existence => {
            tracing::info!("{cloud}: remote object already exists, skipping upload: {key}");
            Some(UploadStatus::Skipped)
        }
        SkipDecision::ContentMatch => {
            tracing::info!("{cloud}: remote content matches local digest, skipping upload: {key}");
            Some(UploadStatus::Skipped)
        }
        SkipDecision::Upload => None,
    }
}

/// Test builder for a minimal [`PreparedUpload`] carrying `digest` — an empty
/// in-memory payload, no CSE. This is the shape all three clouds' skip tests
/// need; sharing it here removes the copy-pasted per-cloud builders. The skip
/// branch returns before the body is streamed, so the empty payload is never
/// read.
#[cfg(test)]
pub(crate) fn prepared_upload_with_digest(digest: &str) -> PreparedUpload {
    PreparedUpload {
        source: types::PreparedSource::Bytes(bytes::Bytes::new()),
        digest: digest.to_string(),
        cse: None,
    }
}

/// Bytes read from the source for compression auto-detection. Every
/// `CompressionType` we currently detect has its magic at offset 0 (gzip 0–1,
/// bzip2 0–2, zstd 0–3, parquet/ORC 0–3), so 16 bytes would suffice today.
/// The 512-byte buffer is future-proofing: the `infer` crate's archive
/// matchers read up to ~265 bytes (e.g. tar's `ustar` at offset 257), so if
/// we ever map one of those archive kinds to a `CompressionType` the buffer
/// already covers it. 512 is O(1) regardless of file size.
const COMPRESSION_DETECT_PREFIX_LEN: usize = 512;

/// Uploads every file matching `data.src_location_pattern`, sequentially.
///
/// On cancellation the loop stops, so later files never begin; the in-flight one
/// is aborted by the cleanup registered further down; files already uploaded stay
/// on the stage, complete and valid but unreported — the caller sees only
/// `ApiError::Cancelled`, never partial result rows.
pub async fn upload_files(
    data: &UploadData,
    policy: &RetryPolicy,
    tx: TransferCtx<'_>,
) -> Result<Vec<UploadResult>, FileManagerError> {
    // `expand_filenames` globs the filesystem and canonicalizes every match
    // (stat/readlink syscalls per path component), so it runs in
    // `spawn_blocking` to keep the runtime thread free, matching every other
    // blocking-I/O call in this file.
    let pattern = data.src_location_pattern.clone();
    let file_locations = tokio::task::spawn_blocking(move || expand_filenames(&pattern))
        .await
        .context(BlockingTaskSnafu)?
        .context(PathExpansionSnafu)?;

    if file_locations.is_empty() {
        return NoFilesMatchedSnafu {
            pattern: data.src_location_pattern.clone(),
        }
        .fail();
    }

    let mut results = Vec::with_capacity(file_locations.len());
    let mut failures: Vec<String> = Vec::new();

    // The refresher owns the latest stage info (creds + presigned URLs) for
    // the batch via its shared `StageInfoCache`; per-file calls read from
    // that cache, so refreshed creds/URLs heal the remaining files
    // automatically (matching Python's shared `StorageCredential`). The
    // refresher coalesces rapid-fire token refresh calls across files; URL
    // refresh is intentionally not coalesced (each file may carry its own
    // presigned URL).
    for file_location in file_locations {
        let stage_info = current_stage_info(&data.stage_info, tx.refresher);
        let path = PathBuf::from(&file_location.path);
        // Retained for the aggregate-failure report: `filename` moves into
        // `SingleUploadData` below, but a failed transfer still needs a name
        // to list in `UploadBatch`.
        let name = file_location.filename.clone();
        let single_upload_data = SingleUploadData {
            source: ByteSource::Path(path),
            filename: file_location.filename,
            stage_info,
            encryption_material: data.encryption_material.clone(),
            auto_compress: data.auto_compress,
            source_compression: data.source_compression.clone(),
            overwrite: data.overwrite,
            flavor: data.flavor.clone(),
            legacy_odbc_compression_autodetect: data.legacy_odbc_compression_autodetect,
            skip_upload_on_content_match: data.skip_upload_on_content_match,
            multipart: data.multipart,
        };

        match upload_single_file(single_upload_data, policy, tx).await {
            Ok(result) => results.push(result),
            // Fail-fast aborts at the first error; collect-all (the default)
            // attempts every file and reports all failures together (ODBC parity,
            // SNOW-3838438).
            Err(e) => {
                if data.put_fastfail {
                    return Err(e);
                }
                failures.push(format!("{name}: {e}"));
            }
        }
    }

    if !failures.is_empty() {
        return UploadBatchSnafu {
            failure_count: failures.len(),
            failures: failures.join("\n"),
        }
        .fail();
    }

    Ok(results)
}

/// Returns a copy of `base` with `creds` and `presigned_url` overlaid from
/// the refresher's current `StageInfoSnapshot`, when a refresher is present.
/// Without a refresher, `base` is returned unchanged.
///
/// The snapshot's `presigned_urls[]` lives on `DownloadData` (not
/// `StageInfo`); the per-file GCS GET path reads it directly from the
/// refresher cache at the call site (see `download_from_gcs`).
fn current_stage_info(base: &StageInfo, refresher: Option<&dyn StageInfoRefresher>) -> StageInfo {
    refresher.map_or_else(
        || base.clone(),
        |r| base.with_snapshot(r.cache().snapshot()),
    )
}

/// Uploads one file. The `refresher` (if any) is used to refresh stage info
/// on recoverable errors:
/// - S3 stages: AWS `ExpiredToken` triggers a creds refresh
///   (`s3_transfer::upload_to_s3_or_skip`).
/// - GCS stages: 401 triggers a creds refresh; 400 in presigned-mode
///   triggers a URL refresh (`gcs_transfer::upload_to_gcs_or_skip`).
/// - Azure stages: any 403 triggers a SAS refresh via a fresh PUT/GET query
///   (`azure_transfer::upload_to_azure_or_skip`, `download_from_azure`).
///
/// Refreshed snapshots are stored in the refresher's `StageInfoCache` rather
/// than returned here.
pub async fn upload_single_file(
    data: SingleUploadData,
    policy: &RetryPolicy,
    tx: TransferCtx<'_>,
) -> Result<UploadResult, FileManagerError> {
    // `preprocess_file_before_upload` reads a `ByteSource::Path` itself
    // (streaming), so `upload_single_file` no longer pre-reads the file.
    upload_prepared_source(data.source.clone(), data, policy, tx).await
}

/// Uploads an in-memory byte buffer to the stage location described by
/// `data`. Skips the `ByteSource::Path` disk read that [`upload_single_file`]
/// delegates and instead wraps the buffer in `ByteSource::Bytes`, sharing the
/// same cloud-upload path so encryption, compression, SHA-256 digesting, and
/// the per-cloud (S3 / GCS / Azure) dispatch behave identically.
///
/// The upload result's `source` column is derived from `data.source` /
/// `data.filename` (see `upload_result_source`); callers that do not surface
/// the upload result back to the user (notably the large-bindings stage
/// uploader) need not set a meaningful `data.source`.
pub async fn upload_in_memory_file(
    buffer: Vec<u8>,
    data: SingleUploadData,
    policy: &RetryPolicy,
    tx: TransferCtx<'_>,
) -> Result<UploadResult, FileManagerError> {
    upload_prepared_source(ByteSource::Bytes(buffer.into()), data, policy, tx).await
}

/// Shared core of the upload path used by `upload_single_file` (file
/// source), `upload_in_memory_file` (in-memory source), and the chunked
/// upload stream path (`build_and_upload_stream`, which may pass either a
/// `ByteSource::Bytes` or a `ByteSource::Path` pointing at a spooled temp
/// file). Taking the `ByteSource` as a parameter lets every caller reuse the
/// same preprocess + cloud dispatch with no behavior drift.
pub(crate) async fn upload_prepared_source(
    source: ByteSource,
    data: SingleUploadData,
    policy: &RetryPolicy,
    tx: TransferCtx<'_>,
) -> Result<UploadResult, FileManagerError> {
    // `preprocess_file_before_upload` reads the source file from disk and
    // AES-encrypts it (blocking I/O + CPU-bound); run it off the async executor
    // via `spawn_blocking`, including JDBC ERROR-row `stat`. `data` is moved
    // in and handed back out so the cloud dispatch below can keep using it
    // without cloning.
    let preprocess_outcome =
        tokio::task::spawn_blocking(move || match preprocess_file_before_upload(source, &data) {
            Ok(ok) => Ok(Ok((data, ok))),
            Err(e) => on_upload_preprocess_error(&data, e).map(Err),
        })
        .await
        .context(BlockingTaskSnafu)??;
    let (data, (prepared, file_metadata)) = match preprocess_outcome {
        Ok(ready) => ready,
        Err(row) => return Ok(row),
    };

    let status = match data.stage_info.location_type {
        LocationType::S3 => upload_to_s3_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            data.skip_upload_on_content_match,
            policy,
            data.multipart,
            tx,
        )
        .await
        .context(S3UploadSnafu)?,
        LocationType::Gcs => upload_to_gcs_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            data.skip_upload_on_content_match,
            data.multipart,
            // Build the policy here, where `using_presigned_url` is known, and
            // pass it by reference so the test seam can inject zero backoff.
            &gcs_retry_policy(data.stage_info.presigned_url.is_some(), policy),
            tx,
        )
        .await
        .context(GcsUploadSnafu)?,
        LocationType::Azure => upload_to_azure_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            data.skip_upload_on_content_match,
            data.multipart,
            policy,
            // Refresher only: Azure has no abort to register (see
            // `azure_multipart_upload`).
            tx.refresher,
        )
        .await
        .context(AzureUploadSnafu)?,
    };

    // `message` is populated for `Skipped` under ODBC (see `upload_result_message`).
    // Failures are handled by the caller, `upload_files` — this success path only
    // ever returns `Uploaded`/`Skipped`.
    Ok(UploadResult {
        source: file_metadata.source,
        target: file_metadata.target,
        source_size: file_metadata.source_size,
        target_size: file_metadata.target_size,
        source_compression: file_metadata
            .source_compression
            .get_snowflake_representation()
            .to_string(),
        target_compression: file_metadata
            .target_compression
            .get_snowflake_representation()
            .to_string(),
        message: upload_result_message(status, &data.flavor).to_string(),
        status: status.to_string(),
    })
}

/// Returns the `message` column value for a completed upload, gated on the
/// active wrapper flavor. Legacy ODBC always populates the message with
/// `ODBC_PUT_MESSAGE_SKIPPED` for skipped uploads (overwrite=false +
/// target already exists); every other (flavor, status) combination uses
/// an empty string.
fn upload_result_message(status: UploadStatus, flavor: &PutGetResultsetFlavor) -> &'static str {
    match (status, flavor) {
        (UploadStatus::Skipped, PutGetResultsetFlavor::Odbc) => ODBC_PUT_MESSAGE_SKIPPED,
        _ => "",
    }
}

/// Returns the `source` column value for a completed upload, gated on the
/// active wrapper flavor and host platform. Legacy driver provides full path
/// verbatim on Windows, the `Odbc` flavor restores that behaviour; every other
/// combination keeps the `Path::file_name()` basename that UD-Python has always
/// reported.
///
/// `is_windows` is parameterized rather than read from `cfg!(windows)`
/// inside the helper so the unit tests can exercise both branches on
/// any host.
fn upload_result_source(
    source: &ByteSource,
    filename: &str,
    flavor: &PutGetResultsetFlavor,
    is_windows: bool,
) -> String {
    match (is_windows, flavor, source) {
        // Windows ODBC parity: emit the original local path (with forward
        // slashes) for the result's `source` column. For in-memory uploads
        // there is no local path, so fall back to the basename like every
        // other (flavor, host) combination.
        (true, PutGetResultsetFlavor::Odbc, ByteSource::Path(p)) => {
            p.display().to_string().replace('\\', "/")
        }
        _ => filename.to_string(),
    }
}

/// Sets file metadata, compresses the file if needed, and optionally encrypts the data.
/// For SSE stages (no encryption material), the data is uploaded without client-side encryption.
fn preprocess_file_before_upload(
    source: ByteSource,
    data: &SingleUploadData,
) -> Result<(PreparedUpload, UploadMetadata), FileManagerError> {
    let (prefix, source_size) = read_prefix_and_size(&source)?;

    let source_compression = get_source_compression(
        data.filename.as_str(),
        &prefix,
        &data.source_compression,
        data.legacy_odbc_compression_autodetect,
    )
    .context(CompressionTypeSnafu)?;

    let result_source = upload_result_source(
        &data.source,
        data.filename.as_str(),
        &data.flavor,
        cfg!(windows),
    );
    let mut target = data.filename.clone();

    let (upload_source, target_compression, gzip_tempfile) =
        if data.auto_compress && source_compression == CompressionType::None {
            // Stream the gzip output to a tempfile instead of buffering it in
            // heap; that tempfile then becomes the upload source (read lazily
            // during the body stream), so it must outlive the upload.
            let (path, temp_path) = compress_to_tempfile(&source).context(CompressionSnafu)?;
            target = format!("{}.gz", data.filename);
            (
                ByteSource::Path(path),
                CompressionType::Gzip,
                Some(temp_path),
            )
        } else {
            (source, source_compression.clone(), None)
        };

    // The upload source after optional auto-compression: the gzip tempfile, the
    // original file, or in-memory bytes. Encryption (CSE) is applied lazily
    // while building the cloud body, so the source is what we measure and hash
    // here; ciphertext is never materialized.
    let source_len = match &upload_source {
        ByteSource::Bytes(b) => b.len() as i64,
        ByteSource::Path(p) => std::fs::metadata(p).context(IoSnafu)?.len() as i64,
    };

    // `sfc-digest` is the SHA-256 of the pre-encryption source for both CSE and
    // SSE (matching JDBC/ODBC), so it can be computed once, up front.
    let digest = compute_sha256_digest(&upload_source).context(DigestComputationSnafu)?;

    let cse = match &data.encryption_material {
        Some(material) => {
            let (encryptor, metadata) =
                build_encryptor(material, source_len).context(EncryptionSnafu)?;
            Some(CseParams {
                metadata,
                encryptor,
            })
        }
        None => None,
    };

    // `target_size` = bytes landing in the stage: ciphertext length for CSE, or
    // source length for SSE. JDBC diverges — it reports the post-compression,
    // pre-encryption size (`source_len`) instead of the ciphertext length.
    let target_size = match data.flavor {
        PutGetResultsetFlavor::Jdbc => source_len,
        _ => cse
            .as_ref()
            .map(|c| c.encryptor.cipher_len())
            .unwrap_or(source_len),
    };

    // Bundle the body source with its tempfile guard (if any). For the gzip
    // path the tempfile *is* the source, so the guard travels with it; every
    // other source carries no guard.
    let source = match gzip_tempfile {
        Some(temp_path) => PreparedSource::GzipTempfile {
            path: temp_path.to_path_buf(),
            _guard: Arc::new(temp_path),
        },
        None => PreparedSource::from(upload_source),
    };

    let prepared = PreparedUpload {
        source,
        digest,
        cse,
    };

    Ok((
        prepared,
        UploadMetadata {
            source: result_source,
            target,
            source_size,
            source_compression,
            target_size,
            target_compression,
        },
    ))
}

/// Reads the first `COMPRESSION_DETECT_PREFIX_LEN` bytes for compression
/// auto-detect, plus the source's total byte count.
///
/// For `ByteSource::Path` this opens the file once for the prefix + metadata
/// read; the upload path opens it again later (to compute the digest, and again
/// per attempt to stream/encrypt the body). If the file changes between opens,
/// the `source_size` reported here — and, for CSE, the analytic `Content-Length`
/// derived from it — can disagree with the bytes actually produced, which the
/// cloud SDK rejects (a digest mismatch is the milder failure). This is inherent
/// to streaming a mutable on-disk source; the pre-streaming code did one atomic
/// `read_to_end`, at the cost of the entire memory bound.
fn read_prefix_and_size(source: &ByteSource) -> Result<(Vec<u8>, i64), FileManagerError> {
    match source {
        ByteSource::Path(p) => {
            let f = File::open(p).context(IoSnafu)?;
            let size = f.metadata().context(IoSnafu)?.len() as i64;
            let mut prefix = Vec::with_capacity(COMPRESSION_DETECT_PREFIX_LEN);
            f.take(COMPRESSION_DETECT_PREFIX_LEN as u64)
                .read_to_end(&mut prefix)
                .context(IoSnafu)?;
            Ok((prefix, size))
        }
        ByteSource::Bytes(b) => {
            let prefix = b[..b.len().min(COMPRESSION_DETECT_PREFIX_LEN)].to_vec();
            Ok((prefix, b.len() as i64))
        }
    }
}

/// Uses user-specified compression type or auto-detects the compression type based on the file name and content.
fn get_source_compression(
    filename: &str,
    file_buffer: &[u8],
    source_compression: &SourceCompressionParam,
    legacy_odbc_compression_autodetect: bool,
) -> Result<CompressionType, CompressionTypeError> {
    match source_compression {
        SourceCompressionParam::AutoDetect => auto_detect_source_compression(
            filename,
            file_buffer,
            legacy_odbc_compression_autodetect,
        ),
        SourceCompressionParam::None => Ok(CompressionType::None),
        SourceCompressionParam::Gzip => Ok(CompressionType::Gzip),
        SourceCompressionParam::Bzip2 => Ok(CompressionType::Bzip2),
        SourceCompressionParam::Brotli => Ok(CompressionType::Brotli),
        SourceCompressionParam::Zstd => Ok(CompressionType::Zstd),
        SourceCompressionParam::Deflate => Ok(CompressionType::Deflate),
        SourceCompressionParam::RawDeflate => Ok(CompressionType::RawDeflate),
        SourceCompressionParam::Parquet => Ok(CompressionType::Parquet),
        SourceCompressionParam::Orc => Ok(CompressionType::Orc),
    }
}

/// Returns the resolved compression type for the `AUTO_DETECT` path.
/// `legacy_odbc_compression_autodetect` (true) opts
/// into two libsnowflakeclient-parity behaviors at once (see
/// `WrapperPresets` for the full doc-comment):
///
/// 1. Short-prefix magic-byte table runs ahead of the `infer` crate,
///    detecting 2-byte gzip / 2-byte zlib (mapped to `Deflate`) / 4-byte
///    snowflake brotli marker that `infer` would miss.
/// 2. Unsupported formats (`.xz`, `.lz`, `.lzma`, `.lzo`, `.Z`, plus the
///    buffer-detected equivalents) are silently treated as uncompressed
///    instead of erroring. Recovery is keyed on the
///    `UnsupportedCompressionType` error variant, so it fires regardless
///    of whether detection went through the filename extension or the
///    magic-bytes path.
fn auto_detect_source_compression(
    filename: &str,
    file_buffer: &[u8],
    legacy_odbc_compression_autodetect: bool,
) -> Result<CompressionType, CompressionTypeError> {
    let detected =
        try_guess_compression_type(filename, file_buffer, legacy_odbc_compression_autodetect);
    if legacy_odbc_compression_autodetect {
        match detected {
            Err(CompressionTypeError::UnsupportedCompressionType { .. }) => {
                Ok(CompressionType::None)
            }
            other => other,
        }
    } else {
        detected
    }
}

/// Downloads every file listed in `data.src_locations`, sequentially.
///
/// On cancellation the loop stops, so later files never begin; the in-flight one
/// is stopped and its `.part` removed by the cleanup registered in
/// [`download_single_file`]; files already downloaded stay on disk, whole because
/// each was published by an atomic rename. The caller sees only
/// `ApiError::Cancelled`, never partial result rows.
pub async fn download_files(
    mut data: DownloadData,
    policy: &RetryPolicy,
    tx: TransferCtx<'_>,
) -> Result<Vec<DownloadResult>, FileManagerError> {
    let mut results = Vec::new();

    // Three-way zip: src_locations / encryption_materials / presigned_urls.
    // `presigned_urls` is built in `query_response::to_file_download_data` to
    // be the same length as `src_locations` (padded with `None` when GS
    // omitted entries) so the zip never silently drops a file. See
    // `DownloadData.presigned_urls` doc-comment for the alignment invariant.
    //
    // The per-file index (`enumerate`) is forwarded into `download_single_file`
    // so the GCS layer can re-resolve `presigned_urls[i]` from the refresher
    // cache after a 400-triggered URL refresh.
    let download_iter = data
        .src_locations
        .drain(..)
        .zip(data.encryption_materials.drain(..))
        .zip(data.presigned_urls.drain(..))
        .enumerate();
    for (index, ((file_location, encryption_material), presigned_url)) in download_iter {
        let stage_info = current_stage_info(&data.stage_info, tx.refresher);
        // Retained for the collect-all ERROR row: `file_location` moves into
        // `SingleDownloadData` below, but a failed transfer still needs a
        // `file` column to report.
        let name = file_location.clone();
        let single_download_data = SingleDownloadData {
            src_location: file_location,
            local_location: data.local_location.clone(),
            stage_info,
            encryption_material,
            presigned_url,
            flavor: data.flavor.clone(),
            multipart: data.multipart,
            unsafe_file_write: data.unsafe_file_write,
        };

        match download_single_file(single_download_data, policy, index, tx).await {
            Ok(result) => results.push(result),
            // Fail-fast (`get_fastfail`) aborts the batch on the first error;
            // collect-all records an ERROR row and continues (ODBC parity,
            // SNOW-3838438).
            Err(e) => results.push(on_download_file_error(data.get_fastfail, name, e)?),
        }
    }

    Ok(results)
}

/// JDBC-only: map unsupported AUTO_DETECT codecs to a PUT ERROR row. Other
/// flavors and other preprocess errors still return `Err`.
fn on_upload_preprocess_error(
    data: &SingleUploadData,
    error: FileManagerError,
) -> Result<UploadResult, FileManagerError> {
    if data.flavor == PutGetResultsetFlavor::Jdbc
        && let FileManagerError::CompressionType { source, .. } = &error
    {
        let CompressionTypeError::UnsupportedCompressionType { type_name, .. } = source;
        return Ok(unsupported_compression_error_row(data, type_name));
    }
    Err(error)
}

fn unsupported_compression_error_row(data: &SingleUploadData, type_name: &str) -> UploadResult {
    let source_size = match &data.source {
        ByteSource::Path(p) => std::fs::metadata(p).map(|m| m.len() as i64).unwrap_or(0),
        ByteSource::Bytes(b) => b.len() as i64,
    };
    UploadResult {
        source: data.filename.clone(),
        target: data.filename.clone(),
        source_size,
        target_size: 0,
        source_compression: type_name.to_string(),
        target_compression: CompressionType::None
            .get_snowflake_representation()
            .to_string(),
        status: "ERROR".to_string(),
        message: jdbc_unsupported_compression_message(type_name),
    }
}

/// Batch failure policy for a single failed GET. Fail-fast
/// (`get_fastfail = true`) returns the error to abort the batch; collect-all
/// (`get_fastfail = false`, ODBC's default) folds it into an `ERROR`-status
/// result row so the remaining files still download. PUT diverges here: a
/// failed collect-all PUT raises `UploadBatch` (see `upload_files`),
/// matching legacy ODBC where GET returns per-file rows but PUT throws.
fn on_download_file_error(
    get_fastfail: bool,
    name: String,
    error: FileManagerError,
) -> Result<DownloadResult, FileManagerError> {
    if get_fastfail {
        return Err(error);
    }
    Ok(DownloadResult {
        file: name,
        size: 0,
        status: "ERROR".to_string(),
        message: error.to_string(),
    })
}

/// GET path guard layer 1 (SNOW-3663590; mirrors JDBC
/// `extractSafeDestFileName`): reduce the server-controlled `src_location` to a
/// single basename, rejecting empty / `.` / `..` / NUL / separators / `:`.
fn safe_download_file_name(src_location: &str) -> Result<&str, FileManagerError> {
    let name = match src_location.rfind(['/', '\\']) {
        Some(idx) => &src_location[idx + 1..],
        None => src_location,
    };

    let rejected =
        name.is_empty() || name == "." || name == ".." || name.contains(['\0', '/', '\\', ':']);
    if rejected {
        return DownloadPathRejectedSnafu {
            src_location: src_location.to_string(),
            local_location: String::new(),
        }
        .fail();
    }
    Ok(name)
}

/// GET path guard layer 2 (mirrors JDBC `assertWithinDirectory`): join the
/// safe basename onto the canonicalized `local_location` and confirm the result
/// stays inside it before any file is created. The caller creates
/// `local_location` first (SNOW-3704966), so `canonicalize` here also confirms it.
/// The containment check is defense-in-depth against future layer-1 changes and
/// catches a leaf that already exists as a symlink escaping `base_dir`.
fn resolve_validated_output_path(
    local_location: &str,
    src_location: &str,
) -> Result<PathBuf, FileManagerError> {
    let filename = safe_download_file_name(src_location)?;
    let base_dir = std::fs::canonicalize(local_location).context(IoSnafu)?;
    let output_path = base_dir.join(filename);
    // Layer 1 guarantees a separator-free basename, so the lexical join can only
    // be a direct child. But the leaf may already exist as a symlink pointing
    // outside `base_dir` (JDBC canonicalizes the full dest to catch this); if so,
    // resolve and re-check. A nonexistent leaf is the normal case — no escape.
    let resolved = std::fs::canonicalize(&output_path).unwrap_or_else(|_| output_path.clone());
    if !resolved.starts_with(&base_dir) {
        return DownloadPathRejectedSnafu {
            src_location: src_location.to_string(),
            local_location: local_location.to_string(),
        }
        .fail();
    }
    Ok(output_path)
}

/// Prepares the on-disk destination for one downloaded file: create
/// `local_location` recursively if missing (SNOW-3704966; matches Python
/// `os.makedirs` and JDBC), run the GET path guard, and derive the sibling
/// `<output>.part` temp path (downloads write there and `rename` on success, so
/// observers never see partial plaintext). Blocking; call inside `spawn_blocking`.
fn prepare_download_output_paths(
    local_location: &str,
    src_location: &str,
) -> Result<(PathBuf, PathBuf), FileManagerError> {
    std::fs::create_dir_all(local_location).context(IoSnafu)?;
    let output_path = resolve_validated_output_path(local_location, src_location)?;
    let partial_path = {
        let mut s = output_path.as_os_str().to_owned();
        s.push(".part");
        PathBuf::from(s)
    };
    Ok((output_path, partial_path))
}

/// Downloads one file. See `upload_single_file` for the refresh semantics.
///
/// `per_file_index` is the file's index inside the GET batch — i.e. its
/// position in `DownloadData.presigned_urls` / `DownloadData.src_locations`.
/// The GCS branch uses it to re-pick `presigned_urls[i]` from the refresher
/// cache after a 400-triggered URL refresh. Non-GCS branches ignore it.
///
/// For GCS and Azure, the response body is streamed directly into the
/// decrypt/write operation via `decrypt_ciphertext_to_writer` without buffering
/// the full ciphertext in memory. The blocking decrypt call runs in
/// `tokio::task::spawn_blocking` so the async runtime thread is free while the
/// blocking channel receive waits for the next chunk from the async producer.
///
/// For S3, a single buffered GET is used below the multipart threshold and
/// parallel ranged GETs into a tempfile above it. CSE objects decrypt the
/// ciphertext through a blocking `Read`; SSE objects skip decryption — a spilled
/// (ranged) download is renamed into place, an in-memory (small) one is copied
/// straight to the destination.
pub async fn download_single_file(
    data: SingleDownloadData,
    policy: &RetryPolicy,
    per_file_index: usize,
    tx: TransferCtx<'_>,
) -> Result<DownloadResult, FileManagerError> {
    // Blocking FS syscalls (create_dir_all/canonicalize); keep off the async executor.
    let (output_path, partial_path) = {
        let local_location = data.local_location.clone();
        let src_location = data.src_location.clone();
        tokio::task::spawn_blocking(move || {
            prepare_download_output_paths(&local_location, &src_location)
        })
        .await
        .context(BlockingTaskSnafu)??
    };

    // Armed before the first byte is requested, because `<dst>.part` can exist as
    // early as the ranged assembly's pre-allocation. Cancellation only — a *failed*
    // download still cleans up through its own error paths, unchanged.
    let remove_partial_on_cancel = {
        let partial = partial_path.clone();
        async move { remove_partial_after_cancel(partial).await }
    };

    with_cleanup_scope_opt(
        tx.cleanup,
        remove_partial_on_cancel,
        // Boxed to keep this large future off the frame — see clippy.toml.
        Box::pin(download_single_file_to(
            data,
            policy,
            per_file_index,
            tx,
            output_path,
            partial_path,
        )),
    )
    .await
}

/// Body of [`download_single_file`], with the destination paths already resolved
/// so the caller can arm cleanup for `partial_path` before any transfer begins.
async fn download_single_file_to(
    mut data: SingleDownloadData,
    policy: &RetryPolicy,
    per_file_index: usize,
    tx: TransferCtx<'_>,
    output_path: PathBuf,
    partial_path: PathBuf,
) -> Result<DownloadResult, FileManagerError> {
    // CSE downloads decrypt the ciphertext through a blocking `Read`; SSE
    // downloads skip decryption and write the raw bytes. S3 buffers small blobs
    // in memory and spills large ranged downloads to a tempfile (renamed into
    // place on the SSE path); GCS/Azure stream from the network.
    //
    // CSE verifies the SHA-256 digest at finalize time rather than pre-checking
    // it: pre-verification would require buffering the full ciphertext, which
    // defeats the streaming refactor. The integrity guarantee is preserved (a
    // tampered byte still yields DigestMismatch); only the failure-mode timing
    // differs. Every branch writes to `partial_path` and renames on success — the
    // user-visible destination only ever appears as a complete artefact, even if
    // a concurrent FS observer is racing.
    // Extract enc_material and unsafe_file_write before the match so all three
    // arms can move them into their spawn_blocking closures
    // (EncryptionMaterial is not Clone).
    let enc_material = data.encryption_material.take();
    let unsafe_file_write = data.unsafe_file_write;
    let (cloud_byte_count, output_byte_len) = match data.stage_info.location_type {
        LocationType::S3 => {
            // Spill parallel ranged downloads next to the destination (not the
            // system temp dir) so the SSE finalize below is a same-filesystem
            // rename rather than a cross-device copy.
            // unwrap_or_else uses "." (current dir) rather than temp_dir so
            // the spill stays on the same filesystem as the destination,
            // keeping the subsequent rename cross-device-safe. temp_dir can
            // be on a different FS, which makes NamedTempFile::persist fail
            // with EXDEV. parent() is only None when output_path has no
            // directory component (a bare filename), in which case "." is
            // the correct implicit parent.
            let spill_dir = output_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            // A non-encrypted ranged download assembles straight into `.part`
            // (one rename to publish; any hard-kill leftover is a
            // self-overwriting `.part`). An encrypted (or git-stage) download
            // has `encryption_material`, so its ciphertext goes to a temp in
            // `spill_dir` and is decrypted into `.part` below.
            let spill_target = if enc_material.is_some() {
                CloudSpillTarget::Temp {
                    dir: &spill_dir,
                    cleanup: tx.cleanup,
                }
            } else {
                CloudSpillTarget::Part(&partial_path)
            };
            let S3Download {
                body,
                digest,
                file_metadata,
                cloud_byte_count,
            } = download_from_s3(
                &data.stage_info,
                data.src_location.as_str(),
                policy,
                data.multipart,
                tx.refresher,
                unsafe_file_write,
                spill_target,
            )
            .await
            .context(S3DownloadSnafu)?;

            let partial_path2 = partial_path.clone();

            // Write to `<dst>.part` but do NOT rename inside spawn_blocking.
            // Rename happens after the `.await` (see below), so a cancelled/
            // dropped outer future cannot publish a file written by a detached
            // blocking task. The `.await` itself is the cancellation point.
            let (output_byte_len, spilled_temp) = tokio::task::spawn_blocking(
                move || -> Result<(i64, Option<tempfile::TempPath>), FileManagerError> {
                    match (enc_material, file_metadata, digest) {
                        // Client-side-encrypted object: decrypt the ciphertext
                        // (from the in-memory buffer or the spilled tempfile),
                        // verifying the SHA-256 digest at finalize time.
                        (Some(enc_material), Some(enc_metadata), Some(d)) => {
                            let reader = body.into_reader().context(IoSnafu)?;
                            let mut output_file =
                                create_output_file(&partial_path2, unsafe_file_write)
                                    .context(IoSnafu)?;
                            let result = decrypt_ciphertext_to_writer(
                                reader,
                                &enc_metadata,
                                d.as_str(),
                                &enc_material,
                                &mut output_file,
                            )
                            .context(DecryptionSnafu);
                            write_or_cleanup(output_file, &partial_path2, result).map(|n| (n, None))
                        }
                        // Non-decrypting cases — the cloud bytes are already the
                        // final plaintext:
                        //   * SSE stage — no `encryption_material` (server-side
                        //     decryption).
                        //   * `encryption_material` present but the object carries no
                        //     client-side-encryption headers (e.g. git-stage objects
                        //     on S3) — write raw bytes, matching legacy connector
                        //     behaviour (SNOW git-stage fix).
                        (maybe_enc, _, _) => {
                            if maybe_enc.is_some() {
                                tracing::debug!(
                                    "encryption_material present but S3 encryption headers absent; \
                                     writing raw bytes"
                                );
                            }
                            match body {
                                // Non-encrypted ranged download: the parallel GETs already
                                // assembled the whole object straight into `.part`. Nothing to
                                // copy — signal `None` so the post-await branch renames `.part`
                                // to the output (a single same-FS rename, no copy).
                                S3DownloadBody::Spilled(CloudSpilledBody::Part(_)) => {
                                    Ok((cloud_byte_count, None))
                                }
                                // git-stage ranged download: raw bytes were assembled into a
                                // temp (chosen because `encryption_material` was present). Hand
                                // the TempPath out so the caller renames it straight to output.
                                S3DownloadBody::Spilled(CloudSpilledBody::Temp(temp)) => {
                                    Ok((cloud_byte_count, Some(temp)))
                                }
                                // Small buffered download: copy the already-in-RAM
                                // bytes out (unavoidable and cheap).
                                S3DownloadBody::InMemory(bytes) => {
                                    let mut output_file =
                                        create_output_file(&partial_path2, unsafe_file_write)
                                            .context(IoSnafu)?;
                                    let result = std::io::copy(&mut &bytes[..], &mut output_file)
                                        .map(|n| n as i64)
                                        .context(IoSnafu);
                                    write_or_cleanup(output_file, &partial_path2, result)
                                        .map(|n| (n, None))
                                }
                            }
                        }
                    }
                },
            )
            .await
            .context(BlockingTaskSnafu)??;

            // Atomic publish: rename into place after the .await cancellation point.
            // `Some(temp)` (git-stage ranged download): the raw temp is renamed
            // directly to output — single same-FS rename.
            // `None` (CSE decrypt, InMemory copy, or non-encrypted ranged
            // download): the `.part` file is renamed to output via finalize_rename.
            // Running here — not inside spawn_blocking — means a dropped outer
            // future cannot publish; the blocking task may finish writing, but this
            // rename never executes unless the future reaches this point.
            match spilled_temp {
                Some(temp) => {
                    // git-stage ranged download: rename temp directly to output — single same-FS rename.
                    let output_for_rename = output_path.clone();
                    tokio::task::spawn_blocking(move || {
                        temp.persist(&output_for_rename)
                            .map(|_| ())
                            .map_err(|e| e.error)
                            .context(IoSnafu)
                    })
                    .await
                    .context(BlockingTaskSnafu)??;
                }
                None => {
                    // CSE / InMemory / non-encrypted-ranged path: rename `.part` into place.
                    let partial_for_rename = partial_path.clone();
                    let output_for_rename = output_path.clone();
                    tokio::task::spawn_blocking(move || {
                        finalize_rename(&partial_for_rename, &output_for_rename)
                    })
                    .await
                    .context(BlockingTaskSnafu)?
                    .context(IoSnafu)?;
                }
            }

            (cloud_byte_count, output_byte_len)
        }

        LocationType::Gcs => {
            // Spill ranged downloads next to the destination (see the S3 arm) so
            // an SSE finalize is a same-filesystem rename, not a cross-device copy.
            // unwrap_or_else uses "." (current dir) rather than temp_dir so the
            // spill stays on the same filesystem as the destination, keeping the
            // subsequent rename cross-device-safe. parent() is only None when
            // output_path has no directory component (a bare filename).
            let spill_dir = output_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            // A non-encrypted ranged download assembles straight into `.part`
            // (one rename to publish); an encrypted (or git-stage) download has
            // `encryption_material`, so its ciphertext goes to a temp in
            // `spill_dir` and is decrypted into `.part` below. Mirrors the S3 arm.
            let spill_target = if enc_material.is_some() {
                CloudSpillTarget::Temp {
                    dir: &spill_dir,
                    cleanup: tx.cleanup,
                }
            } else {
                CloudSpillTarget::Part(&partial_path)
            };
            let dl = download_from_gcs_streaming(
                &data.stage_info,
                data.src_location.as_str(),
                data.presigned_url.as_deref(),
                // Build the policy here, where `using_presigned_url` is known
                // (per-file URL or stage URL), and pass it by reference so the
                // test seam can inject zero backoff.
                &gcs_retry_policy(
                    data.presigned_url.is_some() || data.stage_info.presigned_url.is_some(),
                    policy,
                ),
                per_file_index,
                data.multipart,
                tx.refresher,
                unsafe_file_write,
                spill_target,
            )
            .await
            .context(GcsDownloadSnafu)?;

            let cloud_byte_count_hint = dl.cloud_byte_count;
            let cloud_bytes_read = dl.cloud_bytes_read;
            let cse_info = dl.cse_info;
            // Taken before `body` is moved into the blocking task, which consumes it.
            let producer_guard = ProducerAbortGuard::arm(dl.body.producer_abort());
            let body = dl.body;
            let partial_path2 = partial_path.clone();
            // Blocking finalize (decrypt / copy, no rename) in a spawn_blocking task
            // so the async runtime thread stays free to run the GCS producer that
            // feeds a streamed body's channel reader.
            // Write to `<dst>.part` but do NOT rename inside spawn_blocking.
            // Rename happens after the `.await` (see below), so a cancelled/
            // dropped outer future cannot publish a file written by a detached
            // blocking task. The `.await` itself is the cancellation point.
            let (output_byte_len, spilled_temp) = tokio::task::spawn_blocking(move || {
                write_cloud_download(
                    body,
                    cse_info,
                    enc_material,
                    cloud_byte_count_hint,
                    &partial_path2,
                    unsafe_file_write,
                )
            })
            .await
            .context(BlockingTaskSnafu)??;
            producer_guard.disarm();

            // Atomic publish — runs after .await so a dropped future cannot publish.
            match spilled_temp {
                Some(temp) => {
                    let output_for_rename = output_path.clone();
                    tokio::task::spawn_blocking(move || {
                        temp.persist(&output_for_rename)
                            .map(|_| ())
                            .map_err(|e| e.error)
                            .context(IoSnafu)
                    })
                    .await
                    .context(BlockingTaskSnafu)??;
                }
                None => {
                    let partial_for_rename = partial_path.clone();
                    let output_for_rename = output_path.clone();
                    tokio::task::spawn_blocking(move || {
                        finalize_rename(&partial_for_rename, &output_for_rename)
                    })
                    .await
                    .context(BlockingTaskSnafu)?
                    .context(IoSnafu)?;
                }
            }

            // Use Content-Length hint as cloud_byte_count; if absent (chunked TE),
            // fall back to the on-cloud ciphertext bytes actually pulled off the
            // wire. We must NOT fall back to output_byte_len here: for CSE objects
            // that is the decrypted *plaintext* length, which under-reports the
            // on-cloud size by the AES-CBC PKCS#7 padding delta (1–16 bytes) and
            // violates the documented "on-cloud (pre-decryption) byte count" contract.
            let cloud_byte_count = if cloud_byte_count_hint > 0 {
                cloud_byte_count_hint
            } else {
                cloud_bytes_read.load(std::sync::atomic::Ordering::Relaxed) as i64
            };
            (cloud_byte_count, output_byte_len)
        }

        LocationType::Azure => {
            // Spill ranged downloads next to the destination (see the S3 arm) so
            // an SSE finalize is a same-filesystem rename, not a cross-device copy.
            // unwrap_or_else uses "." (current dir) rather than temp_dir so the
            // spill stays on the same filesystem as the destination, keeping the
            // subsequent rename cross-device-safe. parent() is only None when
            // output_path has no directory component (a bare filename), in which
            // case "." is the correct implicit parent.
            let spill_dir = output_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            // A non-encrypted ranged download assembles straight into `.part`
            // (one rename to publish); an encrypted (or git-stage) download has
            // `encryption_material`, so its ciphertext goes to a temp in
            // `spill_dir` and is decrypted into `.part` below. Mirrors the S3 arm.
            let spill_target = if enc_material.is_some() {
                CloudSpillTarget::Temp {
                    dir: &spill_dir,
                    cleanup: tx.cleanup,
                }
            } else {
                CloudSpillTarget::Part(&partial_path)
            };
            let dl = download_from_azure_streaming(
                &data.stage_info,
                data.src_location.as_str(),
                data.multipart,
                policy,
                unsafe_file_write,
                spill_target,
                tx.refresher,
            )
            .await
            .context(AzureDownloadSnafu)?;

            let cloud_byte_count_hint = dl.cloud_byte_count;
            let cloud_bytes_read = dl.cloud_bytes_read;
            let cse_info = dl.cse_info;
            // Taken before `body` is consumed — see the GCS arm.
            let producer_guard = ProducerAbortGuard::arm(dl.body.producer_abort());
            let body = dl.body;
            let partial_path2 = partial_path.clone();
            // Write to `<dst>.part` but do NOT rename inside spawn_blocking;
            // same cancellation-safety rationale as the GCS arm above.
            let (output_byte_len, spilled_temp) = tokio::task::spawn_blocking(move || {
                write_cloud_download(
                    body,
                    cse_info,
                    enc_material,
                    cloud_byte_count_hint,
                    &partial_path2,
                    unsafe_file_write,
                )
            })
            .await
            .context(BlockingTaskSnafu)??;
            producer_guard.disarm();

            // Atomic publish — runs after .await so a dropped future cannot publish.
            match spilled_temp {
                Some(temp) => {
                    let output_for_rename = output_path.clone();
                    tokio::task::spawn_blocking(move || {
                        temp.persist(&output_for_rename)
                            .map(|_| ())
                            .map_err(|e| e.error)
                            .context(IoSnafu)
                    })
                    .await
                    .context(BlockingTaskSnafu)??;
                }
                None => {
                    let partial_for_rename = partial_path.clone();
                    let output_for_rename = output_path.clone();
                    tokio::task::spawn_blocking(move || {
                        finalize_rename(&partial_for_rename, &output_for_rename)
                    })
                    .await
                    .context(BlockingTaskSnafu)?
                    .context(IoSnafu)?;
                }
            }

            let cloud_byte_count = if cloud_byte_count_hint > 0 {
                cloud_byte_count_hint
            } else {
                // Same CSE caveat as the GCS arm: fall back to on-cloud ciphertext
                // bytes, never the decrypted plaintext length (output_byte_len).
                cloud_bytes_read.load(std::sync::atomic::Ordering::Relaxed) as i64
            };
            (cloud_byte_count, output_byte_len)
        }
    };

    tracing::info!(
        "File downloaded to '{}' ({} bytes)",
        output_path.display(),
        output_byte_len
    );

    Ok(DownloadResult {
        file: data.src_location,
        size: download_result_size(cloud_byte_count, output_byte_len, &data.flavor),
        status: "DOWNLOADED".to_string(),
        message: "".to_string(),
    })
}

/// Result of [`open_download_stream_for_stage`]: the chunk channel
/// `download_stream_chunk` drains, the background producer task, and the
/// on-cloud byte count from the GET's `Content-Length` (`0` if absent or the
/// object is empty).
pub struct DownloadStreamOpen {
    /// Plaintext chunks, in order. Terminal `Err` is the last item; clean
    /// EOF is the channel closing, not a final `Ok`.
    pub chunks: tokio::sync::mpsc::Receiver<Result<Vec<u8>, FileManagerError>>,
    /// The background `spawn_blocking` task draining the cloud body (and,
    /// in-flight, decrypting/gunzipping it) into `chunks`' sender half.
    pub task: tokio::task::JoinHandle<()>,
    /// Abort handle for the *inner* producer task reading the cloud body
    /// (see `cloud_http::spawn_s3_byte_stream_producer` /
    /// `spawn_byte_stream_producer`). Aborting `task` alone stops the
    /// decrypt/gunzip pipeline but leaves this parked on `body.next()` if
    /// the connection stalled — abort both.
    pub producer_abort: tokio::task::AbortHandle,
    pub cloud_byte_count: i64,
}

/// Opens a zero-disk, chunked streaming download from S3: issues one GET,
/// then a background task drains and decrypts/gunzips the body into
/// [`DownloadStreamOpen::chunks`]. Unlike [`download_single_file`]'s S3 arm,
/// nothing is buffered or spilled to disk beyond a few channel-sized chunks.
///
/// Only opening the GET is covered by the STS-refresh retry; once the body
/// is in hand, a mid-body failure surfaces as a terminal channel error with
/// no retry and no Range-resume (same tradeoff as GCS/Azure — see
/// `cloud_http::spawn_s3_byte_stream_producer`).
pub async fn open_s3_download_stream(
    stage_info: &StageInfo,
    src_location: &str,
    policy: &RetryPolicy,
    refresher: Option<&dyn StageInfoRefresher>,
    encryption_material: Option<EncryptionMaterial>,
    decompress: bool,
) -> Result<DownloadStreamOpen, FileManagerError> {
    let S3StreamingDownload {
        body,
        digest,
        file_metadata,
        cloud_byte_count,
    } = download_from_s3_streaming(stage_info, src_location, policy, refresher)
        .await
        .context(S3DownloadSnafu)?;

    let cse_info = match (file_metadata, digest) {
        (Some(metadata), Some(digest)) => Some(CseDownloadInfo { metadata, digest }),
        _ => None,
    };
    let (reader, producer_abort) = spawn_s3_byte_stream_producer(body);

    Ok(spawn_download_stream_pipeline(
        reader,
        producer_abort,
        encryption_material,
        cse_info,
        decompress,
        cloud_byte_count,
    ))
}

/// Opens a zero-disk, chunked streaming download against a GCS-backed stage.
/// See [`open_s3_download_stream`] for the shared contract; the only
/// difference is the cloud-specific GET (`download_from_gcs_streaming`).
pub async fn open_gcs_download_stream(
    stage_info: &StageInfo,
    src_location: &str,
    per_file_presigned_url: Option<&str>,
    policy: &RetryPolicy,
    refresher: Option<&dyn StageInfoRefresher>,
    encryption_material: Option<EncryptionMaterial>,
    decompress: bool,
) -> Result<DownloadStreamOpen, FileManagerError> {
    let using_presigned_url =
        per_file_presigned_url.is_some() || stage_info.presigned_url.is_some();
    // Zero-disk needs a single unranged, abortable GET. Call
    // `gcs_get_streaming` directly, not `download_from_gcs_streaming`, which
    // may route large downloads to a ranged/spilled body that
    // `open_cloud_download_stream` can't consume.
    let dl = gcs_get_streaming(
        stage_info,
        src_location,
        per_file_presigned_url,
        &gcs_retry_policy(using_presigned_url, policy),
        0,
        refresher,
    )
    .await
    .context(GcsDownloadSnafu)?;

    open_cloud_download_stream(dl, encryption_material, decompress)
}

/// Opens a zero-disk, chunked streaming download against an Azure-backed
/// stage. See [`open_s3_download_stream`] for the shared contract; the only
/// difference is the cloud-specific GET (`azure_get_streaming`).
pub async fn open_azure_download_stream(
    stage_info: &StageInfo,
    src_location: &str,
    policy: &RetryPolicy,
    refresher: Option<&dyn StageInfoRefresher>,
    encryption_material: Option<EncryptionMaterial>,
    decompress: bool,
) -> Result<DownloadStreamOpen, FileManagerError> {
    let dl = azure_get_streaming(stage_info, src_location, policy, refresher)
        .await
        .context(AzureDownloadSnafu)?;

    open_cloud_download_stream(dl, encryption_material, decompress)
}

/// Shared tail of [`open_gcs_download_stream`] and [`open_azure_download_stream`]:
/// unwraps a single-GET [`cloud_http::CloudStreamingDownload`] into its
/// live-network reader and hands off to [`spawn_download_stream_pipeline`].
///
/// `body` is always [`cloud_http::CloudDownloadBody::Streamed`] in practice,
/// since GCS's and Azure's zero-disk GETs never route to a ranged/spilled
/// body. The `Spilled` case is just a guard against a ranged/spilled
/// `CloudStreamingDownload` (e.g. Azure's HEAD-routed
/// `download_from_azure_streaming`) being wired in here by mistake — this
/// path can't support one.
fn open_cloud_download_stream(
    dl: cloud_http::CloudStreamingDownload,
    encryption_material: Option<EncryptionMaterial>,
    decompress: bool,
) -> Result<DownloadStreamOpen, FileManagerError> {
    let cloud_http::CloudStreamingDownload {
        cloud_byte_count,
        cse_info,
        body,
        ..
    } = dl;

    let cloud_http::CloudDownloadBody::Streamed {
        reader,
        producer_abort,
    } = body
    else {
        return Err(io::Error::other(
            "zero-disk download requires a single-GET streamed body",
        ))
        .context(IoSnafu);
    };

    Ok(spawn_download_stream_pipeline(
        reader,
        producer_abort,
        encryption_material,
        cse_info,
        decompress,
        cloud_byte_count,
    ))
}

/// Dispatches to the right cloud's [`open_s3_download_stream`] /
/// [`open_gcs_download_stream`] / [`open_azure_download_stream`] based on
/// `stage_info.location_type`. `LocationType` is exhaustive (S3/Gcs/Azure),
/// so this `match` needs no fallback arm.
pub async fn open_download_stream_for_stage(
    stage_info: &StageInfo,
    src_location: &str,
    presigned_url: Option<&str>,
    policy: &RetryPolicy,
    refresher: Option<&dyn StageInfoRefresher>,
    encryption_material: Option<EncryptionMaterial>,
    decompress: bool,
) -> Result<DownloadStreamOpen, FileManagerError> {
    match stage_info.location_type {
        LocationType::S3 => {
            open_s3_download_stream(
                stage_info,
                src_location,
                policy,
                refresher,
                encryption_material,
                decompress,
            )
            .await
        }
        LocationType::Gcs => {
            open_gcs_download_stream(
                stage_info,
                src_location,
                presigned_url,
                policy,
                refresher,
                encryption_material,
                decompress,
            )
            .await
        }
        LocationType::Azure => {
            open_azure_download_stream(
                stage_info,
                src_location,
                policy,
                refresher,
                encryption_material,
                decompress,
            )
            .await
        }
    }
}

/// Wires a live cloud body into a background `spawn_blocking` decrypt/gunzip
/// pipeline, returning the [`DownloadStreamOpen`] shared by every
/// `open_*_download_stream` variant.
///
/// Decrypt/gunzip is CPU-bound, so it runs off the async executor. The only
/// blocking points are the reader's `recv` and the writer's `blocking_send`,
/// so this task parks instead of spinning when it runs ahead of either end.
fn spawn_download_stream_pipeline<R: Read + Send + 'static>(
    reader: R,
    producer_abort: tokio::task::AbortHandle,
    encryption_material: Option<EncryptionMaterial>,
    cse_info: Option<CseDownloadInfo>,
    decompress: bool,
    cloud_byte_count: i64,
) -> DownloadStreamOpen {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, FileManagerError>>(8);

    let error_tx = tx.clone();
    let task = tokio::task::spawn_blocking(move || {
        let outcome = run_streaming_download_pipeline(
            reader,
            encryption_material,
            cse_info,
            decompress,
            ChannelWriter { tx },
        );
        if let Err(e) = outcome {
            // Consumer already disconnected (e.g. close ran first); nobody
            // is left to hear about the failure, which is fine.
            let _ = error_tx.blocking_send(Err(e));
        }
        // All Senders drop here, closing the channel — a clean EOF, not an
        // extra empty chunk.
    });

    DownloadStreamOpen {
        chunks: rx,
        task,
        producer_abort,
        cloud_byte_count,
    }
}

/// Sync `Write` counterpart to [`cloud_http::StreamReader`], over a bounded
/// channel of plaintext chunks (or a terminal error). Runs inside
/// `spawn_blocking`, so `blocking_send` is correct, not `send().await`.
struct ChannelWriter {
    tx: tokio::sync::mpsc::Sender<Result<Vec<u8>, FileManagerError>>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tx
            .blocking_send(Ok(buf.to_vec()))
            .map_err(|_| io::Error::other("download stream consumer disconnected"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Where a streaming download's plaintext goes: straight to `W`, or through
/// gunzip first. An enum, not `Box<dyn Write>` — `GzDecoder::finish` takes
/// `self` by value to flush the trailer, which a boxed trait object can't do
/// (see `code-review-design-discipline.md` #5).
enum OutputSink<W: Write> {
    Raw(W),
    Gunzip(GzDecoder<W>),
}

impl<W: Write> Write for OutputSink<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            OutputSink::Raw(w) => w.write(buf),
            OutputSink::Gunzip(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            OutputSink::Raw(w) => w.flush(),
            OutputSink::Gunzip(w) => w.flush(),
        }
    }
}

impl<W: Write> OutputSink<W> {
    /// Flushes trailing gunzip state and returns the underlying `W`. No-op for `Raw`.
    fn finish(self) -> io::Result<W> {
        match self {
            OutputSink::Raw(w) => Ok(w),
            OutputSink::Gunzip(w) => w.finish(),
        }
    }
}

/// Runs the blocking half of a cloud streaming download: reads `reader` to
/// EOF, optionally decrypting (CSE: `encryption_material` and `cse_info`
/// both present) and/or gunzipping (`decompress`), writing plaintext into
/// `output`. Mirrors `write_cloud_download`'s match arms, but writes through
/// a live channel sink instead of a file. Generic over `R` since the reader
/// is a live network body for S3, GCS, or Azure.
fn run_streaming_download_pipeline<R: Read>(
    mut reader: R,
    encryption_material: Option<EncryptionMaterial>,
    cse_info: Option<CseDownloadInfo>,
    decompress: bool,
    output: ChannelWriter,
) -> Result<(), FileManagerError> {
    let mut sink = if decompress {
        OutputSink::Gunzip(GzDecoder::new(output))
    } else {
        OutputSink::Raw(output)
    };

    match (encryption_material, cse_info) {
        // Client-side-encrypted object: decrypt the ciphertext stream,
        // verifying the SHA-256 digest at finalize time.
        (Some(enc_material), Some(CseDownloadInfo { metadata, digest })) => {
            decrypt_ciphertext_to_writer(
                &mut reader,
                &metadata,
                digest.as_str(),
                &enc_material,
                &mut sink,
            )
            .context(DecryptionSnafu)?;
        }
        // `encryption_material` present but no CSE headers (e.g. git-stage
        // objects — same handling as `download_single_file` /
        // `write_cloud_download`'s non-streaming path) — stream raw bytes.
        (maybe_enc, _) => {
            if maybe_enc.is_some() {
                tracing::debug!(
                    "encryption_material present but cloud encryption headers absent; \
                     streaming raw bytes"
                );
            }
            std::io::copy(&mut reader, &mut sink).context(IoSnafu)?;
        }
    }

    sink.finish().context(IoSnafu)?;
    Ok(())
}

/// Creates the `.part` output file for a GET download, applying owner-only
/// permissions (`0o600`) on Unix when `unsafe_file_write` is `false`.
///
/// On Unix with `unsafe_file_write = false`, forces mode `0o600`; otherwise uses the process umask.
pub(super) fn create_output_file(path: &Path, unsafe_file_write: bool) -> std::io::Result<File> {
    #[cfg(unix)]
    if !unsafe_file_write {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        // O_CREAT only sets the mode on newly-created files; if a stale .part
        // file exists its permissions are left untouched by truncate.  fchmod
        // (via set_permissions on the fd) covers that case.
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        return Ok(file);
    }
    let _ = unsafe_file_write;
    File::create(path)
}

/// Best-effort cleanup of the `<output_path>.part` temp file when an
/// atomic-rename download fails mid-stream. Logs (rather than ignoring) the
/// removal error so a subsequent disk-full failure on the same path is at
/// least diagnosable.
fn warn_remove_partial(partial_path: &Path) {
    if let Err(rm_err) = std::fs::remove_file(partial_path) {
        tracing::warn!(
            "failed to remove partial download {}: {}",
            partial_path.display(),
            rm_err
        );
    }
}

/// Aborts the task draining a download body off the network unless disarmed.
///
/// A `spawn_blocking` writer cannot be cancelled, so a dropped download future
/// would otherwise leave the producer pulling the rest of the file and the writer
/// committing it to `<dst>.part`. Aborting the producer closes the channel, which
/// stops both: the writer's next read ends the transfer instead of waiting for more
/// bytes.
///
/// How the writer *ends* differs by path, and only one of them cleans up after
/// itself. On a CSE object, `decrypt_ciphertext_to_writer` fails its digest check on
/// the truncated body, so [`write_or_cleanup`] removes `.part` — dropping the file
/// handle first, which is what makes that removal work on Windows. On a plain SSE
/// object the closed channel reads as a clean EOF ([`cloud_http::StreamReader`]
/// returns `Ok(0)`), `std::io::copy` returns `Ok`, and nothing is removed. So on SSE
/// this guard stops the fetch but leaves `.part` behind for
/// [`remove_partial_after_cancel`] to deal with.
///
/// `Drop`-based rather than registered as cleanup: the abort is synchronous, so
/// there is nothing to await, and a guard fires even when the caller had no
/// operation ctx — an internal caller, or a wrapper racing its own token above the
/// core.
struct ProducerAbortGuard(Option<tokio::task::AbortHandle>);

impl ProducerAbortGuard {
    /// `None` for a body with no producer — a spilled ranged download, or S3's
    /// in-memory GET — which makes the guard inert.
    fn arm(producer: Option<tokio::task::AbortHandle>) -> Self {
        Self(producer)
    }

    /// The body has been fully written, so the producer is finished and must not
    /// be aborted.
    fn disarm(mut self) {
        self.0 = None;
    }
}

impl Drop for ProducerAbortGuard {
    fn drop(&mut self) {
        if let Some(producer) = self.0.take() {
            // Not necessarily a cancellation: the guard is also still armed when the
            // writer returns `Err` (a disk-full or permission failure propagates
            // before the disarm). Aborting is right either way — nobody is left to
            // consume the bytes — so the message stays neutral about the cause.
            tracing::debug!("aborting download byte-stream producer (transfer did not complete)");
            producer.abort();
        }
    }
}

/// Removes a staging file left behind by a **cancelled** download, polling until it
/// is gone or the window expires.
///
/// Polls rather than removing once, because "not there" is ambiguous and both
/// readings need covering:
///
/// * **Not created yet.** The writers create the file on a detached
///   `spawn_blocking` task — `create_output_file` inside `write_cloud_download`, and
///   the setup task in `assemble_ranged_download`, which also pre-allocates it to
///   the full content length. Cancellation can reach this cleanup before that task
///   is scheduled, so returning on the first `NotFound` would let the file be
///   created afterwards with nothing left to remove it. For the ranged path that
///   orphans a full-size file.
/// * **Already gone.** A cancelled *CSE* download's writer removes its own `.part`
///   via [`write_or_cleanup`]; a plain SSE one does not (see
///   [`ProducerAbortGuard`]), so this is the only thing that removes it there.
///
/// Retrying also covers Windows, which refuses to unlink a file a detached
/// positioned-write task still holds open; Unix unlinks regardless. Residual risk on
/// Windows only: a writer that holds the handle past the whole window leaves the file
/// behind. That is stale debris, never a published partial file — the rename that
/// publishes a download runs only if the transfer future reaches it.
///
/// Bounded well inside `OperationCtx`'s cleanup wait, so a cancelled caller never
/// blocks noticeably on it.
async fn remove_partial_after_cancel(partial_path: PathBuf) {
    /// Long enough to outlast blocking-pool scheduling of the writer task and a
    /// short positioned-write on Windows, while staying far inside the
    /// operation-level cleanup budget. Note the cost of polling: a cancelled
    /// download whose staging file never appeared pays the whole window before this
    /// gives up, and `OperationCtx::run` waits for it — so keep the product small.
    const ATTEMPTS: u32 = 8;
    const BACKOFF: std::time::Duration = std::time::Duration::from_millis(25);

    for attempt in 1..=ATTEMPTS {
        let path = partial_path.clone();
        let removed = tokio::task::spawn_blocking(move || std::fs::remove_file(&path)).await;

        match removed {
            // Removed it — done.
            Ok(Ok(())) => return,
            Ok(Err(error)) if attempt < ATTEMPTS => {
                tracing::debug!(
                    path = %partial_path.display(),
                    attempt,
                    %error,
                    "staging file not removable yet (absent or locked); retrying"
                );
                tokio::time::sleep(BACKOFF).await;
            }
            // Never appeared within the window: the writer never got far enough to
            // create it, so there is nothing to clean up.
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(
                    path = %partial_path.display(),
                    "no staging file to remove after cancellation"
                );
                return;
            }
            Ok(Err(error)) => {
                tracing::warn!(
                    path = %partial_path.display(),
                    %error,
                    "failed to remove staging file after cancellation"
                );
                return;
            }
            Err(error) => {
                tracing::warn!(%error, "staging-file removal task failed");
                return;
            }
        }
    }
}

/// Write-only finalizer for a streaming (GCS / Azure) download into `partial_path`.
/// A client-side-encrypted body is decrypted (digest verified); a non-encrypting
/// (SSE) body has its raw bytes written — and a **spilled** SSE body (parallel
/// ranged GETs already assembled in the destination dir) is written to disk.
/// Does **not** rename to the output path; the caller does that in a separate
/// post-`.await` `spawn_blocking` so a dropped outer future cannot publish a
/// partially-written file. Returns the byte count written and, for the
/// `Spilled` arm, the `TempPath` for the caller to rename directly to
/// `output_path` (no intermediate `.part` rename). For all other arms `None`
/// is returned and the caller renames `partial_path` (`.part`) to `output_path`
/// via `finalize_rename`. Runs on a blocking thread.
fn write_cloud_download(
    body: CloudDownloadBody,
    cse_info: Option<CseDownloadInfo>,
    enc_material: Option<EncryptionMaterial>,
    cloud_byte_count_hint: i64,
    partial_path: &Path,
    unsafe_file_write: bool,
) -> Result<(i64, Option<tempfile::TempPath>), FileManagerError> {
    match (enc_material, cse_info) {
        // Client-side-encrypted object: decrypt (verifying the digest).
        (Some(enc_material), Some(cse)) => {
            let reader = body.into_reader().context(IoSnafu)?;
            let mut output_file =
                create_output_file(partial_path, unsafe_file_write).context(IoSnafu)?;
            let result = decrypt_ciphertext_to_writer(
                reader,
                &cse.metadata,
                &cse.digest,
                &enc_material,
                &mut output_file,
            )
            .context(DecryptionSnafu);
            write_or_cleanup(output_file, partial_path, result).map(|n| (n, None))
        }
        // enc_material present but no CSE headers on the object (sfcdigest absent) —
        // git-stage objects on Azure/GCS carry encryption key-wrap headers but no
        // sfcdigest; the download path sets cse_info=None (see cloud git-stage fix).
        // Treat as raw bytes: the spill_target was Temp (because enc_material.is_some()),
        // so hand the temp out for the caller to rename, or copy a streamed body.
        (Some(_), None) => {
            tracing::debug!(
                "enc_material present but cse_info absent; treating as raw bytes (git-stage)"
            );
            match body {
                CloudDownloadBody::Spilled(CloudSpilledBody::Temp(temp)) => {
                    Ok((cloud_byte_count_hint, Some(temp)))
                }
                CloudDownloadBody::Streamed { reader, .. } => {
                    let mut output_file =
                        create_output_file(partial_path, unsafe_file_write).context(IoSnafu)?;
                    let result = std::io::copy(&mut { reader }, &mut output_file)
                        .map(|n| n as i64)
                        .context(IoSnafu);
                    write_or_cleanup(output_file, partial_path, result).map(|n| (n, None))
                }
                CloudDownloadBody::Spilled(CloudSpilledBody::Part(_)) => {
                    // Unreachable: spill_target=Part is only chosen when enc_material=None.
                    unreachable!("Part spill with enc_material present")
                }
            }
        }
        // Non-encrypting (SSE) object — the cloud bytes are the final plaintext.
        // spill_target=Part was chosen (enc_material=None), so Temp cannot appear here.
        (None, _) => match body {
            // Non-encrypted ranged download: parallel GETs assembled straight into `.part`.
            // Nothing to copy — signal `None` so the post-await branch renames `.part`.
            CloudDownloadBody::Spilled(CloudSpilledBody::Part(_)) => {
                Ok((cloud_byte_count_hint, None))
            }
            CloudDownloadBody::Spilled(CloudSpilledBody::Temp(_)) => {
                // Unreachable: spill_target=Temp is only chosen when enc_material.is_some().
                unreachable!("Temp spill with no encryption material")
            }
            // Single buffered GET: copy the network stream to disk.
            CloudDownloadBody::Streamed { reader, .. } => {
                let mut output_file =
                    create_output_file(partial_path, unsafe_file_write).context(IoSnafu)?;
                let result = std::io::copy(&mut { reader }, &mut output_file)
                    .map(|n| n as i64)
                    .context(IoSnafu);
                write_or_cleanup(output_file, partial_path, result).map(|n| (n, None))
            }
        },
    }
}

/// Write-only finalizer used where rename is handled separately (async arm).
/// Drops the file handle and cleans up `.part` on error.
fn write_or_cleanup(
    output_file: File,
    partial: &Path,
    write_result: Result<i64, FileManagerError>,
) -> Result<i64, FileManagerError> {
    drop(output_file);
    if write_result.is_err() {
        warn_remove_partial(partial);
    }
    write_result
}

/// Atomically promotes the verified `<output>.part` temp file to its final
/// destination. On rename failure (cross-device link, destination is a
/// directory, AV holding the handle on Windows, …) the partial is
/// best-effort-removed via [`warn_remove_partial`] so a failed finalize never
/// orphans a `.part` file beside the user-visible path. The rename error is
/// returned unchanged for the caller to `.context(IoSnafu)`.
fn finalize_rename(partial_path: &Path, output_path: &Path) -> std::io::Result<()> {
    std::fs::rename(partial_path, output_path).inspect_err(|_| warn_remove_partial(partial_path))
}

/// Returns the `size` column value for a completed download, gated on the
/// active wrapper flavor. Legacy ODBC reports the on-cloud
/// (pre-decryption) byte count via `srcFileSize`; Python keeps reporting
/// the post-decryption buffer length.
fn download_result_size(
    cloud_byte_count: i64,
    output_byte_len: i64,
    flavor: &PutGetResultsetFlavor,
) -> i64 {
    match flavor {
        PutGetResultsetFlavor::Odbc => cloud_byte_count,
        _ => output_byte_len,
    }
}

// Error types for file manager operations
#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum FileManagerError {
    #[snafu(display("Failed to read or write file: {source}"))]
    Io {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to encrypt data"))]
    Encryption {
        source: EncryptionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decrypt data"))]
    Decryption {
        source: EncryptionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to compress data"))]
    Compression {
        source: CompressionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to compute file digest"))]
    DigestComputation {
        source: EncryptionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload file to S3"))]
    S3Upload {
        source: UploadFileError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to download file from S3: {source}"))]
    S3Download {
        source: DownloadFileError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload file to GCS"))]
    GcsUpload {
        source: GcsUploadError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to download file from GCS: {source}"))]
    GcsDownload {
        source: GcsDownloadError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload file to Azure"))]
    AzureUpload {
        source: AzureUploadError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to download file from Azure: {source}"))]
    AzureDownload {
        source: AzureDownloadError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to expand file paths"))]
    PathExpansion {
        source: PathExpansionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to get compression type"))]
    CompressionType {
        source: CompressionTypeError,
        #[snafu(implicit)]
        location: Location,
        backtrace: snafu::Backtrace,
    },
    #[snafu(display("File does not exist: {pattern}"))]
    NoFilesMatched {
        pattern: String,
        #[snafu(implicit)]
        location: Location,
    },
    /// A GET download was refused because the resolved output path is not a
    /// contained child of the target directory (SNOW-3663590).
    /// Kept distinct from `Io` so the refusal is discriminable.
    #[snafu(display(
        "Refusing to write GET download outside the target directory \
         (src_location={src_location:?}, local_location={local_location:?})"
    ))]
    DownloadPathRejected {
        src_location: String,
        local_location: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Blocking task failed: {source}"))]
    BlockingTask {
        source: tokio::task::JoinError,
        #[snafu(implicit)]
        location: Location,
    },
    /// One or more files in a collect-all PUT batch failed; `failures` lists
    /// each failed file and its error. See `on_download_file_error` for the
    /// fail-fast/collect-all split.
    #[snafu(display("PUT failed for {failure_count} file(s):\n{failures}"))]
    UploadBatch {
        failure_count: usize,
        failures: String,
        #[snafu(implicit)]
        location: Location,
    },
}

impl FileManagerError {
    /// Whether this is a "file/object exceeds the cloud's max-object ceiling"
    /// error — an input error `ApiError::kind()` maps to `InvalidArgument`
    /// rather than `Io`. Defined here because the cloud `*FileError`
    /// enums are private to this module.
    pub(crate) fn is_file_too_large(&self) -> bool {
        matches!(
            self,
            FileManagerError::S3Upload {
                source: UploadFileError::FileTooLarge { .. },
                ..
            } | FileManagerError::S3Download {
                source: DownloadFileError::FileTooLarge { .. },
                ..
            } | FileManagerError::GcsUpload {
                source: GcsUploadError::FileTooLarge { .. },
                ..
            } | FileManagerError::GcsDownload {
                source: GcsDownloadError::FileTooLarge { .. },
                ..
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    // ---------------------------------------------------------------
    // classify_pre_upload_skip — pure decision tests, shared by all three
    // clouds. Relocated here (from azure_transfer.rs) when the skip decision
    // was hoisted so every cloud call site could use it (SNOW-3715266).
    //
    // These bypass each cloud's HEAD-elision optimization entirely, so a
    // guard regression fails here even when the higher-level wiremock
    // scenarios in azure_transfer.rs / s3_transfer.rs / gcs_transfer.rs still
    // pass — e.g. the overwrite=true, skip_match=false, remote-digest-matches
    // case is UNREACHABLE through any `upload_to_*_or_skip` because HEAD is
    // elided in that configuration (`head_needed = !overwrite || skip_match`).
    // ---------------------------------------------------------------

    /// Mutation guard: dropping `skip_upload_on_content_match &&` from the
    /// content branch flips this to `ContentMatch` and the assertion fails.
    #[test]
    fn classify_does_not_fire_content_branch_without_opt_in() {
        let decision = classify_pre_upload_skip(
            /* overwrite */ true,
            /* skip_upload_on_content_match */ false,
            &RemoteHead::Present {
                digest: Some("abc"),
            },
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
        let decision = classify_pre_upload_skip(
            true,
            true,
            &RemoteHead::Present {
                digest: Some("abc"),
            },
            "abc",
        );
        assert_eq!(decision, SkipDecision::ContentMatch);
    }

    /// Existence wins over content-match when `!overwrite`: a remote object
    /// that exists is treated as authoritative, digest comparison is skipped.
    #[test]
    fn classify_existence_wins_under_no_overwrite() {
        let decision = classify_pre_upload_skip(
            false,
            true,
            &RemoteHead::Present {
                digest: Some("abc"),
            },
            "abc",
        );
        assert_eq!(decision, SkipDecision::Existence);
    }

    /// `!overwrite` with no remote means upload — the object doesn't exist
    /// yet. Common first-upload path.
    #[test]
    fn classify_uploads_when_remote_absent_under_no_overwrite() {
        let decision = classify_pre_upload_skip(false, false, &RemoteHead::Absent, "abc");
        assert_eq!(decision, SkipDecision::Upload);
    }

    /// `overwrite && skip_match && remote present but digest absent` — the
    /// HEAD returned 200 but no digest metadata header. Cannot compare, so
    /// upload runs (fail-open at the comparison site).
    #[test]
    fn classify_uploads_when_remote_digest_missing() {
        let decision =
            classify_pre_upload_skip(true, true, &RemoteHead::Present { digest: None }, "abc");
        assert_eq!(decision, SkipDecision::Upload);
    }

    /// `overwrite && skip_match && remote digest differs` — the racing
    /// uploader had different content; we must overwrite, not skip.
    #[test]
    fn classify_uploads_when_digests_differ() {
        let decision = classify_pre_upload_skip(
            true,
            true,
            &RemoteHead::Present {
                digest: Some("xyz"),
            },
            "abc",
        );
        assert_eq!(decision, SkipDecision::Upload);
    }

    #[cfg(unix)]
    #[test]
    fn create_output_file_uses_owner_only_mode_when_unsafe_file_write_is_false() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_owned();
        drop(tmp); // remove so create_output_file creates it fresh

        create_output_file(&path, false).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn create_output_file_uses_owner_only_mode_on_stale_part_file() {
        use std::os::unix::fs::PermissionsExt;
        // Pre-create a .part file with loose permissions to simulate a stale
        // leftover from a previous failed download.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_owned();
        drop(tmp);
        let stale = File::create(&path).unwrap();
        stale
            .set_permissions(std::fs::Permissions::from_mode(0o644))
            .unwrap();
        drop(stale);

        create_output_file(&path, false).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn create_output_file_uses_umask_mode_when_unsafe_file_write_is_true() {
        use std::os::unix::fs::PermissionsExt;

        // Baseline: mode produced by standard File::create (umask-dependent).
        let tmp_base = tempfile::NamedTempFile::new().unwrap();
        let base_path = tmp_base.path().to_owned();
        drop(tmp_base);
        File::create(&base_path).unwrap();
        let baseline_mode = std::fs::metadata(&base_path).unwrap().permissions().mode() & 0o777;

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_owned();
        drop(tmp);
        create_output_file(&path, true).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;

        // unsafe_file_write=true must use the same permissions as File::create,
        // not the forced 0o600 of the secure path.
        assert_eq!(mode, baseline_mode);
    }

    #[test]
    fn upload_result_message_odbc_skipped_uses_legacy_literal() {
        assert_eq!(
            upload_result_message(UploadStatus::Skipped, &PutGetResultsetFlavor::Odbc),
            ODBC_PUT_MESSAGE_SKIPPED,
        );
    }

    #[test]
    fn upload_result_message_python_skipped_is_empty() {
        assert_eq!(
            upload_result_message(UploadStatus::Skipped, &PutGetResultsetFlavor::Python),
            "",
        );
    }

    #[test]
    fn upload_result_message_odbc_uploaded_is_empty() {
        assert_eq!(
            upload_result_message(UploadStatus::Uploaded, &PutGetResultsetFlavor::Odbc),
            "",
        );
    }

    #[test]
    fn upload_result_message_python_uploaded_is_empty() {
        assert_eq!(
            upload_result_message(UploadStatus::Uploaded, &PutGetResultsetFlavor::Python),
            "",
        );
    }

    #[test]
    fn is_file_too_large_true_for_s3_over_ceiling_upload_and_download() {
        // `u64::MAX` is well past S3's `max_object`, so `compute_part_size`
        // yields the `FileTooLarge` inner error the transfer paths wrap.
        let inner = || {
            Box::new(
                multipart::compute_part_size(u64::MAX, &multipart::MultipartConfig::S3)
                    .unwrap_err(),
            )
        };

        let upload = FileManagerError::S3Upload {
            source: UploadFileError::FileTooLarge {
                source: inner(),
                location: Location::new(file!(), line!(), 0),
            },
            location: Location::new(file!(), line!(), 0),
        };
        assert!(upload.is_file_too_large());

        let download = FileManagerError::S3Download {
            source: DownloadFileError::FileTooLarge {
                source: inner(),
                location: Location::new(file!(), line!(), 0),
            },
            location: Location::new(file!(), line!(), 0),
        };
        assert!(download.is_file_too_large());
    }

    #[test]
    fn is_file_too_large_true_for_gcs_over_ceiling_upload_and_download() {
        // The GCS `FileTooLarge` variants carry a `detail: String` (not a boxed
        // source like S3), so build them directly. The routing that depends on
        // this — GCS file-too-large → `InvalidArgument`, not `InternalError` —
        // is what these arms exist to make correct.
        let upload = FileManagerError::GcsUpload {
            source: GcsUploadError::FileTooLarge {
                detail: "object exceeds GCS max object size".to_string(),
                location: Location::new(file!(), line!(), 0),
            },
            location: Location::new(file!(), line!(), 0),
        };
        assert!(upload.is_file_too_large());

        let download = FileManagerError::GcsDownload {
            source: GcsDownloadError::FileTooLarge {
                detail: "object exceeds GCS max object size".to_string(),
                location: Location::new(file!(), line!(), 0),
            },
            location: Location::new(file!(), line!(), 0),
        };
        assert!(download.is_file_too_large());
    }

    #[test]
    fn is_file_too_large_false_for_unrelated_errors() {
        let not_found = FileManagerError::NoFilesMatched {
            pattern: "no-such-file".to_string(),
            location: Location::new(file!(), line!(), 0),
        };
        assert!(!not_found.is_file_too_large());
    }

    fn sample_transfer_error() -> FileManagerError {
        FileManagerError::NoFilesMatched {
            pattern: "boom".to_string(),
            location: Location::new(file!(), line!(), 0),
        }
    }

    /// Builds an `UploadData` batch for a glob pattern matching every file in
    /// `dir`, targeting an S3 stage pointed at `mock` (path-style, since the
    /// endpoint host is an IP literal). `overwrite=true` skips the HEAD probe
    /// so each file goes straight to a single `PutObject`.
    fn batch_upload_data_for(
        dir: &std::path::Path,
        mock_uri: &str,
        put_fastfail: bool,
    ) -> UploadData {
        UploadData {
            src_location_pattern: dir.join("*.dat").to_string_lossy().into_owned(),
            stage_info: StageInfo {
                location_type: LocationType::S3,
                bucket: "test-bucket".to_string(),
                key_prefix: "prefix/".to_string(),
                region: "us-east-1".to_string(),
                creds: CloudCredentials::S3 {
                    aws_key_id: "AKIA-TEST".to_string(),
                    aws_secret_key: crate::sensitive::SensitiveString::from("secret".to_string()),
                    aws_token: crate::sensitive::SensitiveString::from("token".to_string()),
                },
                endpoint: Some(mock_uri.to_string()),
                presigned_url: None,
                use_virtual_url: false,
                use_regional_url: false,
                use_s3_regional_url: false,
                tls_config: crate::tls::config::TlsConfig::default(),
                crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
                proxy_config: crate::tls::config::ProxyConfig::default(),
                storage_account: None,
            },
            encryption_material: None,
            auto_compress: false,
            source_compression: SourceCompressionParam::None,
            overwrite: true,
            flavor: PutGetResultsetFlavor::Python,
            legacy_odbc_compression_autodetect: false,
            skip_upload_on_content_match: false,
            multipart: MultipartParams::default(),
            put_fastfail,
        }
    }

    /// Sets up good.dat + two failing files against a mock S3 endpoint. Uses a
    /// plain 400 (not 500) for the failures — mirrors
    /// `s3_multipart_upload_aborts_on_part_failure`'s technique for a terminal
    /// per-file failure without triggering an SDK retry storm.
    async fn setup_mixed_success_and_failure_batch(
        tmp: &tempfile::TempDir,
        put_fastfail: bool,
    ) -> (wiremock::MockServer, UploadData) {
        for (name, content) in [
            ("good.dat", "ok"),
            ("bad1.dat", "boom1"),
            ("bad2.dat", "boom2"),
        ] {
            std::fs::write(tmp.path().join(name), content).unwrap();
        }

        let mock = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path_regex(r"good\.dat$"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&mock)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path_regex(r"bad(1|2)\.dat$"))
            .respond_with(wiremock::ResponseTemplate::new(400).set_body_string("nope"))
            .mount(&mock)
            .await;

        let data = batch_upload_data_for(tmp.path(), &mock.uri(), put_fastfail);
        (mock, data)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn upload_fail_fast_propagates_error_to_abort_batch() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (_mock, data) = setup_mixed_success_and_failure_batch(&tmp, true).await;
        let policy = crate::config::retry::RetryPolicy::put_get(
            &crate::config::param_store::ParamStore::new(),
        );

        let result = upload_files(&data, &policy, TransferCtx::default()).await;

        let err = result.expect_err(
            "fail-fast must propagate the first per-file error so upload_files aborts the batch",
        );
        assert!(
            !matches!(err, FileManagerError::UploadBatch { .. }),
            "fail-fast must abort with the raw per-file error, not the collect-all aggregate: {err}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn upload_collect_all_attempts_all_then_raises_aggregate() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (_mock, data) = setup_mixed_success_and_failure_batch(&tmp, false).await;
        let policy = crate::config::retry::RetryPolicy::put_get(
            &crate::config::param_store::ParamStore::new(),
        );

        let result = upload_files(&data, &policy, TransferCtx::default()).await;

        match result {
            Err(FileManagerError::UploadBatch {
                failure_count,
                failures,
                ..
            }) => {
                assert_eq!(
                    failure_count, 2,
                    "both bad1.dat and bad2.dat must be attempted and reported, \
                     while good.dat succeeds silently"
                );
                assert!(
                    failures.contains("bad1.dat"),
                    "aggregate failure message must name bad1.dat: {failures}"
                );
                assert!(
                    failures.contains("bad2.dat"),
                    "aggregate failure message must name bad2.dat: {failures}"
                );
            }
            other => {
                panic!("collect-all must attempt every file then raise UploadBatch, got {other:?}")
            }
        }
    }

    #[test]
    fn download_fail_fast_propagates_error_to_abort_batch() {
        let result =
            on_download_file_error(true, "@stage/f.csv".to_string(), sample_transfer_error());
        assert!(
            result.is_err(),
            "fail-fast must propagate the first error so download_files aborts the batch"
        );
    }

    #[test]
    fn download_collect_all_folds_failure_into_error_row() {
        let row =
            on_download_file_error(false, "@stage/f.csv".to_string(), sample_transfer_error())
                .expect("collect-all must yield an ERROR row rather than abort");
        assert_eq!(row.status, "ERROR");
        assert_eq!(row.file, "@stage/f.csv");
        assert_eq!(row.size, 0);
        assert!(
            !row.message.is_empty(),
            "the ERROR row must carry the failure detail in its message column"
        );
    }

    #[test]
    fn odbc_put_message_skipped_matches_legacy_libsnowflakeclient() {
        // The exact string is part of the wrapper contract — every ODBC
        // application that parses the `message` column will key off this
        // value verbatim. Pinning it in a test prevents silent rewording.
        assert_eq!(
            ODBC_PUT_MESSAGE_SKIPPED,
            "File with same name already exists. SKIPPED",
        );
    }

    // BD#17 — `upload_result_source` must return the full source path
    // under `Odbc` on Windows with `\` normalised to `/` (matching the
    // legacy libsnowflakeclient wire-level value, whose `srcFileName`
    // came from the file:// URI parser and was therefore already
    // all-forward-slash), and the basename everywhere else (matching
    // the historical UD-Python behaviour).
    const WINDOWS_BACKSLASH_PATH: &str = r"C:\Users\test\test_data.csv";
    const WINDOWS_MIXED_PATH: &str = r"D:/a\universal-driver\tests\test_data.csv";
    const WINDOWS_FORWARD_SLASH_PATH: &str = "C:/Users/test/test_data.csv";
    const WINDOWS_BACKSLASH_PATH_NORMALISED: &str = "C:/Users/test/test_data.csv";
    const WINDOWS_MIXED_PATH_NORMALISED: &str = "D:/a/universal-driver/tests/test_data.csv";
    const UNIX_FULL_PATH: &str = "/home/test/test_data.csv";
    const BASENAME: &str = "test_data.csv";

    #[test]
    fn upload_result_source_windows_odbc_returns_full_path_with_forward_slashes() {
        // Pure backslash input — the form a path-like API surface might
        // produce; must be normalised to forward slashes to match legacy.
        assert_eq!(
            upload_result_source(
                &ByteSource::Path(PathBuf::from(WINDOWS_BACKSLASH_PATH)),
                BASENAME,
                &PutGetResultsetFlavor::Odbc,
                true,
            ),
            WINDOWS_BACKSLASH_PATH_NORMALISED,
        );
        // Mixed-separator input — the actual shape `glob` produces on
        // Windows when fed a file:// URI pattern (drive letter and first
        // segment as `/`, deeper segments rewritten to `\` during
        // filesystem traversal). This is the case that broke PR4 in CI.
        assert_eq!(
            upload_result_source(
                &ByteSource::Path(PathBuf::from(WINDOWS_MIXED_PATH)),
                BASENAME,
                &PutGetResultsetFlavor::Odbc,
                true,
            ),
            WINDOWS_MIXED_PATH_NORMALISED,
        );
        // Already-normalised input must be returned unchanged.
        assert_eq!(
            upload_result_source(
                &ByteSource::Path(PathBuf::from(WINDOWS_FORWARD_SLASH_PATH)),
                BASENAME,
                &PutGetResultsetFlavor::Odbc,
                true,
            ),
            WINDOWS_FORWARD_SLASH_PATH,
        );
    }

    #[test]
    fn upload_result_source_windows_python_returns_basename() {
        for full_path in [
            WINDOWS_BACKSLASH_PATH,
            WINDOWS_MIXED_PATH,
            WINDOWS_FORWARD_SLASH_PATH,
        ] {
            assert_eq!(
                upload_result_source(
                    &ByteSource::Path(PathBuf::from(full_path)),
                    BASENAME,
                    &PutGetResultsetFlavor::Python,
                    true,
                ),
                BASENAME,
                "Python on Windows must continue stripping directories from `{full_path}`",
            );
        }
    }

    #[test]
    fn upload_result_source_non_windows_returns_basename_for_both_flavors() {
        for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
            assert_eq!(
                upload_result_source(
                    &ByteSource::Path(PathBuf::from(UNIX_FULL_PATH)),
                    BASENAME,
                    &flavor,
                    false,
                ),
                BASENAME,
                "{flavor:?} on non-Windows must always return the basename — \
                 legacy ODBC's `find_last_of('/')` worked correctly on Unix paths",
            );
        }
    }

    #[test]
    fn upload_result_source_basename_only_input_unchanged_for_all_combinations() {
        // When `file_path` already equals the basename (e.g. the user
        // passed a relative single-segment path) the two branches must
        // collapse to the same value regardless of host or flavor.
        // Backslash-free input guarantees the Odbc-on-Windows
        // normalisation is a no-op here.
        for is_windows in [false, true] {
            for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
                assert_eq!(
                    upload_result_source(
                        &ByteSource::Path(PathBuf::from(BASENAME)),
                        BASENAME,
                        &flavor,
                        is_windows,
                    ),
                    BASENAME,
                    "is_windows={is_windows}, flavor={flavor:?} must return {BASENAME}",
                );
            }
        }
    }

    #[test]
    fn upload_result_source_bytes_source_falls_back_to_basename() {
        // For in-memory uploads there is no local path; even Windows ODBC
        // (the only flavor/host combo that would emit a path) must fall
        // back to the basename.
        for is_windows in [false, true] {
            for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
                assert_eq!(
                    upload_result_source(
                        &ByteSource::Bytes(Bytes::new()),
                        BASENAME,
                        &flavor,
                        is_windows,
                    ),
                    BASENAME,
                );
            }
        }
    }

    // BD#4 — `download_single_file` must report the on-cloud
    // (pre-decryption) byte count under `Odbc` (matching legacy
    // libsnowflakeclient `srcFileSize`) and the post-decryption buffer
    // length under `Python` (current UD-Python contract).
    #[test]
    fn download_result_size_odbc_uses_cloud_byte_count() {
        let cloud_byte_count = 32;
        let output_byte_len = 26;
        assert_eq!(
            download_result_size(
                cloud_byte_count,
                output_byte_len,
                &PutGetResultsetFlavor::Odbc
            ),
            cloud_byte_count,
        );
    }

    #[test]
    fn download_result_size_python_uses_output_length() {
        let cloud_byte_count = 32;
        let output_byte_len = 26;
        assert_eq!(
            download_result_size(
                cloud_byte_count,
                output_byte_len,
                &PutGetResultsetFlavor::Python,
            ),
            output_byte_len,
        );
    }

    #[test]
    fn download_result_size_sse_branches_collapse_to_same_value() {
        // For SSE stages (no client-side encryption) the cloud byte
        // count and the post-decryption buffer length are identical, so
        // both wrapper flavors must report exactly `n`.
        for n in [0, 1, 1000] {
            assert_eq!(
                download_result_size(n, n, &PutGetResultsetFlavor::Odbc),
                n,
                "Odbc flavor must report n={n} when cloud == output",
            );
            assert_eq!(
                download_result_size(n, n, &PutGetResultsetFlavor::Python),
                n,
                "Python flavor must report n={n} when cloud == output",
            );
        }
    }

    // SNOW-3663590 — GET download path guard. Layer 1 strips to a
    // basename; layer 2 confirms containment in the target dir. Mirrors
    // JDBC's `DownloadPathValidatorTest`.

    #[test]
    fn safe_download_file_name_plain_basename_passes() {
        assert_eq!(safe_download_file_name("file.csv").unwrap(), "file.csv");
    }

    #[test]
    fn safe_download_file_name_strips_forward_slash_dirs() {
        assert_eq!(safe_download_file_name("a/b/c.csv").unwrap(), "c.csv");
    }

    #[test]
    fn safe_download_file_name_strips_backslash_dirs() {
        assert_eq!(safe_download_file_name(r"a\b\c.csv").unwrap(), "c.csv");
    }

    #[test]
    fn safe_download_file_name_strips_absolute_path_to_basename() {
        // Only the basename survives, contained within `local_location`.
        assert_eq!(safe_download_file_name("/etc/passwd").unwrap(), "passwd");
    }

    #[test]
    fn safe_download_file_name_rejects_traversal_and_self_refs() {
        for bad in ["..", ".", "a/..", "a/.", "dir/"] {
            assert!(
                matches!(
                    safe_download_file_name(bad),
                    Err(FileManagerError::DownloadPathRejected { .. })
                ),
                "expected {bad:?} to be rejected",
            );
        }
    }

    #[test]
    fn safe_download_file_name_rejects_empty_and_bare_separators() {
        for bad in ["", "/", r"\"] {
            assert!(
                matches!(
                    safe_download_file_name(bad),
                    Err(FileManagerError::DownloadPathRejected { .. })
                ),
                "expected {bad:?} to be rejected",
            );
        }
    }

    #[test]
    fn safe_download_file_name_rejects_nul_and_colon() {
        // `:` guards Windows drive-letter / alternate-data-stream forms.
        for bad in ["evil\0.csv", "C:evil", "stream:ads"] {
            assert!(
                matches!(
                    safe_download_file_name(bad),
                    Err(FileManagerError::DownloadPathRejected { .. })
                ),
                "expected {bad:?} to be rejected",
            );
        }
    }

    #[test]
    fn resolve_validated_output_path_safe_name_stays_in_dir() {
        let dir = tempfile::tempdir().unwrap();
        let base = std::fs::canonicalize(dir.path()).unwrap();
        let out = resolve_validated_output_path(dir.path().to_str().unwrap(), "data.csv").unwrap();
        assert_eq!(out, base.join("data.csv"));
        assert!(out.starts_with(&base));
    }

    #[test]
    fn resolve_validated_output_path_absolute_src_cannot_escape() {
        // Server returns an absolute `src_location`; output must stay in the dir.
        let dir = tempfile::tempdir().unwrap();
        let base = std::fs::canonicalize(dir.path()).unwrap();
        let out = resolve_validated_output_path(dir.path().to_str().unwrap(), "/etc/cron.d/evil")
            .unwrap();
        assert_eq!(out, base.join("evil"));
        assert!(
            out.starts_with(&base),
            "absolute src_location must not escape the target dir: {out:?}",
        );
    }

    #[test]
    fn resolve_validated_output_path_rejects_traversal_src() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            resolve_validated_output_path(dir.path().to_str().unwrap(), "subdir/.."),
            Err(FileManagerError::DownloadPathRejected { .. })
        ));
    }

    #[test]
    fn resolve_validated_output_path_missing_dir_is_io_error() {
        // The guard still requires an existing dir (the GET flow creates it
        // upstream, SNOW-3704966); this pins the guard's standalone contract.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(matches!(
            resolve_validated_output_path(missing.to_str().unwrap(), "data.csv"),
            Err(FileManagerError::Io { .. })
        ));
    }

    // SNOW-3704966: a missing destination dir is created recursively before write.
    #[test]
    fn prepare_download_output_paths_creates_missing_dir_recursively() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("nested").join("missing");
        assert!(
            !missing.exists(),
            "precondition: destination must not exist"
        );

        let (output_path, partial_path) =
            prepare_download_output_paths(missing.to_str().unwrap(), "data.csv")
                .expect("missing destination dir must be created, not rejected");

        assert!(
            missing.is_dir(),
            "GET must create the destination directory tree"
        );
        let base = std::fs::canonicalize(&missing).unwrap();
        assert_eq!(output_path, base.join("data.csv"));
        let mut expected_partial = output_path.clone().into_os_string();
        expected_partial.push(".part");
        assert_eq!(partial_path, PathBuf::from(expected_partial));
    }

    #[test]
    fn prepare_download_output_paths_existing_dir_is_ok() {
        // create_dir_all is a no-op when the directory already exists.
        let dir = tempfile::tempdir().unwrap();
        let base = std::fs::canonicalize(dir.path()).unwrap();
        let (output_path, _partial) =
            prepare_download_output_paths(dir.path().to_str().unwrap(), "f.bin").unwrap();
        assert_eq!(output_path, base.join("f.bin"));
    }

    // Mirrors JDBC `symlinkEscapeIsRejected`: a leaf that already exists as a
    // symlink out of the target dir must be refused, not silently followed.
    #[cfg(unix)]
    #[test]
    fn resolve_validated_output_path_rejects_symlink_leaf_escape() {
        let base = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("evil.bin");
        std::fs::write(&target, b"x").unwrap();
        let link = base.path().join("data.csv");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(matches!(
            resolve_validated_output_path(base.path().to_str().unwrap(), "data.csv"),
            Err(FileManagerError::DownloadPathRejected { .. })
        ));
    }

    // BD#6 — when SOURCE_COMPRESSION=AUTO_DETECT detects an unsupported
    // compression format, legacy libsnowflakeclient silently fell back to
    // no compression. ODBC (`legacy_odbc_compression_autodetect = true`)
    // restores that behavior; Python / JDBC (false) keep surfacing the
    // error. JDBC behavior verified equivalent to Python via
    // `SnowflakeFileTransferAgent.java:3163-3308`.
    #[rustfmt::skip]
    const UNSUPPORTED_COMPRESSION_FILENAMES: &[&str] = &[
        "test.xz",
        "test.lzma",
        "test.lz",
        "test.lzo",
        "test.Z",
    ];

    #[test]
    fn auto_detect_source_compression_legacy_flag_true_swallows_unsupported_error() {
        for filename in UNSUPPORTED_COMPRESSION_FILENAMES {
            let result = auto_detect_source_compression(filename, b"", true);
            assert_eq!(
                result.unwrap(),
                CompressionType::None,
                "legacy=true must fall back to None for {filename}",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_legacy_flag_false_propagates_unsupported_error() {
        for filename in UNSUPPORTED_COMPRESSION_FILENAMES {
            let result = auto_detect_source_compression(filename, b"", false);
            assert!(
                matches!(
                    result,
                    Err(CompressionTypeError::UnsupportedCompressionType { .. })
                ),
                "legacy=false must surface the unsupported error for {filename}, got: {result:?}",
            );
        }
    }

    // Buffer-detection branch (infer crate): an extension-less file whose
    // magic bytes match an unsupported format must still trigger the
    // legacy-flag fallback. Locks in that the recovery is keyed on the
    // `UnsupportedCompressionType` error variant, not on the
    // filename-extension detection path.
    #[test]
    fn auto_detect_source_compression_legacy_flag_true_swallows_buffer_detected_unsupported() {
        let xz_magic = b"\xFD7zXZ\x00\x00\x01\x69\x22\xDE\x36";
        let result = auto_detect_source_compression("noext", xz_magic, true);
        assert_eq!(result.unwrap(), CompressionType::None);
    }

    #[test]
    fn auto_detect_source_compression_legacy_flag_false_propagates_buffer_detected_unsupported() {
        let xz_magic = b"\xFD7zXZ\x00\x00\x01\x69\x22\xDE\x36";
        let result = auto_detect_source_compression("noext", xz_magic, false);
        assert!(
            matches!(
                result,
                Err(CompressionTypeError::UnsupportedCompressionType { .. })
            ),
            "legacy=false must surface the buffer-detected unsupported error, got: {result:?}",
        );
    }

    #[test]
    fn jdbc_unsupported_autodetect_becomes_error_row() {
        let data = passthrough_upload_data("test.xz", PutGetResultsetFlavor::Jdbc, false);
        let err = preprocess_file_before_upload(ByteSource::Bytes(Bytes::new()), &data)
            .expect_err("xz must be an unsupported AUTO_DETECT codec");
        let row = on_upload_preprocess_error(&data, err)
            .expect("JDBC must fold unsupported AUTO_DETECT into an ERROR row");
        assert_eq!(
            row.status, "ERROR",
            "expected ERROR status, got {}",
            row.status
        );
        assert_eq!(row.source, "test.xz");
        assert_eq!(row.target, "test.xz");
        assert_eq!(row.source_compression, "XZ");
        assert_eq!(row.target_compression, "NONE");
        assert_eq!(
            row.message, "Copy command does not support compression type XZ.",
            "JDBC ERROR-row message must match legacy 200004 text, got {}",
            row.message
        );
    }

    #[test]
    fn auto_detect_source_compression_recognizes_gzip_for_both_flag_values() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("test.csv.gz", b"", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Gzip,
                "legacy={legacy} must still recognize supported extensions",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_returns_none_for_uncompressed_for_both_flag_values() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("test.csv", b"", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::None,
                "legacy={legacy} must report None for plain files",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_recognizes_parquet_regardless_of_flag() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("test.parquet", b"", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Parquet,
                "must recognize .parquet regardless of legacy flag (legacy={legacy})",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_recognizes_orc_regardless_of_flag() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("test.orc", b"", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Orc,
                "must recognize .orc regardless of legacy flag (legacy={legacy})",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_recognizes_parquet_magic_regardless_of_flag() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("noext", b"PAR1payload", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Parquet,
                "must recognize PAR1 magic regardless of legacy flag (legacy={legacy})",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_recognizes_orc_magic_regardless_of_flag() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("noext", b"ORCpayload", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Orc,
                "must recognize ORC magic regardless of legacy flag (legacy={legacy})",
            );
        }
    }

    // Partial-prefix detection: `\x1F\x8B` is the first 2 bytes of gzip's
    // 3-byte magic. With the legacy flag false (Python/JDBC default)
    // `infer` requires the full 3 bytes and returns `None` here. With the
    // legacy flag true (ODBC default), the short-prefix table matches
    // first and returns `Gzip`, mirroring `libsnowflakeclient`'s
    // `m_magicBytes = 2` for gzip.
    #[test]
    fn auto_detect_source_compression_legacy_flag_true_detects_2byte_gzip() {
        let two_byte_gzip: &[u8] = &[0x1F, 0x8B];
        let result = auto_detect_source_compression("noext", two_byte_gzip, true);
        assert_eq!(result.unwrap(), CompressionType::Gzip);
    }

    #[test]
    fn auto_detect_source_compression_legacy_flag_false_misses_2byte_gzip() {
        let two_byte_gzip: &[u8] = &[0x1F, 0x8B];
        let result = auto_detect_source_compression("noext", two_byte_gzip, false);
        assert_eq!(result.unwrap(), CompressionType::None);
    }

    #[test]
    fn get_source_compression_explicit_param_ignores_flag() {
        // Explicit SOURCE_COMPRESSION=<known type> never goes through the
        // auto-detect path, so the flag branch is a no-op here.
        for legacy in [false, true] {
            assert_eq!(
                get_source_compression("ignored.xz", b"", &SourceCompressionParam::Gzip, legacy)
                    .unwrap(),
                CompressionType::Gzip,
            );
            assert_eq!(
                get_source_compression("ignored.xz", b"", &SourceCompressionParam::None, legacy)
                    .unwrap(),
                CompressionType::None,
            );
        }
    }

    // Explicit SOURCE_COMPRESSION=PARQUET / =ORC short-circuits auto-detect:
    // user-specified compression is trusted, regardless of filename or
    // magic bytes. Mirrors Python `file_transfer_agent.py:1207`
    // (`current_file_compression_type = user_specified_source_compression`).
    #[test]
    fn get_source_compression_explicit_parquet_skips_autodetect() {
        for legacy in [false, true] {
            assert_eq!(
                get_source_compression(
                    "actually-not-parquet.csv",
                    b"some-csv,content",
                    &SourceCompressionParam::Parquet,
                    legacy,
                )
                .unwrap(),
                CompressionType::Parquet,
            );
        }
    }

    #[test]
    fn get_source_compression_explicit_orc_skips_autodetect() {
        for legacy in [false, true] {
            assert_eq!(
                get_source_compression(
                    "actually-not-orc.csv",
                    b"some-csv,content",
                    &SourceCompressionParam::Orc,
                    legacy,
                )
                .unwrap(),
                CompressionType::Orc,
            );
        }
    }

    // Upload-prep passthrough: a `.parquet` source under
    // `auto_compress = true` must NOT be re-wrapped in gzip. The target
    // filename keeps its original `.parquet` suffix (no `.gz` appended)
    // and `target_compression` is reported as `Parquet`. Asserting the
    // payload is bit-identical to the input distinguishes "didn't gzip"
    // from "gzipped a tiny buffer that happens to start with PAR1".
    #[test]
    fn preprocess_parquet_passthrough_under_auto_compress() {
        let payload = b"PAR1\x00\x01\x02\x03some-parquet-bytes-go-here".to_vec();
        let data = passthrough_upload_data("data.parquet", PutGetResultsetFlavor::Python, false);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.parquet", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(metadata.source_compression, CompressionType::Parquet);
        assert_eq!(
            prepared.source.byte_source().into_bytes().unwrap(),
            payload,
            "payload must pass through bit-identical (no gzip wrap)",
        );
    }

    #[test]
    fn preprocess_orc_passthrough_under_auto_compress() {
        let payload = b"ORC\x00\x01\x02some-orc-bytes-go-here".to_vec();
        let data = passthrough_upload_data("data.orc", PutGetResultsetFlavor::Python, false);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.orc", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(metadata.source_compression, CompressionType::Orc);
        assert_eq!(
            prepared.source.byte_source().into_bytes().unwrap(),
            payload,
            "payload must pass through bit-identical"
        );
    }

    // Upload-prep passthrough on the explicit-param path: when the user
    // sets `SOURCE_COMPRESSION=PARQUET` / `=ORC`, the file must NOT be
    // re-wrapped in gzip even with `auto_compress = true`. Parallels the
    // auto-detect passthrough tests above; the difference is that the
    // compression type is taken from the user param rather than sniffed
    // from filename or magic bytes.
    #[test]
    fn preprocess_parquet_passthrough_under_explicit_param() {
        let payload = b"PAR1\x00\x01\x02\x03some-parquet-bytes-go-here".to_vec();
        let data = SingleUploadData {
            source_compression: SourceCompressionParam::Parquet,
            ..passthrough_upload_data("data.parquet", PutGetResultsetFlavor::Python, false)
        };

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.parquet", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(metadata.source_compression, CompressionType::Parquet);
        assert_eq!(
            prepared.source.byte_source().into_bytes().unwrap(),
            payload,
            "payload must pass through bit-identical (no gzip wrap)",
        );
    }

    #[test]
    fn preprocess_orc_passthrough_under_explicit_param() {
        let payload = b"ORC\x00\x01\x02some-orc-bytes-go-here".to_vec();
        let data = SingleUploadData {
            source_compression: SourceCompressionParam::Orc,
            ..passthrough_upload_data("data.orc", PutGetResultsetFlavor::Python, false)
        };

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.orc", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(metadata.source_compression, CompressionType::Orc);
        assert_eq!(
            prepared.source.byte_source().into_bytes().unwrap(),
            payload,
            "payload must pass through bit-identical"
        );
    }

    // Locks in PR2 of Gap-12: parquet/orc detection is independent of the
    // unsupported-compression flag (ODBC sets the flag to true, matching
    // legacy libsnowflakeclient which detects PAR1/ORC magic via
    // FileCompressionType::PARQUET / ::ORC with isSupported=true).
    #[test]
    fn preprocess_parquet_passthrough_when_unsupported_compression_swallowed() {
        let payload = b"PAR1\x00\x01\x02\x03more-bytes".to_vec();
        let data = passthrough_upload_data("data.parquet", PutGetResultsetFlavor::Odbc, true);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.parquet");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(prepared.source.byte_source().into_bytes().unwrap(), payload);
    }

    #[test]
    fn preprocess_orc_passthrough_when_unsupported_compression_swallowed() {
        let payload = b"ORC\x00\x01\x02more-bytes".to_vec();
        let data = passthrough_upload_data("data.orc", PutGetResultsetFlavor::Odbc, true);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.orc");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(prepared.source.byte_source().into_bytes().unwrap(), payload);
    }

    // Auto-compress of a plain (not-already-compressed) payload must stream the
    // gzip output to a tempfile and adopt it as the upload source: target gains
    // a `.gz` suffix, target compression is Gzip, and the source becomes a
    // `PreparedSource::GzipTempfile` whose `_guard` keeps the tempfile alive
    // (the lazily-read source must outlive the upload). Complements the
    // end-to-end `auto_compress_then_encrypt_decrypt_decompress_roundtrip` in
    // `tests/byte_source_roundtrip.rs`.
    #[test]
    fn preprocess_auto_compress_streams_gzip_to_tempfile() {
        let payload = b"plain csv payload that is not already compressed".to_vec();
        let data = passthrough_upload_data("data.csv", PutGetResultsetFlavor::Python, false);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload)), &data).unwrap();

        assert_eq!(metadata.target, "data.csv.gz", ".gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Gzip);
        assert!(
            matches!(prepared.source, PreparedSource::GzipTempfile { .. }),
            "auto-compress must make the gzip tempfile (which carries its own \
             unlink guard) the upload source",
        );
    }

    // Prefix-read coverage: the 512-byte prefix window must be wide enough
    // to cover non-zero-offset magic bytes (e.g. tar's `ustar` at offset 257).
    // No `CompressionType` currently uses a non-zero offset, but the constant
    // is sized for future archive matchers. This test pins the contract: a
    // file larger than 512 bytes yields a prefix of exactly
    // COMPRESSION_DETECT_PREFIX_LEN bytes, and that prefix covers at least
    // offset 257 so a future matcher with magic there would see it.
    #[test]
    fn read_prefix_and_size_covers_non_zero_offset_up_to_512_bytes() {
        use std::io::Write;

        let file_len = 600usize;
        let mut data = vec![0u8; file_len];
        // Write a sentinel at offset 257 (tar's `ustar` position) and at
        // offset COMPRESSION_DETECT_PREFIX_LEN - 1 (last byte of the window).
        data[257] = 0xAA;
        data[COMPRESSION_DETECT_PREFIX_LEN - 1] = 0xBB;
        // Byte just outside the window must NOT appear in the prefix.
        data[COMPRESSION_DETECT_PREFIX_LEN] = 0xCC;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.bin");
        File::create(&path).unwrap().write_all(&data).unwrap();

        let (prefix, size) =
            read_prefix_and_size(&ByteSource::Path(path)).expect("read_prefix_and_size");

        assert_eq!(size, file_len as i64);
        assert_eq!(
            prefix.len(),
            COMPRESSION_DETECT_PREFIX_LEN,
            "prefix must be exactly COMPRESSION_DETECT_PREFIX_LEN bytes"
        );
        assert_eq!(
            prefix[257], 0xAA,
            "prefix must cover offset 257 (tar ustar position)"
        );
        assert_eq!(
            prefix[COMPRESSION_DETECT_PREFIX_LEN - 1],
            0xBB,
            "prefix must include the last byte of the window"
        );
        assert!(
            !prefix.contains(&0xCC),
            "prefix must not contain bytes beyond the window"
        );
    }

    #[test]
    fn read_prefix_and_size_bytes_source_truncates_to_window() {
        // ByteSource::Bytes also caps the prefix at COMPRESSION_DETECT_PREFIX_LEN.
        let data: Vec<u8> = (0..600u16).map(|i| (i % 251) as u8).collect();
        let (prefix, size) = read_prefix_and_size(&ByteSource::Bytes(Bytes::from(data.clone())))
            .expect("read_prefix_and_size for Bytes");

        assert_eq!(size, 600);
        assert_eq!(prefix.len(), COMPRESSION_DETECT_PREFIX_LEN);
        assert_eq!(&prefix[..], &data[..COMPRESSION_DETECT_PREFIX_LEN]);
    }

    // Determinism pin for `auto_compress = true`. The post-compression
    // SHA-256 digest is the value Snowflake stores as the remote
    // `x-ms-meta-sfcdigest` header (Azure) and the equivalent on GCS;
    // the skip-on-content-match optimization across UD and the legacy
    // Python connector compares this digest. If the gzip output is not
    // byte-stable across calls with identical input, the digest changes
    // every upload and the optimization silently never fires on the
    // default (auto_compress) path. This test pins both bytes and digest.
    #[test]
    fn preprocess_auto_compress_is_byte_deterministic_across_calls() {
        let payload = b"some payload that will be gzipped in preprocess".to_vec();
        let data = passthrough_upload_data("data.csv", PutGetResultsetFlavor::Python, false);

        let (a, meta_a) =
            preprocess_file_before_upload(ByteSource::Bytes(payload.clone().into()), &data)
                .unwrap();
        let (b, meta_b) =
            preprocess_file_before_upload(ByteSource::Bytes(payload.clone().into()), &data)
                .unwrap();

        assert_eq!(
            meta_a.target, "data.csv.gz",
            "auto_compress should produce a .gz target"
        );
        assert_eq!(meta_a.target_compression, CompressionType::Gzip);
        assert_eq!(meta_a.target, meta_b.target);
        assert_eq!(meta_a.target_compression, meta_b.target_compression);

        let bytes_a = a.source.byte_source().into_bytes().unwrap();
        let bytes_b = b.source.byte_source().into_bytes().unwrap();
        assert_eq!(
            bytes_a, bytes_b,
            "gzip output must be byte-identical across calls with the same input"
        );
        assert_eq!(
            a.digest, b.digest,
            "post-compression digest must be stable; otherwise content-match skip never fires"
        );
        assert_ne!(
            bytes_a, payload,
            "sanity: compressed bytes should differ from the raw payload (this test would be \
             vacuous on a passthrough path)"
        );
    }

    fn passthrough_upload_data(
        filename: &str,
        flavor: PutGetResultsetFlavor,
        legacy_odbc_compression_autodetect: bool,
    ) -> SingleUploadData {
        // Tests that call preprocess_file_before_upload directly pass a
        // ByteSource::Bytes so they don't depend on the filesystem.
        SingleUploadData {
            source: ByteSource::Bytes(Bytes::new()),
            filename: filename.to_string(),
            stage_info: dummy_stage_info(),
            encryption_material: None,
            auto_compress: true,
            source_compression: SourceCompressionParam::AutoDetect,
            overwrite: false,
            flavor,
            legacy_odbc_compression_autodetect,
            skip_upload_on_content_match: false,
            multipart: MultipartParams::default(),
        }
    }

    fn dummy_stage_info() -> StageInfo {
        StageInfo {
            location_type: LocationType::S3,
            bucket: "b".to_string(),
            key_prefix: "p".to_string(),
            region: "us-east-1".to_string(),
            creds: CloudCredentials::S3 {
                aws_key_id: String::new(),
                aws_secret_key: SensitiveString::from(String::new()),
                aws_token: SensitiveString::from(String::new()),
            },
            endpoint: None,
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
            proxy_config: crate::tls::config::ProxyConfig::default(),
            storage_account: None,
        }
    }

    // ---------------------------------------------------------------
    // Cross-wrapper result mapping for content-match skip
    //
    // Content-match skip under `overwrite=true` is a new path that produces
    // `(UploadStatus::Skipped, flavor)`. The `upload_result_message` unit
    // tests (above) cover the static mapping, but the END-TO-END behaviour
    // — that the path actually arrives at `Skipped` and the message column
    // gets populated correctly per wrapper — wasn't pinned. A future change
    // that splits content-match into a separate UploadStatus variant could
    // silently break the ODBC contract unless caught here.
    //
    // Drives `upload_single_file` against a wiremock Azure where HEAD
    // returns a matching digest, asserts the resulting `UploadResult` per
    // wrapper flavor.
    // ---------------------------------------------------------------

    use crate::sensitive::SensitiveString;
    use std::io::Write;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn run_content_match_skip(flavor: PutGetResultsetFlavor) -> UploadResult {
        // Real on-disk file so `upload_single_file`'s `File::open` works.
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let payload = b"hello-azure-cross-wrapper";
        // Disable auto_compress so the prepared source == payload and the digest
        // computed on the file matches what the test plants in the HEAD
        // response. With auto_compress=true the upload-prep would gzip the
        // bytes and the digest would be over the gzipped form.
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(tmp.path())
            .unwrap()
            .write_all(payload)
            .unwrap();

        let real_digest =
            compute_sha256_digest(&ByteSource::Bytes(payload.to_vec().into())).expect("digest");

        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-ms-meta-sfcdigest", real_digest.as_str()),
            )
            .mount(&mock)
            .await;
        // Load-bearing: skip must fire (no Azure block-blob PUT) for this path.
        // Path-scoped to /test-container/ so stray S3 UploadPart requests from
        // concurrent tests don't spuriously trip the expect(0) assertion.
        Mock::given(method("PUT"))
            .and(path_regex("^/test-container/"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&mock)
            .await;

        let stage_info = StageInfo {
            location_type: LocationType::Azure,
            bucket: "test-container".to_string(),
            key_prefix: "prefix/".to_string(),
            region: "eastus2".to_string(),
            creds: CloudCredentials::Azure {
                sas_token: SensitiveString::from("sv=test&sig=test&se=2099-01-01"),
            },
            endpoint: Some(mock.uri()),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
            proxy_config: crate::tls::config::ProxyConfig::default(),
            storage_account: Some("test".to_string()),
        };

        let data = SingleUploadData {
            source: ByteSource::Path(tmp.path().to_str().unwrap().into()),
            filename: "f.dat".to_string(),
            stage_info,
            encryption_material: None,
            auto_compress: false,
            source_compression: SourceCompressionParam::None,
            overwrite: true,
            flavor,
            legacy_odbc_compression_autodetect: false,
            skip_upload_on_content_match: true,
            multipart: MultipartParams::default(),
        };

        let refresher: Option<&dyn StageInfoRefresher> = None;
        let policy = RetryPolicy::put_get(&crate::config::param_store::ParamStore::new());
        upload_single_file(data, &policy, TransferCtx::new(refresher, None))
            .await
            .expect("upload_single_file should succeed against the mock")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn content_match_skip_under_odbc_emits_legacy_message() {
        let result = run_content_match_skip(PutGetResultsetFlavor::Odbc).await;
        assert_eq!(result.status, "SKIPPED");
        assert_eq!(
            result.message, ODBC_PUT_MESSAGE_SKIPPED,
            "ODBC users who set OVERWRITE=TRUE and hit content-match must get the legacy SKIPPED message",
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn content_match_skip_under_python_emits_empty_message() {
        let result = run_content_match_skip(PutGetResultsetFlavor::Python).await;
        assert_eq!(result.status, "SKIPPED");
        assert_eq!(
            result.message, "",
            "Python flavor must keep the message column empty even when content-match fires",
        );
    }

    // ---------------------------------------------------------------
    // S3 / GCS scope pin: skip_upload_on_content_match cross-cloud
    // coverage.
    //
    // SNOW-3715266 made the opt-in content-match skip uniform across all
    // three clouds (S3, Azure, GCS): under
    // `overwrite=true && skip_upload_on_content_match=true` with a matching
    // remote `sfc-digest`, the upload returns Skipped without re-uploading;
    // with the flag off it re-uploads regardless (the legacy-Python parity
    // the ticket restored for GCS). The `content_match_skip_fires_for_*`
    // and `content_match_skip_does_not_fire_for_*_without_opt_in` pairs
    // below drive that through the full `upload_single_file` dispatch path
    // (end-to-end, not just the lower-level wiremock tests in
    // `s3_transfer.rs` / `gcs_transfer.rs`) so a regression in the dispatch
    // wiring (e.g. dropping the flag from a `LocationType` match arm) is
    // caught here too.
    // ---------------------------------------------------------------

    fn write_local_payload(content: &[u8]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(tmp.path())
            .unwrap()
            .write_all(content)
            .unwrap();
        tmp
    }

    fn single_upload_data_for(
        location_type: LocationType,
        endpoint: &str,
        file_path: &str,
    ) -> SingleUploadData {
        let creds = match location_type {
            LocationType::S3 => CloudCredentials::S3 {
                aws_key_id: "AKIA-TEST".to_string(),
                aws_secret_key: SensitiveString::from("secret".to_string()),
                aws_token: SensitiveString::from("token".to_string()),
            },
            LocationType::Gcs => CloudCredentials::Gcs {
                gcs_access_token: Some(SensitiveString::from("test-bearer-token".to_string())),
            },
            LocationType::Azure => unreachable!("Azure path covered by content_match_skip tests"),
        };
        SingleUploadData {
            source: ByteSource::Path(file_path.into()),
            filename: "f.dat".to_string(),
            stage_info: StageInfo {
                location_type,
                bucket: "test-bucket".to_string(),
                key_prefix: "prefix/".to_string(),
                region: "us-east-1".to_string(),
                creds,
                endpoint: Some(endpoint.to_string()),
                presigned_url: None,
                use_virtual_url: false,
                use_regional_url: false,
                use_s3_regional_url: false,
                tls_config: crate::tls::config::TlsConfig::default(),
                crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
                proxy_config: crate::tls::config::ProxyConfig::default(),
                storage_account: None,
            },
            encryption_material: None,
            auto_compress: false,
            source_compression: SourceCompressionParam::None,
            overwrite: true,
            flavor: PutGetResultsetFlavor::Python,
            legacy_odbc_compression_autodetect: false,
            skip_upload_on_content_match: true,
            multipart: MultipartParams::default(),
        }
    }

    /// End-to-end (dispatch-level) pin for SNOW-3715266: under
    /// `overwrite=true && skip_upload_on_content_match=true`, when the
    /// remote object's `sfc-digest` metadata already matches the local
    /// digest, S3 must short-circuit to `Skipped` without ever issuing the
    /// PUT — driven through the full `upload_single_file` dispatch path
    /// (not just the lower-level `s3_transfer.rs` unit tests), so the
    /// `LocationType::S3` match arm wiring the flag through stays honest.
    #[tokio::test(flavor = "multi_thread")]
    async fn content_match_skip_fires_for_s3() {
        let payload = b"hello-s3-content-match";
        let real_digest =
            encryption::compute_sha256_digest(&ByteSource::Bytes(payload.to_vec().into()))
                .expect("digest");

        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-amz-meta-sfc-digest", real_digest.as_str()),
            )
            .mount(&mock)
            .await;
        // Load-bearing: skip must fire (no PutObject) for this path.
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock)
            .await;

        let tmp = write_local_payload(payload);
        let mut data =
            single_upload_data_for(LocationType::S3, &mock.uri(), tmp.path().to_str().unwrap());
        // Set the flag explicitly (rather than leaning on the builder default)
        // so this pin stays meaningful if that default ever changes — mirrors
        // the companion `..._without_opt_in` test.
        data.skip_upload_on_content_match = true;

        let refresher: Option<&dyn StageInfoRefresher> = None;
        let policy = crate::config::retry::RetryPolicy::put_get(
            &crate::config::param_store::ParamStore::new(),
        );
        let result = upload_single_file(data, &policy, TransferCtx::new(refresher, None))
            .await
            .expect("S3 upload should succeed against the mock");
        assert_eq!(result.status, "SKIPPED");
    }

    /// Companion pin: with `skip_upload_on_content_match=false` (default),
    /// S3 must upload even though `overwrite=true` and the remote object
    /// exists — the opt-in flag, not mere existence, gates the S3
    /// content-match branch (the opt-in semantics now shared by all three
    /// clouds).
    #[tokio::test(flavor = "multi_thread")]
    async fn content_match_skip_does_not_fire_for_s3_without_opt_in() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock)
            .await;

        let tmp = write_local_payload(b"hello-s3-no-op");
        let mut data =
            single_upload_data_for(LocationType::S3, &mock.uri(), tmp.path().to_str().unwrap());
        data.skip_upload_on_content_match = false;

        let refresher: Option<&dyn StageInfoRefresher> = None;
        let policy = RetryPolicy::put_get(&crate::config::param_store::ParamStore::new());
        let result = upload_single_file(data, &policy, TransferCtx::new(refresher, None))
            .await
            .expect("S3 upload should succeed against the mock");
        assert_eq!(result.status, "UPLOADED");
    }

    /// End-to-end (dispatch-level) pin for SNOW-3715266: GCS now honors the
    /// opt-in flag exactly like S3/Azure. Under
    /// `overwrite=true && skip_upload_on_content_match=true` with a matching
    /// remote `x-goog-meta-sfc-digest`, GCS must short-circuit to `Skipped`
    /// without issuing the PUT — driven through the full `upload_single_file`
    /// dispatch path so the `LocationType::Gcs` arm wiring the flag through
    /// stays honest.
    #[tokio::test(flavor = "multi_thread")]
    async fn content_match_skip_fires_for_gcs() {
        let payload = b"hello-gcs-content-match";
        let real_digest =
            encryption::compute_sha256_digest(&ByteSource::Bytes(payload.to_vec().into()))
                .expect("digest");

        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-meta-sfc-digest", real_digest.as_str()),
            )
            .mount(&mock)
            .await;
        // Load-bearing: skip must fire (no PUT) for this path.
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock)
            .await;

        let tmp = write_local_payload(payload);
        let data =
            single_upload_data_for(LocationType::Gcs, &mock.uri(), tmp.path().to_str().unwrap());

        let refresher: Option<&dyn StageInfoRefresher> = None;
        let policy = crate::config::retry::RetryPolicy::put_get(
            &crate::config::param_store::ParamStore::new(),
        );
        let result = upload_single_file(data, &policy, TransferCtx::new(refresher, None))
            .await
            .expect("GCS upload should succeed against the mock");
        assert_eq!(result.status, "SKIPPED");
    }

    /// Companion pin (the core SNOW-3715266 regression): with
    /// `skip_upload_on_content_match=false` (default) and `overwrite=true`,
    /// GCS re-uploads. What this proves is that the `LocationType::Gcs`
    /// dispatch arm threads the flag through, so `head_needed` is false and
    /// the content-match skip is elided — pinned by the `HEAD .expect(0)` +
    /// `PUT .expect(1)` mocks (no digest is mounted, fetched, or compared).
    /// GCS previously skipped here unconditionally, diverging from legacy
    /// Python; this pins the restored parity.
    #[tokio::test(flavor = "multi_thread")]
    async fn content_match_skip_does_not_fire_for_gcs_without_opt_in() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock)
            .await;

        let tmp = write_local_payload(b"hello-gcs-no-op");
        let mut data =
            single_upload_data_for(LocationType::Gcs, &mock.uri(), tmp.path().to_str().unwrap());
        data.skip_upload_on_content_match = false;

        let refresher: Option<&dyn StageInfoRefresher> = None;
        let policy = crate::config::retry::RetryPolicy::put_get(
            &crate::config::param_store::ParamStore::new(),
        );
        let result = upload_single_file(data, &policy, TransferCtx::new(refresher, None))
            .await
            .expect("GCS upload should succeed against the mock");
        assert_eq!(result.status, "UPLOADED");
    }

    /// The cleanup must not give up the first time the staging file is absent.
    ///
    /// Both download writers create it on a *detached* blocking task, so a cancel can
    /// reach this cleanup before that task is scheduled. Treating the first `NotFound`
    /// as "already removed" orphaned the file created a moment later — and for a ranged
    /// download that file has already been pre-allocated to the object's full length.
    #[tokio::test(flavor = "multi_thread")]
    async fn remove_partial_after_cancel_removes_a_staging_file_that_appears_late() {
        let dir = tempfile::tempdir().expect("tempdir");
        let partial = dir.path().join("late-arrival.part");

        // Appears well after the first poll, standing in for a writer task that had
        // not yet been scheduled when the cancel landed.
        let late = partial.clone();
        let writer = tokio::task::spawn_blocking(move || {
            std::thread::sleep(std::time::Duration::from_millis(60));
            std::fs::write(&late, b"partially written bytes").expect("create staging file");
        });

        remove_partial_after_cancel(partial.clone()).await;
        writer.await.expect("writer task panicked");

        assert!(
            !partial.exists(),
            "a staging file created after the cleanup started must still be removed"
        );
    }

    /// The absent-forever case must settle quietly rather than warn: a cancel that
    /// lands before any byte was written has nothing to clean up.
    #[tokio::test(flavor = "multi_thread")]
    async fn remove_partial_after_cancel_is_quiet_when_no_staging_file_is_ever_created() {
        let dir = tempfile::tempdir().expect("tempdir");
        let absent = dir.path().join("never-created.part");

        remove_partial_after_cancel(absent.clone()).await;

        assert!(!absent.exists(), "nothing should be created by the cleanup");
    }
}
