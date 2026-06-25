//! Helpers shared by the GCS and Azure HTTP transfer paths.
//!
//! S3 isn't a caller — it goes through the AWS SDK, which has its own
//! retry/backoff and streaming-body machinery. The two reqwest-based
//! transports converge here so the manual exponential-backoff loop, the
//! async-stream → sync-`Read` bridge, and a couple of one-liner helpers
//! aren't reimplemented twice.

use super::encryption::Encryptor;
use super::types::{ByteSource, EncryptedFileMetadata};
use crate::config::retry::{BackoffConfig, RetryPolicy};
use bytes::Bytes;
use futures::StreamExt as _;
use futures::stream::Stream;
use reqwest::StatusCode;
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Read-buffer size in bytes for the streaming upload producer — one channel chunk.
const UPLOAD_CHUNK_SIZE_BYTES: usize = 64 * 1024;

/// Per-attempt HTTP timeout. Matches the cloud transfer modules' historical
/// 300s cap; the retry budget (`policy.max_elapsed`) must exceed this so at
/// least one full attempt can complete.
const REQUEST_TIMEOUT_SECS: u64 = 300;

/// Returns true when the HTTP status code should trigger a retry. Mirrors
/// `http::retry::should_retry_status` — kept inline so the cloud transfer
/// modules don't take an indirect dep just for the constant set.
pub(super) fn is_retryable_status(status: u16, extra: &[u16]) -> bool {
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
            tracing::warn!("Failed to read cloud error response body: {}", e);
            format!("<could not read body: {}>", e)
        }
    }
}

/// Sync `Read` adapter over a bounded mpsc channel of `reqwest::Bytes` results.
/// `read` blocks waiting for the next chunk if the producer hasn't sent one.
/// Used to bridge an async `bytes_stream()` from a reqwest response into the
/// sync decryption path that runs inside `tokio::task::spawn_blocking` in
/// `mod.rs`. `buf` holds the current unconsumed tail of the last received
/// chunk as a `Bytes` slice — advancing it is an O(1) reference-count update
/// with no per-chunk allocation.
///
/// `bytes_read` accumulates the running total of ciphertext (on-cloud,
/// pre-decryption) bytes pulled out of the stream. It is shared via
/// [`StreamReader::bytes_read_handle`] so the caller can recover the on-cloud
/// byte count after the reader is consumed — needed when the `Content-Length`
/// header is absent (chunked transfer encoding) and the decrypted plaintext
/// length would otherwise be misreported as the on-cloud size.
pub struct StreamReader {
    rx: std::sync::mpsc::Receiver<std::io::Result<Bytes>>,
    buf: Bytes,
    bytes_read: Arc<AtomicU64>,
}

impl StreamReader {
    fn new(rx: std::sync::mpsc::Receiver<std::io::Result<Bytes>>) -> Self {
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
            match self.rx.recv() {
                Ok(Ok(chunk)) => self.buf = chunk,
                Ok(Err(e)) => return Err(e),
                Err(_disconnected) => return Ok(0),
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
/// mpsc channel and returns the corresponding [`StreamReader`]. Channel
/// capacity is 8 chunks (≈2 MiB at typical 256 KiB chunks) — enough to keep
/// the producer busy while the consumer decrypts.
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
pub(super) fn spawn_byte_stream_producer(response: reqwest::Response) -> StreamReader {
    let (tx, rx) = std::sync::mpsc::sync_channel::<std::io::Result<Bytes>>(8);
    let stream = response.bytes_stream();
    tokio::spawn(async move {
        let mut stream = stream;
        while let Some(chunk_result) = stream.next().await {
            let mapped = chunk_result.map_err(std::io::Error::other);
            // If the consumer dropped (decryption finished/errored) while we
            // had a pending error, the error is silently lost — the consumer
            // already has its own failure to report. Log at debug so it's
            // recoverable from traces if downstream behaviour ever surprises.
            if let Err(send_err) = tx.send(mapped) {
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
    StreamReader::new(rx)
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
    /// Streaming body reader — feed to `decrypt_ciphertext_to_writer` or
    /// `std::io::copy` from a `spawn_blocking` task.
    pub reader: StreamReader,
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
pub(super) async fn upload_with_retry<F, M>(
    policy: &RetryPolicy,
    adapter: &M,
    build_request: F,
) -> Result<(), M::Err>
where
    F: AsyncFn() -> Result<reqwest::RequestBuilder, M::BuildErr>,
    M: UploadRetryAdapter,
{
    let max_attempts = policy.max_attempts;
    let start = Instant::now();
    let mut sleep_ms = policy.backoff.base.as_millis() as f64;

    for attempt in 1..=max_attempts {
        let elapsed = start.elapsed();
        if elapsed >= policy.max_elapsed {
            return Err(adapter.on_exhausted(format!(
                "deadline exceeded after {elapsed:?} (budget {:?})",
                policy.max_elapsed
            )));
        }
        let remaining = policy.max_elapsed - elapsed;
        let timeout = remaining.min(Duration::from_secs(REQUEST_TIMEOUT_SECS));

        let req = match build_request().await {
            Ok(r) => r.timeout(timeout),
            Err(e) => return Err(adapter.on_build_err(e)),
        };

        match req.send().await {
            Ok(resp) => {
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
