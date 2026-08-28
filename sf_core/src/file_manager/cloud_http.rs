//! Helpers shared by the GCS, Azure, and (partially) S3 transfer paths.
//!
//! S3 skips the reqwest-specific retry loop (`upload_with_retry`) and uses
//! the AWS SDK's own retry/backoff instead. But it reuses the
//! async-stream → sync-`Read` bridge (`StreamReader`) via its own producer,
//! `spawn_s3_byte_stream_producer`, since `ByteStream::next` isn't a
//! `futures_core::Stream` and needs its own drain loop.

use super::encryption::Encryptor;
use super::types::{ByteSource, EncryptedFileMetadata};
use crate::apis::operation_ctx::{CleanupScope, with_cleanup_scope_opt};
use crate::config::retry::{BackoffConfig, RetryPolicy};
use crate::log_foreign_error;
use bytes::Bytes;
use futures::StreamExt as _;
use futures::stream::Stream;
use reqwest::StatusCode;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;

/// Read-buffer size in bytes for the streaming upload producer — one channel chunk.
const UPLOAD_CHUNK_SIZE_BYTES: usize = 64 * 1024;

/// Per-attempt HTTP timeout. Matches the cloud transfer modules' historical
/// 300s cap; the retry budget (`policy.max_elapsed`) must exceed this so at
/// least one full attempt can complete.
const REQUEST_TIMEOUT_SECS: u64 = 300;

use std::collections::BTreeSet;

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

/// Spawns a tokio task that drains `response.bytes_stream()` into a bounded
/// mpsc channel, returning the [`StreamReader`] plus an
/// [`tokio::task::AbortHandle`] for the task — the reqwest-based (GCS/Azure)
/// analogue of [`spawn_s3_byte_stream_producer`]'s abort handle, so a
/// stalled connection can be cancelled the same way instead of parking
/// forever on `bytes_stream().next()`. Channel capacity is 8 chunks (≈2 MiB
/// at typical 256 KiB chunks) — enough to keep the producer busy while the
/// consumer decrypts.
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
pub(super) fn spawn_byte_stream_producer(
    response: reqwest::Response,
) -> (StreamReader, tokio::task::AbortHandle) {
    let (tx, rx) = tokio::sync::mpsc::channel::<std::io::Result<Bytes>>(8);
    let stream = response.bytes_stream();
    let join_handle = tokio::spawn(async move {
        let mut stream = stream;
        while let Some(chunk_result) = stream.next().await {
            let mapped = chunk_result.map_err(std::io::Error::other);
            // If the consumer dropped (decryption finished/errored) while we
            // had a pending error, the error is silently lost — the consumer
            // already has its own failure to report. Log at debug so it's
            // recoverable from traces if downstream behaviour ever surprises.
            //
            // `.await`, not a blocking send: this runs on a tokio worker and
            // must yield to the runtime when the channel is full, not park
            // the thread.
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

/// S3 analogue of [`spawn_byte_stream_producer`]: drains an
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
pub(super) fn spawn_s3_byte_stream_producer(
    mut body: aws_sdk_s3::primitives::ByteStream,
) -> (StreamReader, tokio::task::AbortHandle) {
    let (tx, rx) = tokio::sync::mpsc::channel::<std::io::Result<Bytes>>(8);
    let join_handle = tokio::spawn(async move {
        while let Some(chunk_result) = body.next().await {
            let mapped = chunk_result.map_err(std::io::Error::other);
            // `.await`, not blocking send — see `spawn_byte_stream_producer`.
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

/// Client-side-encryption inputs a download carries for the decrypt path,
/// bundled so they are present together or not at all. A downloaded CSE object
/// always carries both the encryption-metadata headers and the matching
/// SHA-256 digest, and `decrypt_ciphertext_to_writer` needs both; SSE / raw
/// objects carry neither and the caller sees `None`. Keeping these as one
/// `Option` rather than two makes the "metadata present, digest absent" state
/// (always invalid) unrepresentable — the download path validates both
/// headers at the boundary before constructing this.
pub struct CseDownloadInfo {
    pub metadata: EncryptedFileMetadata,
    pub digest: String,
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
    /// `Some` for a client-side-encrypted object (both metadata + digest
    /// headers were present); `None` for SSE / raw objects.
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
    /// Builds a [`CloudStreamingDownload`] from a single-GET reqwest
    /// `response` whose body streams straight off the network — the shared
    /// tail `gcs_transfer::download_from_gcs_streaming` and
    /// `azure_transfer::azure_get_streaming` delegate to after parsing their
    /// cloud-specific headers into `digest` / `file_metadata`. Not used by
    /// `download_from_azure_streaming`'s streamed branch, whose
    /// `cloud_byte_count` comes from an earlier HEAD probe instead.
    pub(super) fn from_reqwest_response(
        response: reqwest::Response,
        digest: Option<String>,
        file_metadata: Option<EncryptedFileMetadata>,
    ) -> Self {
        let cloud_byte_count = response.content_length().unwrap_or(0) as i64;
        // Git-stage objects carry encryption headers but no sfc-digest —
        // treat as non-CSE, matching every other download path.
        let cse_info = match (file_metadata, digest) {
            (Some(metadata), Some(digest)) => Some(CseDownloadInfo { metadata, digest }),
            (Some(_), None) => {
                tracing::debug!(
                    "encryptiondata present but sfc-digest absent (git-stage object); \
                     treating as non-CSE"
                );
                None
            }
            (None, _) => None,
        };
        let (reader, producer_abort) = spawn_byte_stream_producer(response);
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
    /// Cloned so a caller can keep it past [`Self::into_reader`], which drops the
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
#[allow(clippy::too_many_arguments)]
pub(super) async fn assemble_ranged_download<E, Fut, G, M, MR>(
    content_length: u64,
    chunk_size: u64,
    concurrency: usize,
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
            let bytes = get_range(range).await?;
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
        .buffer_unordered(concurrency)
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
/// [`spawn_byte_stream_producer`]. The `Crypter` is built inside the task with
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
}

/// Shared retry/backoff loop for the cloud upload paths. The async closure
/// rebuilds the request per attempt (re-opening the source off the runtime
/// thread via `body_for`) and may fail (e.g. a per-retry file open) — failures
/// are non-retryable and surface via `adapter.on_build_err`.
///
// TODO(SNOW-3780594): this duplicates the budget/backoff/timeout logic in
// `http::retry::execute_with_retry`; S3 conditional-conflict replay now has a
// second policy-driven consumer in `S3ConflictRetryBudget`. Consolidation must
// share one per-transfer deadline across request retries and whole-write
// replays rather than letting each retry layer arm a fresh `max_elapsed`.
// Move both onto the shared retry loop once it supports the per-attempt request
// rebuild these paths need.
pub(super) async fn upload_with_retry<F, M>(
    policy: &RetryPolicy,
    adapter: &M,
    method: &reqwest::Method,
    url: &str,
    build_request: F,
) -> Result<(), M::Err>
where
    F: AsyncFn() -> Result<reqwest::RequestBuilder, M::BuildErr>,
    M: UploadRetryAdapter,
{
    let max_attempts = policy.max_attempts;
    let start = Instant::now();
    let mut sleep_ms = policy.backoff.base.as_millis() as f64;

    // Only the host + path may be logged (ud-log-every-http-call-at-info); the
    // query string can carry a SAS token / signature, so strip it.
    let log_path = url.split(['?', '#']).next().unwrap_or("");

    for attempt in 1..=max_attempts {
        let remaining = if let Some(budget) = policy.max_elapsed {
            let elapsed = start.elapsed();
            if elapsed >= budget {
                return Err(adapter.on_exhausted(format!(
                    "deadline exceeded after {elapsed:?} (budget {budget:?})"
                )));
            }
            Some(budget - elapsed)
        } else {
            None
        };
        let timeout = match (policy.per_request_timeout, remaining) {
            (Some(prt), Some(rem)) => prt.min(rem),
            (Some(prt), None) => prt,
            (None, Some(rem)) => rem.min(Duration::from_secs(REQUEST_TIMEOUT_SECS)),
            (None, None) => Duration::from_secs(REQUEST_TIMEOUT_SECS),
        };

        let req = match build_request().await {
            Ok(r) => r.timeout(timeout),
            Err(e) => return Err(adapter.on_build_err(e)),
        };

        tracing::info!(method = %method, path = %log_path, attempt, "outbound HTTP call");

        match req.send().await {
            Ok(resp) => {
                tracing::info!(status = resp.status().as_u16(), "HTTP response");
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
                let delay = Duration::from_millis(sleep_ms as u64);
                sleep_ms = next_delay_ms(sleep_ms, &policy.backoff);
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                if attempt >= max_attempts {
                    return Err(adapter.on_transport(e));
                }
                let delay = Duration::from_millis(sleep_ms as u64);
                sleep_ms = next_delay_ms(sleep_ms, &policy.backoff);
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(adapter.on_exhausted(format!("upload exhausted {max_attempts} attempts")))
}
