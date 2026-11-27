//! Helpers shared by the GCS, Azure, and (partially) S3 transfer paths.
//!
//! S3 skips the reqwest-specific retry loop (`upload_with_retry`) and uses
//! the AWS SDK's own retry/backoff instead. But it reuses the
//! async-stream → sync-`Read` bridge (`StreamReader`) via its own producer,
//! `spawn_s3_byte_stream_producer`, since `ByteStream::next` isn't a
//! `futures_core::Stream` and needs its own drain loop.

use super::encryption::Encryptor;
use super::types::{ByteSource, EncryptedFileMetadata, LocationType, StageInfo, StageTransport};
use crate::apis::operation_ctx::{CleanupScope, with_cleanup_scope_opt};
use crate::config::retry::{BackoffConfig, RetryPolicy};
use crate::governor::{Deadline, Remaining, RequestPermit};
use crate::log_foreign_error;
use crate::tls::error::{ClientBuildSnafu, TlsError};
use bytes::Bytes;
use futures::StreamExt as _;
use futures::stream::Stream;
use reqwest::StatusCode;
use snafu::ResultExt as _;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tempfile::NamedTempFile;

/// Sleeps `delay` if it fits in `deadline`; otherwise fails immediately so a
/// doomed backoff cannot occupy a permit until the budget expires.
pub(super) async fn sleep_within_deadline<E>(
    deadline: Deadline,
    delay: Duration,
    on_deadline: impl FnOnce(Duration, Duration) -> E,
) -> Result<(), E> {
    match deadline.remaining() {
        Remaining::Expired { elapsed, budget } => Err(on_deadline(elapsed, budget)),
        Remaining::Left(remaining) if delay >= remaining => Err(on_deadline(
            deadline.elapsed(),
            deadline.budget().unwrap_or(remaining),
        )),
        Remaining::Unlimited | Remaining::Left(_) => {
            tokio::time::sleep(delay).await;
            Ok(())
        }
    }
}

/// Shrinks `policy.max_elapsed` to time left on `deadline`. `None` if expired.
pub(super) fn remaining_policy(policy: &RetryPolicy, deadline: Deadline) -> Option<RetryPolicy> {
    let mut remaining_policy = policy.clone();
    match deadline.remaining() {
        Remaining::Unlimited => Some(remaining_policy),
        Remaining::Left(left) => {
            remaining_policy.max_elapsed = Some(left);
            Some(remaining_policy)
        }
        Remaining::Expired { .. } => None,
    }
}

/// Read-buffer size in bytes for the streaming upload producer — one channel chunk.
const UPLOAD_CHUNK_SIZE_BYTES: usize = 64 * 1024;

/// Per-attempt HTTP timeout. Matches the cloud transfer modules' historical
/// 300s cap; the retry budget (`policy.max_elapsed`) must exceed this so at
/// least one full attempt can complete.
pub(super) const REQUEST_TIMEOUT_SECS: u64 = 300;

/// Returns true when the HTTP status code should trigger a retry. Mirrors
/// `http::retry::should_retry_status` — kept inline so the cloud transfer
/// modules don't take an indirect dep just for the constant set.
pub(super) fn is_retryable_status(status: u16, extra: &BTreeSet<u16>) -> bool {
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504) || extra.contains(&status)
}

/// Computes the next backoff delay, clamping to `backoff.cap`.
pub(super) fn next_delay_ms(current: f64, backoff: &BackoffConfig) -> f64 {
    let next = current * backoff.factor;
    next.min(backoff.cap.as_millis() as f64)
}

/// The Azure/GCS client: a client-level request timeout, HTTP/1.1 only.
pub(super) async fn build_azure_gcs_client(
    transport: &StageTransport,
) -> Result<reqwest::Client, TlsError> {
    crate::tls::client::configure_storage_client_builder_prepared(
        crate::tls::client::apply_http_pool_settings(
            reqwest::Client::builder().timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
        ),
        &transport.tls_config,
        Some(&transport.proxy_config),
        transport.crl_manager.clone(),
        transport.root_store_provider.clone(),
    )
    .await?
    .http1_only()
    .build()
    .context(ClientBuildSnafu)
}

/// The S3 client leaves request timeouts to the AWS SDK, so it cannot share a
/// client with Azure/GCS.
#[derive(Clone)]
pub(crate) enum StorageHttpClient {
    S3(crate::tls::aws_http_client::AwsSdkReqwestClient),
    Reqwest(reqwest::Client),
    /// An XP-backend transfer goes through
    /// `SnowflakeBackend::upload_stream`/`download_stream`, so it holds no
    /// HTTP client.
    XpBackend,
}

impl StorageHttpClient {
    pub(crate) async fn for_stage(stage_info: &StageInfo) -> Result<Self, TlsError> {
        Self::build(stage_info.location_type, &stage_info.transport).await
    }

    async fn build(
        location_type: LocationType,
        transport: &StageTransport,
    ) -> Result<Self, TlsError> {
        Ok(match location_type {
            LocationType::Azure | LocationType::Gcs => {
                Self::Reqwest(build_azure_gcs_client(transport).await?)
            }
            LocationType::S3 => Self::S3(
                crate::tls::aws_http_client::AwsSdkReqwestClient::build_prepared(
                    &transport.tls_config,
                    Some(&transport.proxy_config),
                    transport.crl_manager.clone(),
                    transport.root_store_provider.clone(),
                )
                .await?,
            ),
            LocationType::XpBackend => Self::XpBackend,
        })
    }

    pub(crate) fn s3(&self) -> Option<&crate::tls::aws_http_client::AwsSdkReqwestClient> {
        match self {
            Self::S3(client) => Some(client),
            Self::Reqwest(_) | Self::XpBackend => None,
        }
    }

    pub(crate) fn reqwest(&self) -> Option<&reqwest::Client> {
        match self {
            Self::Reqwest(client) => Some(client),
            Self::S3(_) | Self::XpBackend => None,
        }
    }
}

/// The storage HTTP client a connection reuses across its PUT/GET commands, so
/// every command after the first rides pooled TCP/TLS connections to the stage
/// instead of handshaking again. Cloning shares the cache.
///
/// Clients are built from the first command's `StageTransport`; a transport
/// change needs a new cache. A connection's internal stages live in one cloud,
/// so one slot suffices; a stage in another cloud gets an uncached client.
#[derive(Clone)]
pub(crate) struct StorageHttpClientCache(Arc<CacheInner>);

struct CacheInner {
    client: tokio::sync::OnceCell<(LocationType, StorageHttpClient)>,
}

impl StorageHttpClientCache {
    pub(crate) fn new() -> Self {
        Self(Arc::new(CacheInner {
            client: tokio::sync::OnceCell::new(),
        }))
    }

    pub(crate) async fn get(
        &self,
        location_type: LocationType,
        transport: &StageTransport,
    ) -> Result<StorageHttpClient, TlsError> {
        let (cached_type, cached) = self
            .0
            .client
            .get_or_try_init(|| async {
                Ok::<_, TlsError>((
                    location_type,
                    StorageHttpClient::build(location_type, transport).await?,
                ))
            })
            .await?;
        if *cached_type == location_type {
            Ok(cached.clone())
        } else {
            StorageHttpClient::build(location_type, transport).await
        }
    }
}

/// Reads a non-2xx response body for inclusion in error messages. Always
/// succeeds — read errors fold into a placeholder string.
pub(super) async fn read_error_body(response: reqwest::Response) -> String {
    match response.text().await {
        Ok(text) => text,
        Err(e) => {
            log_foreign_error!(warn, e, "Failed to read cloud error response body");
            format!("<could not read body: {}>", e)
        }
    }
}

/// Runs `run` under a permit the caller already holds. The remaining retry
/// budget is taken from `deadline`; `_held` is unused proof of that permit.
pub(super) async fn run_within_deadline<T, E, F, Fut, D>(
    policy: &RetryPolicy,
    deadline: Deadline,
    _held: &RequestPermit,
    on_deadline: D,
    run: F,
) -> Result<T, E>
where
    F: FnOnce(RetryPolicy) -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    D: Fn(Duration, Duration) -> E,
{
    let timeout = match deadline.remaining() {
        Remaining::Unlimited => None,
        Remaining::Left(remaining) => Some(remaining),
        Remaining::Expired { elapsed, budget } => {
            return Err(on_deadline(elapsed, budget));
        }
    };
    let remaining = remaining_policy(policy, deadline).ok_or_else(|| {
        on_deadline(
            deadline.elapsed(),
            deadline.budget().unwrap_or_else(|| deadline.elapsed()),
        )
    })?;
    let future = run(remaining);
    match timeout {
        Some(timeout) => match tokio::time::timeout(timeout, future).await {
            Ok(result) => result,
            Err(_) => Err(on_deadline(
                deadline.elapsed(),
                deadline.budget().unwrap_or(timeout),
            )),
        },
        None => future.await,
    }
}

/// Runs `run` under a caller-owned permit and returns that permit with the
/// body so the caller can move it into the producer.
#[cfg(test)]
pub(super) async fn admitted_request_deferred_body<T, E, F, Fut, D>(
    policy: &RetryPolicy,
    deadline: Deadline,
    permit: RequestPermit,
    on_deadline: D,
    run: F,
) -> Result<(T, RequestPermit), E>
where
    F: FnOnce(RetryPolicy) -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    D: Fn(Duration, Duration) -> E,
{
    let remaining = remaining_policy(policy, deadline).ok_or_else(|| {
        on_deadline(
            deadline.elapsed(),
            deadline.budget().unwrap_or_else(|| deadline.elapsed()),
        )
    })?;
    let value = run(remaining).await?;
    Ok((value, permit))
}

/// Sync `Read` adapter over a bounded tokio mpsc channel of `reqwest::Bytes`
/// results. `read` blocks via `blocking_recv`, which is safe because
/// `StreamReader` only ever runs inside `spawn_blocking` (see `mod.rs`), never
/// on an async worker thread. Bridges an async `bytes_stream()` into the sync
/// decryption path. `buf` is the unconsumed tail of the last chunk;
/// advancing it is an O(1) refcount bump, no allocation.
///
/// `bytes_read` accumulates the running total of ciphertext (on-cloud,
/// pre-decryption) bytes pulled out of the stream. It is shared via
/// [`StreamReader::bytes_read_handle`] so the caller can recover the on-cloud
/// byte count after the reader is consumed — needed when the `Content-Length`
/// header is absent (chunked transfer encoding) and the decrypted plaintext
/// length would otherwise be misreported as the on-cloud size.
pub struct StreamReader {
    rx: tokio::sync::mpsc::Receiver<std::io::Result<Bytes>>,
    buf: Bytes,
    bytes_read: Arc<AtomicU64>,
}

impl StreamReader {
    pub(super) fn new(rx: tokio::sync::mpsc::Receiver<std::io::Result<Bytes>>) -> Self {
        Self {
            rx,
            buf: Bytes::new(),
            bytes_read: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Shared handle to the running total of ciphertext bytes read out of the
    /// stream so far. Clone it *before* moving the reader into a
    /// `spawn_blocking` decrypt task, then `load` it after the task joins to
    /// recover the on-cloud (pre-decryption) byte count. This is the correct
    /// `cloud_byte_count` when `Content-Length` is absent — unlike the
    /// decrypted plaintext length, it counts the actual wire bytes.
    pub fn bytes_read_handle(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.bytes_read)
    }
}

impl std::io::Read for StreamReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.buf.is_empty() {
            // Safe: every StreamReader runs inside spawn_blocking (see struct doc).
            match self.rx.blocking_recv() {
                Some(Ok(chunk)) => self.buf = chunk,
                Some(Err(e)) => return Err(e),
                None => return Ok(0),
            }
        }
        let n = self.buf.len().min(out.len());
        out[..n].copy_from_slice(&self.buf[..n]);
        self.buf = self.buf.slice(n..); // O(1): bumps the range, no allocation
        self.bytes_read.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}

/// Admitted reqwest producer: drains `response.bytes_stream()` into a bounded
/// mpsc channel and owns the GET permit until EOF, error, consumer disconnect,
/// or task abort. Returns the [`StreamReader`] plus an abort handle so a
/// stalled connection can be cancelled instead of parking forever on
/// `bytes_stream().next()`. Channel capacity is 8 chunks (≈2 MiB at typical
/// 256 KiB chunks) — enough to keep the producer busy while the consumer
/// decrypts.
///
/// NOTE: the retry/backoff loop upstream (`gcs_get_with_refresh` /
/// `azure_request_with_retry`) covers only up to the point where response
/// *headers* are received. Once the response is in hand and we begin polling
/// `bytes_stream()` here, a mid-body transport failure (TCP RST, TLS read
/// error, proxy idle-timeout) propagates to the consumer as
/// `io::Error::other(...)` and tears down the decrypt with **no retry and no
/// Range-resume**. This is a deliberate behaviour change vs. the buffered
/// download path, which collected the full body inside the retry loop and so
/// could retry a mid-body failure. Acceptable within the gap-4 streaming
/// scope; revisit if Range-resume becomes a requirement. The
/// `gcs_streaming_mid_body_disconnect_surfaces_error` test pins this
/// behaviour.
///
/// On the zero-disk `download_stream_*` path an idle consumer holds a permit
/// until the stream is closed or reaped.
pub(super) fn spawn_admitted_byte_stream_producer(
    response: reqwest::Response,
    permit: RequestPermit,
) -> (StreamReader, tokio::task::AbortHandle) {
    let (tx, rx) = tokio::sync::mpsc::channel::<std::io::Result<Bytes>>(8);
    let stream = response.bytes_stream();
    let join_handle = tokio::spawn(async move {
        let _permit = permit;
        let mut stream = stream;
        while let Some(chunk_result) = stream.next().await {
            let mapped = chunk_result.map_err(std::io::Error::other);
            if let Err(send_err) = tx.send(mapped).await {
                if send_err.0.is_err() {
                    tracing::debug!(
                        "byte-stream producer: consumer disconnected with pending error: {:?}",
                        send_err.0
                    );
                }
                break;
            }
        }
        // tx is dropped here, signalling EOF to the receiver.
    });
    let abort_handle = join_handle.abort_handle();
    (StreamReader::new(rx), abort_handle)
}

/// S3 analogue of [`spawn_admitted_byte_stream_producer`]: drains an
/// `aws_sdk_s3::primitives::ByteStream` into a bounded mpsc channel and
/// returns a [`StreamReader`] plus a [`tokio::task::AbortHandle`] so a caller
/// (`download_stream_close`, via `DownloadStreamOpen::producer_abort`) can
/// cancel a producer stalled mid-read on `body.next()`. Needs its own drain
/// loop because `ByteStream::next` isn't a `futures_core::Stream`, but feeds
/// the same `StreamReader` as the GCS/Azure paths.
///
/// Same tradeoff as the GCS/Azure producer: the AWS SDK's retry only covers
/// opening the GET. Once the body is in hand, a mid-body failure surfaces to
/// the consumer as `io::Error::other(...)` with no retry and no Range-resume.
///
/// The admitted GET permit is owned until EOF, error, consumer disconnect, or
/// task abort.
pub(super) fn spawn_s3_byte_stream_producer(
    mut body: aws_sdk_s3::primitives::ByteStream,
    permit: RequestPermit,
) -> (StreamReader, tokio::task::AbortHandle) {
    let (tx, rx) = tokio::sync::mpsc::channel::<std::io::Result<Bytes>>(8);
    let join_handle = tokio::spawn(async move {
        let _hold = permit;
        while let Some(chunk_result) = body.next().await {
            let mapped = chunk_result.map_err(std::io::Error::other);
            if let Err(send_err) = tx.send(mapped).await {
                if send_err.0.is_err() {
                    tracing::debug!(
                        "S3 byte-stream producer: consumer disconnected with pending error: {:?}",
                        send_err.0
                    );
                }
                break;
            }
        }
        // tx is dropped here, signalling EOF to the receiver.
    });
    let abort_handle = join_handle.abort_handle();
    (StreamReader::new(rx), abort_handle)
}

/// CSE decrypt inputs. `digest` is verified after decrypt when present;
/// objects written by a server-side unload carry key-wrap headers without
/// `sfc-digest`.
pub struct CseDownloadInfo {
    pub metadata: EncryptedFileMetadata,
    pub digest: Option<String>,
}

impl CseDownloadInfo {
    pub(super) fn from_headers(
        file_metadata: Option<EncryptedFileMetadata>,
        digest: Option<String>,
    ) -> Option<Self> {
        file_metadata.map(|metadata| Self { metadata, digest })
    }
}

/// Result of a streaming download from a reqwest-based cloud transport.
///
/// Unifies the GCS and Azure shapes — both produce identical fields, only
/// the upstream header parsing differs (and is handled before constructing
/// this struct).
///
/// Marked `pub` so the cfg-gated `file_manager::internal` re-export can
/// surface it to integration tests; the parent module `cloud_http` is itself
/// private, so this is not part of the crate's public API.
pub struct CloudStreamingDownload {
    /// On-cloud (pre-decryption) byte count from the `Content-Length` header.
    /// May be 0 when the header is absent (e.g. chunked transfer encoding);
    /// callers fall back to the running total from
    /// [`StreamReader::bytes_read_handle`] in that case, which still counts
    /// on-cloud ciphertext bytes (not the decrypted plaintext length).
    pub cloud_byte_count: i64,
    /// CSE key-wrap metadata and optional digest as the cloud returned them;
    /// `None` when the object has no encryption headers. Whether the body is
    /// decrypted is decided against the GET's encryption material in
    /// `file_manager`, since git-stage objects carry the same headers.
    pub cse_info: Option<CseDownloadInfo>,
    /// Running total of on-cloud (pre-decryption) ciphertext bytes pulled off
    /// the wire. `load` it after the decrypt task joins to recover the
    /// `cloud_byte_count` when `Content-Length` was absent (chunked transfer
    /// encoding) — it counts wire bytes, not the misleading decrypted plaintext
    /// length. Because the body is now type-erased behind [`CloudDownloadBody`],
    /// this handle is surfaced here rather than via
    /// `StreamReader::bytes_read_handle`. For a tempfile-backed ranged download
    /// the size is already known from the HEAD `Content-Length`, so this stays 0
    /// and the hint is used instead.
    pub cloud_bytes_read: Arc<AtomicU64>,
    /// Download body — a live network stream (single GET, carrying its own
    /// producer abort handle) or a spilled tempfile (parallel ranged GETs,
    /// already complete — nothing left to abort). On the SSE / non-decrypting
    /// path a `Spilled` body is renamed into place rather than copied; see
    /// `download_single_file`. Both consumers use the `Streamed` abort handle to stop
    /// a producer that is no longer wanted: the zero-disk `download_stream_*` RPC path
    /// (`file_manager::open_cloud_download_stream`) when a stalled stream is closed,
    /// and the buffered `download_single_file` path via `ProducerAbortGuard` when the
    /// transfer is cancelled.
    pub body: CloudDownloadBody,
}

impl CloudStreamingDownload {
    /// Transfers an admitted GET permit into the response-body producer.
    pub(super) fn from_admitted_reqwest_response(
        response: reqwest::Response,
        permit: RequestPermit,
        digest: Option<String>,
        file_metadata: Option<EncryptedFileMetadata>,
    ) -> Self {
        let cloud_byte_count = response.content_length().unwrap_or(0) as i64;
        let cse_info = CseDownloadInfo::from_headers(file_metadata, digest);
        let (reader, producer_abort) = spawn_admitted_byte_stream_producer(response, permit);
        let cloud_bytes_read = reader.bytes_read_handle();
        Self {
            cloud_byte_count,
            cse_info,
            cloud_bytes_read,
            body: CloudDownloadBody::Streamed {
                reader: Box::new(reader),
                producer_abort,
            },
        }
    }
}

/// Where a streaming download's bytes live. Mirrors S3's `S3DownloadBody`: a
/// network stream must be read (and, for SSE, copied) into the destination, but
/// a spilled tempfile — which the parallel ranged GETs already assembled in the
/// destination directory — can be renamed into place with no extra full-file
/// copy.
pub enum CloudDownloadBody {
    /// Live reqwest byte stream from a single GET (below the multipart
    /// threshold), paired with the abort handle for the task draining it —
    /// carried here (rather than as a sibling `Option` on
    /// [`CloudStreamingDownload`]) so a stalled producer can only be aborted
    /// when there is actually a producer task to abort.
    Streamed {
        reader: Box<dyn Read + Send>,
        producer_abort: tokio::task::AbortHandle,
    },
    /// File assembled by parallel ranged GETs, living in the destination dir.
    Spilled(CloudSpilledBody),
}

/// A ranged cloud download assembled to disk, shared by S3, Azure, and GCS.
/// The two shapes differ only in who owns the file and how it is finalized:
///
/// * `Part` — a non-encrypted (SSE) download assembled straight into the
///   caller's `<dst>.part` staging file. The bytes are already the final
///   plaintext, so the caller just renames `.part` to the destination (a single
///   same-FS rename). Any leftover after a hard kill is a self-documenting,
///   self-overwriting `.part`, never random debris.
/// * `Temp` — a client-side-encrypted (or git-stage) download assembled into a
///   throwaway RAII temp. CSE bytes are ciphertext that must still be decrypted
///   into `.part`, so they cannot land in `.part` directly; the temp is unlinked
///   on drop once consumed.
pub enum CloudSpilledBody {
    Part(PathBuf),
    Temp(tempfile::TempPath),
}

/// Where a ranged cloud download should assemble its bytes. Chosen by the caller
/// (which knows whether the object is client-side-encrypted) and threaded down to
/// the per-cloud `*_range_download`. `Copy` so it can be handed to each retry.
#[derive(Clone, Copy)]
pub enum CloudSpillTarget<'a> {
    /// Non-encrypted download: assemble directly into this `<dst>.part` file. Its
    /// removal on cancellation is registered by `download_single_file`, which owns
    /// the path.
    Part(&'a Path),
    /// Encrypted / git-stage download: assemble ciphertext into a temp in `dir`
    /// (kept on the destination's filesystem so the later finalize is a same-FS
    /// rename, not a cross-device copy).
    Temp {
        dir: &'a Path,
        /// Where [`assemble_ranged_download`] registers removal of the temp it
        /// mints. Carried here rather than as another parameter because `target`
        /// already threads from `download_single_file` — which holds the scope —
        /// down to the assembly, which is the only place the temp's path exists.
        ///
        /// `NamedTempFile`'s own `Drop` unlinks it, but that is a single
        /// best-effort `remove_file` whose error is discarded: on Windows it fails
        /// while a detached positioned-write task still holds the handle, and the
        /// random name means repeated cancels accumulate debris beside the user's
        /// downloads rather than overwriting one `.part`.
        cleanup: Option<&'a CleanupScope>,
    },
}

impl CloudDownloadBody {
    /// Handle for the task draining this body off the network, when there is one.
    ///
    /// Cloned so an async owner can keep it past [`Self::into_reader`], which drops the
    /// original. Aborting it is how a cancelled download stops pulling bytes — see
    /// `ProducerAbortGuard`. `None` for a `Spilled` body, whose ranged GETs have
    /// already finished.
    pub(super) fn producer_abort(&self) -> Option<tokio::task::AbortHandle> {
        match self {
            CloudDownloadBody::Streamed { producer_abort, .. } => Some(producer_abort.clone()),
            CloudDownloadBody::Spilled(_) => None,
        }
    }

    /// A uniform blocking `Read` over the body: the network stream directly, or
    /// a reader over the spilled file (a [`SpilledReader`](super::multipart::SpilledReader)
    /// that keeps a `Temp` alive for the read's duration, or a plain file for a
    /// `Part`).
    pub fn into_reader(self) -> std::io::Result<Box<dyn Read + Send>> {
        match self {
            CloudDownloadBody::Streamed { reader, .. } => Ok(reader),
            // The decrypt/copy step only reads a spilled body for CSE, which is
            // always a `Temp`; a `Part` body is the final plaintext and is
            // finalized by rename, not read back. Handle both for totality.
            CloudDownloadBody::Spilled(CloudSpilledBody::Temp(temp)) => {
                Ok(Box::new(super::multipart::SpilledReader::open(temp)?))
            }
            CloudDownloadBody::Spilled(CloudSpilledBody::Part(path)) => {
                Ok(Box::new(std::fs::File::open(path)?))
            }
        }
    }
}

/// Downloads the object with parallel ranged GETs into a pre-allocated file.
///
/// Shared assembly loop for S3, Azure, and GCS. The assembly file is chosen by
/// `target`: a non-encrypted download writes straight into the caller's
/// `<dst>.part` (one rename from done), while an encrypted / git-stage download
/// writes into a throwaway temp (its ciphertext is decrypted into `.part`
/// afterwards).
///
/// On failure the range futures are *drained*, not short-circuited: `collect`
/// polls EVERY future so all in-flight `write_at` spawn_blocking tasks finish
/// and release their cloned file handles before we return. With no writer
/// holding the file open, the cleanup can unlink the partially-written assembly
/// file even on Windows (which refuses to unlink an open file). The first error
/// is surfaced after the drain.
///
/// `get_range` must return exactly `range.end - range.start + 1` bytes; if an
/// endpoint ignores the Range header and returns the whole object (200 rather
/// than 206) this guard surfaces via `mk_range_err` before the overrun can
/// corrupt the pre-allocated assembly file. `mk_range_err` is kept distinct
/// from `mk_temp_err` (used for setup/join/write failures) so a caller can map
/// the truncation case to its own error variant rather than a generic one —
/// GCS does this today (`GcsRequestError::RangeNotHonored`); S3 and Azure both
/// pass the same closure for both parameters.
///
/// Every ranged GET acquires a governor permit before it is issued, so the
/// ranges of this object compete with the rest of the batch (and with every
/// other in-flight PUT/GET in the process) for one shared budget. Extra range
/// futures wait in the governor waiter queue: `data.parallel` bounds how many
/// ranges are in flight, and `buffer_unordered(part_concurrency)` bounds how
/// many more can be waiting.
///
/// Governor: each `get_range` call is one logical request. Provider closures
/// acquire and retain one permit across all attempts, backoff, and complete
/// body collection; this assembly helper remains provider-agnostic.
/// Callers drop the file permit after dest setup (HEAD / plan) before
/// invoking this helper, so in-flight files + ranges stay within the
/// command budget. Dest-file create/preallocate here is ungoverned, matching
/// S3 and Azure.
#[allow(clippy::too_many_arguments)]
pub(super) async fn assemble_ranged_download<E, Fut, G, M, MR>(
    content_length: u64,
    chunk_size: u64,
    part_concurrency: usize,
    target: CloudSpillTarget<'_>,
    unsafe_file_write: bool,
    mk_temp_err: M,
    mk_range_err: MR,
    get_range: G,
) -> Result<CloudSpilledBody, E>
where
    G: Fn(super::multipart::DownloadRange) -> Fut,
    Fut: std::future::Future<Output = Result<Bytes, E>>,
    M: Fn(String) -> E,
    MR: Fn(String) -> E,
{
    // Owns the assembly file for the download's duration.
    enum Assembly {
        Part(PathBuf),
        Temp(NamedTempFile),
    }

    let owned_target = match target {
        CloudSpillTarget::Part(p) => (true, p.to_path_buf()),
        CloudSpillTarget::Temp { dir, .. } => (false, dir.to_path_buf()),
    };
    // Setup returns io::Error (NOT E) so mk_temp_err need not be Send/'static.
    let (assembly, file): (Assembly, Arc<std::fs::File>) = tokio::task::spawn_blocking(move || {
        let (is_part, path_or_dir) = owned_target;
        if is_part {
            let f = super::create_output_file(&path_or_dir, unsafe_file_write)?;
            f.set_len(content_length)?; // pre-allocate for out-of-order pwrite
            Ok::<_, std::io::Error>((Assembly::Part(path_or_dir), Arc::new(f)))
        } else {
            let named = NamedTempFile::new_in(&path_or_dir)?;
            named.as_file().set_len(content_length)?;
            let file = Arc::new(named.as_file().try_clone()?);
            Ok((Assembly::Temp(named), file))
        }
    })
    .await
    .map_err(|e| mk_temp_err(format!("join error in tempfile setup: {e}")))?
    .map_err(|e| mk_temp_err(e.to_string()))?;

    // The temp only exists now that setup has run, so this is the earliest the
    // removal can be registered. `Part` needs nothing: `download_single_file`
    // already registered its `.part`.
    let temp_to_remove = match (&assembly, target) {
        (
            Assembly::Temp(named),
            CloudSpillTarget::Temp {
                cleanup: Some(_), ..
            },
        ) => Some(named.path().to_path_buf()),
        _ => None,
    };
    let temp_cleanup_scope = match target {
        CloudSpillTarget::Temp { cleanup, .. } => temp_to_remove.as_ref().and(cleanup),
        CloudSpillTarget::Part(_) => None,
    };
    let remove_temp_on_cancel = {
        let temp_to_remove = temp_to_remove.clone();
        async move {
            if let Some(path) = temp_to_remove {
                super::remove_partial_after_cancel(path).await;
            }
        }
    };

    let ranges = super::multipart::plan_ranges(content_length, chunk_size);
    let get_range = &get_range;
    let mk_temp_err = &mk_temp_err;
    let mk_range_err = &mk_range_err;
    let file_handle = &file;
    // Drain, don't short-circuit: `collect` polls EVERY future so all in-flight
    // write_at spawn_blocking tasks release their cloned handles before return,
    // so the assembly file can be unlinked even on Windows. First error surfaced
    // after the drain.
    let fetch_ranges = futures::stream::iter(ranges)
        .map(|range| async move {
            let bytes = { get_range(range).await? };
            // 206-vs-200 guard: an endpoint that ignores Range and returns the
            // whole object (200) would overrun the pre-allocated length.
            let expected_len = range.end - range.start + 1;
            if bytes.len() as u64 != expected_len {
                return Err(mk_range_err(format!(
                    "ranged GET returned {} bytes, expected {expected_len} \
                     (bytes={}-{}); endpoint may not honour Range header",
                    bytes.len(),
                    range.start,
                    range.end
                )));
            }
            let file = Arc::clone(file_handle);
            tokio::task::spawn_blocking(move || {
                super::multipart::write_at(&file, range.start, &bytes)
            })
            .await
            .map_err(|e| mk_temp_err(format!("join error writing chunk: {e}")))?
            .map_err(|e| mk_temp_err(e.to_string()))
        })
        .buffer_unordered(part_concurrency)
        .collect();
    // Boxed to keep this large future off the frame — see clippy.toml.
    let results: Vec<Result<(), E>> = with_cleanup_scope_opt(
        temp_cleanup_scope,
        remove_temp_on_cancel,
        Box::pin(fetch_ranges),
    )
    .await;

    // Release our handle so only `assembly` (Temp) or none (Part) holds the file.
    drop(file);
    let outcome = results.into_iter().collect::<Result<Vec<()>, _>>();

    match assembly {
        Assembly::Part(path) => match outcome {
            Ok(_) => Ok(CloudSpilledBody::Part(path)),
            Err(e) => {
                // drained: safe on Windows
                let _ = tokio::task::spawn_blocking(move || std::fs::remove_file(path)).await;
                Err(e)
            }
        },
        Assembly::Temp(named) => outcome.map(|_| CloudSpilledBody::Temp(named.into_temp_path())),
    }
}

/// Builds a streaming `reqwest::Body` for a GCS/Azure upload. CSE wraps the
/// source in a lazy `EncryptingReader` (ciphertext produced on demand, never
/// materialized); callers then set `Content-Length` to `cipher_len`, as a
/// wrapped stream has no known length. SSE streams the source as-is (handing
/// reqwest a `File` / `Bytes` so it can derive `Content-Length` itself).
pub(super) async fn body_for(
    source: &ByteSource,
    encryptor: Option<&Encryptor>,
) -> std::io::Result<reqwest::Body> {
    match encryptor {
        Some(enc) => {
            // Open async up-front so a slow open on a networked FS (NFS/EBS)
            // runs off the runtime thread *and* a failure surfaces here as a
            // non-retryable build error (before the body streams), not
            // mid-stream. The encrypting stream then just consumes the
            // already-open reader, so its opener is infallible.
            let reader = source.open_async().await?;
            Ok(reqwest::Body::wrap_stream(encrypting_body_stream(
                move || Ok(reader),
                enc.clone(),
            )))
        }
        None => match source {
            ByteSource::Path(p) => {
                // Async open: the `open()` syscall runs on tokio's blocking pool,
                // so a slow open on a networked filesystem (NFS, EBS) never stalls
                // the runtime thread (and, unlike `block_in_place`, this works on a
                // current-thread runtime). The failure still surfaces here as a
                // non-retryable build error, before the body streams — not
                // mid-stream. reqwest then streams the file body off-thread.
                let tokio_file = tokio::fs::File::open(p).await?;
                Ok(reqwest::Body::from(tokio_file))
            }
            ByteSource::Bytes(b) => Ok(reqwest::Body::from(b.clone())),
        },
    }
}

/// Drives an `EncryptingReader` on a `spawn_blocking` task (AES runs off the
/// runtime thread, mirroring the GET-side decrypt) and exposes the ciphertext
/// chunks as a `Stream` — the upload-side counterpart of
/// [`spawn_admitted_byte_stream_producer`]. The `Crypter` is built inside the task with
/// the fixed key+IV, so rebuilding the stream per retry yields identical bytes.
///
/// The source is supplied as an `open` closure rather than an already-open
/// reader so the `open()` syscall itself runs inside the blocking task, off the
/// runtime thread — a slow or hung open on a networked FS (NFS/EBS) must not
/// stall a tokio worker. The S3 CSE path relies on this: its `SdkBody::retryable`
/// builder is a sync `Fn` that can't `await`, so it can't open async up-front
/// like GCS/Azure do; instead it hands in `move || source.open()` and the open
/// happens here. A failed open arrives as the stream's first (error) frame,
/// before any body bytes. GCS/Azure open async up-front and pass an infallible
/// `move || Ok(reader)`, keeping their open failure an up-front build error.
pub(super) fn encrypting_body_stream(
    open: impl FnOnce() -> std::io::Result<Box<dyn Read + Send>> + Send + 'static,
    encryptor: Encryptor,
) -> impl Stream<Item = std::io::Result<Bytes>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<std::io::Result<Bytes>>(8);
    tokio::task::spawn_blocking(move || {
        let reader = match open() {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.blocking_send(Err(e));
                return;
            }
        };
        let mut enc_reader = match encryptor.encrypting_reader(reader) {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.blocking_send(Err(std::io::Error::other(e)));
                return;
            }
        };
        let mut buf = vec![0u8; UPLOAD_CHUNK_SIZE_BYTES];
        loop {
            match enc_reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx
                        .blocking_send(Ok(Bytes::copy_from_slice(&buf[..n])))
                        .is_err()
                    {
                        break; // consumer (request body) dropped
                    }
                }
                Err(e) => {
                    let _ = tx.blocking_send(Err(e));
                    break;
                }
            }
        }
    });
    tokio_stream::wrappers::ReceiverStream::new(rx)
}

/// Strategy each cloud module implements to wire its error variants into
/// [`upload_with_retry`]. Default `on_special_status` lets clouds add a
/// short-circuit for status codes that aren't HTTP failures (GCS: 401 →
/// `TokenExpired`).
pub(super) trait UploadRetryAdapter {
    type Err;
    type BuildErr;

    fn on_build_err(&self, e: Self::BuildErr) -> Self::Err;
    fn on_special_status(&self, _status: StatusCode) -> Option<Self::Err> {
        None
    }
    fn on_http_failure(&self, status: u16, body: String) -> Self::Err;
    fn on_transport(&self, e: reqwest::Error) -> Self::Err;
    fn on_exhausted(&self, detail: String) -> Self::Err;
    fn on_deadline_exceeded(&self, elapsed: Duration, budget: Duration) -> Self::Err;
}

/// Shared retry/backoff loop for the cloud upload paths. The async closure
/// rebuilds the request per attempt (re-opening the source off the runtime
/// thread via `body_for`) and may fail (e.g. a per-retry file open) — failures
/// are non-retryable and surface via `adapter.on_build_err`.
///
/// `deadline` is the caller's one absolute budget for this logical request.
/// The caller holds `_permit` across this loop, including retries and backoff.
/// Credential/SAS refresh of the same unit also holds that permit.
pub(super) async fn upload_with_retry<F, M>(
    policy: &RetryPolicy,
    adapter: &M,
    method: &reqwest::Method,
    url: &str,
    build_request: F,
    deadline: Deadline,
    _permit: &RequestPermit,
) -> Result<(), M::Err>
where
    F: AsyncFn() -> Result<reqwest::RequestBuilder, M::BuildErr>,
    M: UploadRetryAdapter,
{
    let max_attempts = policy.max_attempts;
    let mut sleep_ms = policy.backoff.base.as_millis() as f64;

    // Only the host + path may be logged (ud-log-http-traffic-at-debug); the
    // query string can carry a SAS token / signature, so strip it.
    let log_path = url.split(['?', '#']).next().unwrap_or("");

    for attempt in 1..=max_attempts {
        deadline.check(|elapsed, budget| adapter.on_deadline_exceeded(elapsed, budget))?;

        let req = build_request_within_deadline(deadline, adapter, &build_request).await?;

        let remaining = match deadline.remaining() {
            Remaining::Expired { elapsed, budget } => {
                return Err(adapter.on_deadline_exceeded(elapsed, budget));
            }
            Remaining::Unlimited => None,
            Remaining::Left(left) => Some(left),
        };
        let timeout = request_timeout_after_admission(policy.per_request_timeout, remaining);
        if timeout.is_zero() {
            return Err(match deadline.budget() {
                Some(budget) => adapter.on_deadline_exceeded(deadline.elapsed(), budget),
                None => adapter.on_exhausted(
                    "upload per-request timeout is zero with no retry deadline".to_string(),
                ),
            });
        }
        let req = req.timeout(timeout);

        tracing::debug!(method = %method, path = %log_path, attempt, "outbound HTTP call");

        let send_result = match deadline.timeout_at() {
            Some(absolute) => match tokio::time::timeout_at(absolute, req.send()).await {
                Ok(inner) => inner,
                Err(_) => {
                    return Err(adapter.on_deadline_exceeded(
                        deadline.elapsed(),
                        deadline.budget().unwrap_or_else(|| deadline.elapsed()),
                    ));
                }
            },
            None => req.send().await,
        };

        match send_result {
            Ok(resp) => {
                tracing::debug!(status = resp.status().as_u16(), "HTTP response");
                if resp.status().is_success() {
                    return Ok(());
                }
                if let Some(early) = adapter.on_special_status(resp.status()) {
                    return Err(early);
                }
                let status_code = resp.status().as_u16();
                let retryable = is_retryable_status(status_code, &policy.extra_retryable_statuses);
                if !retryable || attempt >= max_attempts {
                    let body = read_error_body(resp).await;
                    return Err(adapter.on_http_failure(status_code, body));
                }
                drop(resp);
                sleep_backoff_within_deadline(
                    deadline,
                    adapter,
                    Duration::from_millis(sleep_ms as u64),
                )
                .await?;
                sleep_ms = next_delay_ms(sleep_ms, &policy.backoff);
            }
            Err(e) => {
                if attempt >= max_attempts {
                    return Err(adapter.on_transport(e));
                }
                sleep_backoff_within_deadline(
                    deadline,
                    adapter,
                    Duration::from_millis(sleep_ms as u64),
                )
                .await?;
                sleep_ms = next_delay_ms(sleep_ms, &policy.backoff);
            }
        }
    }

    Err(adapter.on_exhausted(format!("upload exhausted {max_attempts} attempts")))
}

#[cfg(test)]
mod ranged_download_tests {
    use super::*;
    use crate::governor::{CommandParallelism, Governor, RequestKind, TransferOp};
    use std::sync::atomic::AtomicUsize;

    fn cache_with(
        tls_config: crate::tls::config::TlsConfig,
    ) -> (StorageHttpClientCache, StageTransport) {
        (
            StorageHttpClientCache::new(),
            StageTransport {
                tls_config,
                ..StageTransport::for_test()
            },
        )
    }

    /// Answers every HTTP/1.1 request with a keep-alive `200 ok` and counts the
    /// TCP connections it accepts.
    async fn keep_alive_listener() -> (std::net::SocketAddr, Arc<AtomicU64>) {
        use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let addr = listener.local_addr().expect("local addr");
        let accepted = Arc::new(AtomicU64::new(0));
        let counter = Arc::clone(&accepted);
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    let mut head = Vec::new();
                    let mut byte = [0u8; 1];
                    while let Ok(1) = socket.read(&mut byte).await {
                        head.push(byte[0]);
                        if head.ends_with(b"\r\n\r\n") {
                            head.clear();
                            let reply = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                            if socket.write_all(reply).await.is_err() {
                                return;
                            }
                        }
                    }
                });
            }
        });
        (addr, accepted)
    }

    async fn get_ok(client: &StorageHttpClient, addr: std::net::SocketAddr) {
        let body = client
            .reqwest()
            .expect("reqwest client")
            .get(format!("http://{addr}/"))
            .send()
            .await
            .expect("request succeeds")
            .text()
            .await
            .expect("body");
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn storage_http_client_cache_reuses_the_connection_across_commands() {
        let (addr, accepted) = keep_alive_listener().await;
        let (cache, transport) = cache_with(Default::default());
        let shared = cache.clone();

        get_ok(
            &cache.get(LocationType::Azure, &transport).await.unwrap(),
            addr,
        )
        .await;
        get_ok(
            &shared.get(LocationType::Azure, &transport).await.unwrap(),
            addr,
        )
        .await;

        assert_eq!(accepted.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn storage_http_client_cache_builds_an_uncached_client_for_another_cloud() {
        let (addr, accepted) = keep_alive_listener().await;
        let (cache, transport) = cache_with(Default::default());

        get_ok(
            &cache.get(LocationType::Gcs, &transport).await.unwrap(),
            addr,
        )
        .await;
        assert!(
            cache
                .get(LocationType::S3, &transport)
                .await
                .unwrap()
                .s3()
                .is_some()
        );
        get_ok(
            &cache.get(LocationType::Gcs, &transport).await.unwrap(),
            addr,
        )
        .await;
        assert_eq!(accepted.load(Ordering::SeqCst), 1);

        get_ok(
            &cache.get(LocationType::Azure, &transport).await.unwrap(),
            addr,
        )
        .await;
        assert_eq!(accepted.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn storage_http_client_cache_does_not_cache_a_failed_build() {
        let (cache, transport) = cache_with(crate::tls::config::TlsConfig {
            custom_root_store_path: Some("/nonexistent/root-store.pem".into()),
            ..Default::default()
        });

        assert!(cache.get(LocationType::Azure, &transport).await.is_err());
        assert!(!cache.0.client.initialized());
    }

    /// Runs `assemble_ranged_download` over a `total`-byte object in
    /// `chunk_size` ranges, serving each range from a synthetic payload. Range 0
    /// sleeps while the rest only yield, so the ranges complete out of order and
    /// the assembled bytes prove the positioned writes, not the arrival order.
    #[derive(Default)]
    struct ConcurrencyProbe {
        in_flight: AtomicUsize,
        max: AtomicUsize,
    }

    impl ConcurrencyProbe {
        fn enter(&self) {
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max.fetch_max(now, Ordering::SeqCst);
        }

        fn leave(&self) {
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
        }

        fn peak(&self) -> usize {
            self.max.load(Ordering::SeqCst)
        }
    }

    async fn assemble_with_probe(
        op: &TransferOp,
        part_concurrency: usize,
        total: u64,
        chunk_size: u64,
    ) -> (Vec<u8>, usize) {
        let payload: Vec<u8> = (0..total).map(|i| (i % 251) as u8).collect();
        let probe = ConcurrencyProbe::default();
        let dir = tempfile::tempdir().expect("tempdir");
        let part_path = dir.path().join("out.bin.part");

        let body = assemble_ranged_download(
            total,
            chunk_size,
            part_concurrency,
            CloudSpillTarget::Part(&part_path),
            /* unsafe_file_write */ false,
            |detail: String| -> String { detail },
            |detail: String| -> String { detail },
            |range: super::super::multipart::DownloadRange| {
                let payload = payload.clone();
                let probe = &probe;
                let op = op.clone();
                async move {
                    let _permit = op.acquire(RequestKind::RangedGet).await.unwrap();
                    probe.enter();
                    if range.start == 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    } else {
                        tokio::task::yield_now().await;
                    }
                    let bytes =
                        Bytes::copy_from_slice(&payload[range.start as usize..=range.end as usize]);
                    probe.leave();
                    Ok::<Bytes, String>(bytes)
                }
            },
        )
        .await
        .expect("ranged assembly should succeed");

        let path = match body {
            CloudSpilledBody::Part(p) => p,
            CloudSpilledBody::Temp(_) => panic!("Part target must produce a Part body"),
        };
        (
            std::fs::read(path).expect("read assembled file"),
            probe.peak(),
        )
    }

    async fn assemble_out_of_order(total: u64, chunk_size: u64) -> Vec<u8> {
        let payload: Vec<u8> = (0..total).map(|i| (i % 251) as u8).collect();
        let dir = tempfile::tempdir().expect("tempdir");
        let part_path = dir.path().join("out.bin.part");

        let body = assemble_ranged_download(
            total,
            chunk_size,
            4,
            CloudSpillTarget::Part(&part_path),
            /* unsafe_file_write */ false,
            |detail: String| -> String { detail },
            |detail: String| -> String { detail },
            |range: super::super::multipart::DownloadRange| {
                let payload = payload.clone();
                async move {
                    if range.start == 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    } else {
                        tokio::task::yield_now().await;
                    }
                    Ok::<Bytes, String>(Bytes::copy_from_slice(
                        &payload[range.start as usize..=range.end as usize],
                    ))
                }
            },
        )
        .await
        .expect("ranged assembly should succeed");

        let path = match body {
            CloudSpilledBody::Part(p) => p,
            CloudSpilledBody::Temp(_) => panic!("Part target must produce a Part body"),
        };
        std::fs::read(path).expect("read assembled file")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_reassemble_ranges_in_order_regardless_of_completion_order() {
        let assembled = assemble_out_of_order(1000, 128).await;

        let expected: Vec<u8> = (0..1000u64).map(|i| (i % 251) as u8).collect();
        assert!(
            assembled == expected,
            "positioned writes must reassemble the object byte-for-byte"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_let_the_command_budget_bind_below_the_stream_width() {
        let governor = Governor::try_with_capacity(32).expect("capacity");
        let op = governor.register_transfer(CommandParallelism::from_resolved(2));
        let (assembled, peak) = assemble_with_probe(&op, 8, 800, 100).await;

        assert_eq!(assembled.len(), 800);
        assert!(
            peak <= 2,
            "the command limit must bind even when the stream width is wider: peak {peak} > 2"
        );
        assert!(
            peak > 1,
            "a limit of 2 must overlap two fetches, else this test would pass vacuously (peak {peak})"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn single_file_ranged_get_uses_full_command_parallel() {
        let governor = Governor::new();
        let op = governor.register_transfer(CommandParallelism::from_resolved(16));
        let (assembled, peak) = assemble_with_probe(&op, 16, 1600, 100).await;

        assert_eq!(assembled.len(), 1600);
        assert_eq!(
            peak, 16,
            "a single-file ranged GET with PARALLEL=16 must still run 16 ranges, not collapse to 1"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ranged_get_with_command_parallel_one_serializes() {
        let governor = Governor::new();
        let op = governor.register_transfer(CommandParallelism::from_resolved(1));
        let (assembled, peak) = assemble_with_probe(&op, 8, 800, 100).await;

        assert_eq!(assembled.len(), 800);
        assert_eq!(
            peak, 1,
            "command_parallel=1 must serialize ranged GETs: peak {peak}"
        );
    }
}

async fn build_request_within_deadline<F, M>(
    deadline: Deadline,
    adapter: &M,
    build_request: &F,
) -> Result<reqwest::RequestBuilder, M::Err>
where
    F: AsyncFn() -> Result<reqwest::RequestBuilder, M::BuildErr>,
    M: UploadRetryAdapter,
{
    deadline.check(|elapsed, budget| adapter.on_deadline_exceeded(elapsed, budget))?;
    match deadline
        .run(build_request(), |elapsed, budget| {
            adapter.on_deadline_exceeded(elapsed, budget)
        })
        .await
    {
        Ok(Ok(req)) => Ok(req),
        Ok(Err(e)) => Err(adapter.on_build_err(e)),
        Err(e) => Err(e),
    }
}

async fn sleep_backoff_within_deadline<M: UploadRetryAdapter>(
    deadline: Deadline,
    adapter: &M,
    delay: Duration,
) -> Result<(), M::Err> {
    sleep_within_deadline(deadline, delay, |elapsed, budget| {
        adapter.on_deadline_exceeded(elapsed, budget)
    })
    .await
}

fn request_timeout_after_admission(
    per_request_timeout: Option<Duration>,
    remaining: Option<Duration>,
) -> Duration {
    match (per_request_timeout, remaining) {
        (Some(prt), Some(rem)) => prt.min(rem),
        (Some(prt), None) => prt,
        (None, Some(rem)) => rem.min(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
        (None, None) => Duration::from_secs(REQUEST_TIMEOUT_SECS),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::retry::Jitter;
    use crate::governor::{
        AcquireWaitError, CommandParallelism, Deadline, Governor, RequestKind, TransferOp,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::time::Instant;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_command_parallel() -> CommandParallelism {
        CommandParallelism::from_resolved(32)
    }

    fn test_governor(max_in_flight: usize) -> Governor {
        Governor::try_with_capacity(max_in_flight).expect("test capacity must be nonzero")
    }

    async fn admit(
        op: &TransferOp,
        kind: RequestKind,
        budget: Option<Duration>,
    ) -> (Deadline, RequestPermit) {
        let deadline = Deadline::from_budget(budget);
        let permit = deadline.acquire(op, kind).await.expect("test admission");
        (deadline, permit)
    }

    struct TestUploadAdapter;

    impl UploadRetryAdapter for TestUploadAdapter {
        type Err = String;
        type BuildErr = String;

        fn on_build_err(&self, e: Self::BuildErr) -> Self::Err {
            e
        }
        fn on_http_failure(&self, status: u16, body: String) -> Self::Err {
            format!("{status} {body}")
        }
        fn on_transport(&self, e: reqwest::Error) -> Self::Err {
            e.to_string()
        }
        fn on_exhausted(&self, detail: String) -> Self::Err {
            detail
        }
        fn on_deadline_exceeded(&self, elapsed: Duration, budget: Duration) -> Self::Err {
            format!("deadline exceeded after {elapsed:?} (budget {budget:?})")
        }
    }

    fn retry_policy_with_backoff(max_attempts: u32, backoff: Duration) -> RetryPolicy {
        RetryPolicy {
            max_attempts,
            backoff: BackoffConfig {
                base: backoff,
                factor: 1.0,
                cap: backoff,
                jitter: Jitter::None,
            },
            ..RetryPolicy::default()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn run_within_deadline_cancels_work_that_outlives_the_budget() {
        let governor = test_governor(1);
        let op = governor.register_transfer(test_command_parallel());
        let policy = retry_policy_with_backoff(1, Duration::from_millis(1));
        let (deadline, permit) = admit(
            &op,
            RequestKind::WholeObjectGet,
            Some(Duration::from_millis(40)),
        )
        .await;

        let err = run_within_deadline(
            &policy,
            deadline,
            &permit,
            |elapsed, budget| format!("deadline exceeded after {elapsed:?} of {budget:?}"),
            |_| async {
                tokio::time::sleep(Duration::from_millis(80)).await;
                Ok::<_, String>(())
            },
        )
        .await
        .expect_err("work that stalls after response headers must hit the absolute deadline");

        assert!(err.starts_with("deadline exceeded"), "{err}");
        drop(permit);
        assert_eq!(governor.in_flight(), 0);
    }

    #[tokio::test]
    async fn upload_retry_retains_permit_during_backoff() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(503).set_body_string("retry-body"))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let governor = test_governor(1);
        let op_a = governor.register_transfer(test_command_parallel());
        let op_b = governor.register_transfer(test_command_parallel());
        let (deadline, permit) = admit(&op_a, RequestKind::WholeObjectPut, None).await;

        let url = server.uri();
        let client = reqwest::Client::new();
        let policy = retry_policy_with_backoff(3, Duration::from_millis(400));
        {
            let upload = upload_with_retry(
                &policy,
                &TestUploadAdapter,
                &reqwest::Method::PUT,
                &url,
                async || Ok(client.put(&url).body("payload")),
                deadline,
                &permit,
            );
            tokio::pin!(upload);

            let wait_start = Instant::now();
            loop {
                tokio::select! {
                    biased;
                    result = &mut upload => {
                        panic!("upload finished before competing acquire: {result:?}");
                    }
                    _ = tokio::time::sleep(Duration::from_millis(5)) => {
                        let received = server.received_requests().await.unwrap_or_default();
                        if received.len() == 1 && governor.in_flight() == 1 {
                            break;
                        }
                        assert!(
                            wait_start.elapsed() < Duration::from_secs(2),
                            "timed out waiting for first attempt to enter backoff"
                        );
                    }
                }
            }

            let competing = tokio::time::timeout(
                Duration::from_millis(80),
                op_b.acquire(RequestKind::WholeObjectPut),
            )
            .await;
            assert!(
                competing.is_err(),
                "competing acquire must remain blocked during backoff"
            );
            assert_eq!(governor.in_flight(), 1);

            upload.await.expect("upload succeeds on retry");
        }
        drop(permit);
        assert_eq!(governor.in_flight(), 0);

        let after = tokio::time::timeout(
            Duration::from_millis(80),
            op_b.acquire(RequestKind::WholeObjectPut),
        )
        .await
        .expect("competing acquire must complete after retry sequence")
        .expect("competing acquire granted");
        drop(after);
    }

    #[tokio::test]
    async fn cancelling_upload_retry_during_backoff_releases_permit() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let governor = test_governor(1);
        let op_a = governor.register_transfer(test_command_parallel());
        let op_b = governor.register_transfer(test_command_parallel());
        let url = server.uri();
        let client = reqwest::Client::new();
        let policy = retry_policy_with_backoff(3, Duration::from_secs(10));
        {
            let (deadline, permit) = admit(&op_a, RequestKind::WholeObjectPut, None).await;
            let upload = upload_with_retry(
                &policy,
                &TestUploadAdapter,
                &reqwest::Method::PUT,
                &url,
                async || Ok(client.put(&url).body("payload")),
                deadline,
                &permit,
            );
            tokio::pin!(upload);

            let wait_start = Instant::now();
            loop {
                tokio::select! {
                    result = &mut upload => {
                        panic!("upload finished before cancellation: {result:?}");
                    }
                    _ = tokio::task::yield_now() => {
                        let received = server.received_requests().await.unwrap_or_default();
                        if received.len() == 1 && governor.in_flight() == 1 {
                            break;
                        }
                        assert!(
                            wait_start.elapsed() < Duration::from_secs(2),
                            "timed out waiting for retry backoff"
                        );
                    }
                }
            }
        }

        assert_eq!(governor.in_flight(), 0);

        let after = tokio::time::timeout(
            Duration::from_millis(80),
            op_b.acquire(RequestKind::WholeObjectPut),
        )
        .await
        .expect("competing acquire must complete after cancellation")
        .expect("competing acquire granted");
        drop(after);
    }

    #[tokio::test]
    async fn upload_retry_drops_retryable_response_before_backoff() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let addr = listener.local_addr().expect("listener address");
        let (closed_tx, closed_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (mut first, _) = listener.accept().await.expect("first connection");
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while first.read_exact(&mut byte).await.is_ok() {
                request.push(byte[0]);
                if request.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            first
                .write_all(
                    b"HTTP/1.1 503 Service Unavailable\r\n\
                      Content-Length: 65536\r\nConnection: close\r\n\r\nx",
                )
                .await
                .expect("write retryable headers");
            first.flush().await.expect("flush retryable headers");
            let closed = first.read(&mut byte).await.expect("observe client close") == 0;
            let _ = closed_tx.send(closed);

            let (mut second, _) = listener.accept().await.expect("retry connection");
            request.clear();
            while second.read_exact(&mut byte).await.is_ok() {
                request.push(byte[0]);
                if request.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            second
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await
                .expect("write success response");
        });

        let governor = test_governor(1);
        let op = governor.register_transfer(test_command_parallel());
        let (deadline, permit) = admit(&op, RequestKind::WholeObjectPut, None).await;
        let url = format!("http://{addr}/");
        let client = reqwest::Client::new();
        let policy = retry_policy_with_backoff(2, Duration::from_millis(500));
        {
            let upload = upload_with_retry(
                &policy,
                &TestUploadAdapter,
                &reqwest::Method::PUT,
                &url,
                async || Ok(client.put(&url)),
                deadline,
                &permit,
            );
            tokio::pin!(upload);
            let closed_before_backoff = tokio::time::timeout(Duration::from_millis(250), async {
                tokio::select! {
                    result = &mut upload => {
                        panic!("upload completed before response disposal was observed: {result:?}")
                    }
                    closed = closed_rx => closed.expect("server close observation"),
                }
            })
            .await
            .expect("retry response must be dropped before the 500 ms backoff");
            assert!(
                closed_before_backoff,
                "dropping an unread retryable response must close its connection"
            );
            upload.await.expect("retry succeeds");
        }
        drop(permit);
        assert_eq!(governor.in_flight(), 0);
    }

    #[tokio::test]
    async fn admission_wait_respects_max_elapsed() {
        let governor = test_governor(1);
        let holder = governor.register_transfer(test_command_parallel());
        let blocked = governor.register_transfer(test_command_parallel());
        let _hold = holder.acquire(RequestKind::WholeObjectPut).await.unwrap();

        let deadline = Deadline::from_budget(Some(Duration::from_millis(40)));
        let err = deadline
            .acquire(&blocked, RequestKind::WholeObjectPut)
            .await
            .expect_err("saturated admission must hit the retry deadline");
        assert!(
            matches!(err, AcquireWaitError::DeadlineExceeded { .. }),
            "expected deadline error, got {err:?}"
        );
        assert_eq!(governor.in_flight(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn upload_retry_slow_builder_hits_deadline_without_send() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let governor = test_governor(1);
        let op = governor.register_transfer(test_command_parallel());
        let url = server.uri();
        let client = reqwest::Client::new();
        let mut policy = retry_policy_with_backoff(2, Duration::from_millis(1));
        policy.max_elapsed = Some(Duration::from_millis(40));
        let (deadline, permit) = admit(&op, RequestKind::WholeObjectPut, policy.max_elapsed).await;

        let err = upload_with_retry(
            &policy,
            &TestUploadAdapter,
            &reqwest::Method::PUT,
            &url,
            async || {
                tokio::time::sleep(Duration::from_millis(80)).await;
                Ok(client.put(&url).body("payload"))
            },
            deadline,
            &permit,
        )
        .await
        .expect_err("slow builder must consume the absolute deadline");
        assert!(
            err.starts_with("deadline exceeded"),
            "expected deadline error, got {err}"
        );
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "no outbound request when the builder exceeds max_elapsed"
        );
        drop(permit);
        assert_eq!(governor.in_flight(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn upload_retry_capacity_free_at_deadline_does_not_send() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let governor = test_governor(1);
        let holder = governor.register_transfer(test_command_parallel());
        let blocked = governor.register_transfer(test_command_parallel());
        let hold = holder.acquire(RequestKind::WholeObjectPut).await.unwrap();

        let url = server.uri();
        let client = reqwest::Client::new();
        let mut policy = retry_policy_with_backoff(2, Duration::from_millis(1));
        policy.max_elapsed = Some(Duration::from_millis(50));
        let deadline = Deadline::from_budget(policy.max_elapsed);

        let acquire = deadline.acquire(&blocked, RequestKind::WholeObjectPut);
        tokio::pin!(acquire);

        // Poll until admission is queued, then expire the budget and free
        // the slot at the same instant. An acquire that wins at the deadline
        // still must not send.
        loop {
            tokio::select! {
                biased;
                result = &mut acquire => {
                    panic!("acquire finished before the deadline elapsed: {result:?}");
                }
                _ = tokio::task::yield_now() => {
                    if governor.queued() == 1 {
                        break;
                    }
                }
            }
        }
        tokio::time::advance(Duration::from_millis(50)).await;
        drop(hold);

        match acquire.await {
            Err(AcquireWaitError::DeadlineExceeded { .. }) => {}
            Ok(permit) => {
                let err = upload_with_retry(
                    &policy,
                    &TestUploadAdapter,
                    &reqwest::Method::PUT,
                    &url,
                    async || Ok(client.put(&url).body("payload")),
                    deadline,
                    &permit,
                )
                .await
                .expect_err("capacity free at the deadline must not send");
                assert!(
                    err.starts_with("deadline exceeded"),
                    "expected deadline error, got {err}"
                );
            }
            Err(other) => panic!("expected deadline error, got {other:?}"),
        }
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "no outbound request when capacity frees at max_elapsed"
        );
    }

    #[tokio::test]
    async fn upload_retry_backoff_exceeding_remaining_budget_does_not_resend() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(503).set_body_string("retry"))
            .mount(&server)
            .await;

        let governor = test_governor(1);
        let op = governor.register_transfer(test_command_parallel());
        let url = server.uri();
        let client = reqwest::Client::new();
        let mut policy = retry_policy_with_backoff(3, Duration::from_millis(80));
        policy.max_elapsed = Some(Duration::from_millis(40));
        let (deadline, permit) = admit(&op, RequestKind::WholeObjectPut, policy.max_elapsed).await;

        let err = upload_with_retry(
            &policy,
            &TestUploadAdapter,
            &reqwest::Method::PUT,
            &url,
            async || Ok(client.put(&url).body("payload")),
            deadline,
            &permit,
        )
        .await
        .expect_err("backoff longer than remaining budget must stop");
        assert!(
            err.starts_with("deadline exceeded"),
            "expected deadline error, got {err}"
        );
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            1,
            "must not start a second outbound request after the backoff would exceed max_elapsed"
        );
        drop(permit);
        assert_eq!(governor.in_flight(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn sleep_backoff_fails_immediately_when_delay_exceeds_remaining() {
        let deadline = Deadline::from_budget(Some(Duration::from_millis(40)));
        let err =
            sleep_backoff_within_deadline(deadline, &TestUploadAdapter, Duration::from_millis(80))
                .await
                .expect_err("delay longer than remaining must fail immediately");
        assert!(
            err.starts_with("deadline exceeded"),
            "expected deadline error, got {err}"
        );
        assert_eq!(
            deadline.elapsed(),
            Duration::ZERO,
            "must not sleep when the backoff cannot complete before the deadline"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn deadline_remaining_distinguishes_none_and_expired() {
        let unlimited = Deadline::from_budget(None);
        assert!(matches!(unlimited.remaining(), Remaining::Unlimited));

        let budget = Duration::from_millis(50);
        let deadline = Deadline::from_budget(Some(budget));
        match deadline.remaining() {
            Remaining::Left(left) => assert_eq!(left, budget),
            other => panic!("expected Left, got {other:?}"),
        }
        assert_eq!(
            request_timeout_after_admission(None, Some(budget)),
            budget.min(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        );
        assert_eq!(
            request_timeout_after_admission(None, None),
            Duration::from_secs(REQUEST_TIMEOUT_SECS)
        );

        tokio::time::advance(budget).await;
        assert!(matches!(deadline.remaining(), Remaining::Expired { .. }));
        assert!(
            matches!(unlimited.remaining(), Remaining::Unlimited),
            "no max_elapsed stays Unlimited after time advances"
        );
    }

    #[tokio::test]
    async fn deferred_body_keeps_permit_until_owner_drops_it() {
        let governor = test_governor(1);
        let op_a = governor.register_transfer(test_command_parallel());
        let op_b = governor.register_transfer(test_command_parallel());
        let (deadline, permit) = admit(&op_a, RequestKind::WholeObjectGet, None).await;

        let (value, permit) = admitted_request_deferred_body(
            &RetryPolicy::default(),
            deadline,
            permit,
            |elapsed, budget| format!("deadline {elapsed:?}/{budget:?}"),
            |_| async { Ok::<_, String>(42) },
        )
        .await
        .expect("deferred request opens");
        assert_eq!(value, 42);
        assert_eq!(governor.in_flight(), 1);

        let waiter = tokio::spawn(op_b.acquire(RequestKind::WholeObjectGet));
        while governor.queued() == 0 {
            tokio::task::yield_now().await;
        }
        assert!(!waiter.is_finished());

        drop(permit);
        let next = waiter.await.expect("waiter task").expect("waiter granted");
        drop(next);
        assert_eq!(governor.in_flight(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_release_the_permit_after_the_producer_drains_to_eof() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello world".to_vec()))
            .mount(&mock)
            .await;
        let response = reqwest::Client::new()
            .get(mock.uri())
            .send()
            .await
            .expect("mock GET must succeed");

        let governor = test_governor(1);
        let op = governor.register_transfer(test_command_parallel());
        let permit = op.acquire(RequestKind::WholeObjectGet).await.unwrap();
        assert_eq!(governor.in_flight(), 1);
        let (mut reader, _abort_handle) = spawn_admitted_byte_stream_producer(response, permit);

        tokio::task::spawn_blocking(move || {
            use std::io::Read;
            let mut buf = Vec::new();
            reader.read_to_end(&mut buf)
        })
        .await
        .expect("join")
        .expect("draining the mock body must succeed");

        let second = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            op.acquire(RequestKind::WholeObjectGet),
        )
        .await
        .expect("draining to EOF must release the permit")
        .expect("second acquire must succeed");
        drop(second);
        assert_eq!(governor.in_flight(), 0);
    }

    #[tokio::test]
    async fn should_release_the_permit_when_the_producer_is_aborted() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"unread".to_vec()))
            .mount(&mock)
            .await;
        let response = reqwest::Client::new()
            .get(mock.uri())
            .send()
            .await
            .expect("mock GET must succeed");

        let governor = test_governor(1);
        let op = governor.register_transfer(test_command_parallel());
        let permit = op.acquire(RequestKind::WholeObjectGet).await.unwrap();
        assert_eq!(governor.in_flight(), 1);
        let (_reader, abort_handle) = spawn_admitted_byte_stream_producer(response, permit);
        abort_handle.abort();

        let second = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            op.acquire(RequestKind::WholeObjectGet),
        )
        .await
        .expect("aborting the producer must still release the permit")
        .expect("second acquire must succeed");
        drop(second);
        assert_eq!(governor.in_flight(), 0);
    }
}
