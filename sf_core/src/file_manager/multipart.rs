//! Foundation for cloud multipart upload + ranged download (S3, Azure, GCS).
//!
//! All three clouds get chunked transfer through one shared policy.
//!
//! Only the cloud-agnostic policy lives here — per-cloud limits
//! ([`MultipartConfig`]), the part-size formula ([`compute_part_size`]), and
//! the server-resolved knobs ([`MultipartThreshold`] / [`MultipartParams`]).
//! The streaming part-reader ([`spawn_part_reader`]) and ranged-GET planner
//! ([`plan_ranges`]) also live here, consumed by the per-cloud transfer
//! modules. S3 and Azure upload parts concurrently; GCS uses an XML-API
//! resumable session (sequential chunks), so it drives the part-reader at
//! concurrency 1.

use bytes::Bytes;
use snafu::{Location, Snafu};
use std::io::Read;
use tempfile::TempPath;
use tokio::sync::mpsc;

use super::encryption::Encryptor;
use super::types::{ByteSource, PreparedUpload};
use file_too_large_error::FileTooLargeSnafu;

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const GIB: u64 = 1024 * MIB;
const TIB: u64 = 1024 * GIB;

/// Upper bound on the number of parts transferred concurrently for a single
/// file. The server-resolved `data.parallel` is clamped to this so a runaway
/// value can't open hundreds of simultaneous connections; uploads are also
/// memory-bounded at roughly `concurrency * part_size` resident bytes.
pub(super) const MAX_PART_CONCURRENCY: usize = 16;

/// Part-transfer concurrency used when the server omits `data.parallel`.
/// Matches the conservative reference-driver fallback (Python resolves a
/// missing field to `1`; JDBC/libsnowflakeclient likewise fall back to a
/// sequential transfer). The SQL `PARALLEL` clause default of 4 is a *server*
/// concern — the server folds it into `data.parallel`, so this fallback only
/// fires when the response omits the field entirely.
const DEFAULT_PART_CONCURRENCY: usize = 1;

/// Per-cloud multipart limits, tabulated as consts so adding a cloud is one
/// struct literal rather than a new branch in every helper.
pub struct MultipartConfig {
    /// Cloud label, used only in `FileTooLarge` messages and logs.
    pub(super) cloud: &'static str,
    /// Part size used when the file fits in `max_parts` at this size.
    /// Picked to mirror each provider's idiomatic SDK default.
    pub(super) default_part: u64,
    /// Lower bound the cloud enforces on every part except the last.
    pub(super) min_part: u64,
    /// Upper bound the cloud enforces on a single part.
    pub(super) max_part: u64,
    /// Upper bound the cloud enforces on the whole object.
    pub(super) max_object: u64,
    /// Upper bound the cloud enforces on the number of parts per upload, or
    /// `None` when the cloud imposes none (GCS resumable). When set,
    /// [`compute_part_size`] grows the part size to keep the count within it.
    pub(super) max_parts: Option<u64>,
}

impl MultipartConfig {
    /// S3 limits (mirror the Python connector's `s3_storage_client` and the S3
    /// multipart API): 8 MiB default part, 5 MiB floor, 5 GiB part ceiling,
    /// 5 TiB object ceiling, 10 000 parts. The floor/part/count limits are the
    /// AWS service limits; the 8 MiB default is the reference-driver choice, and
    /// 5 TiB is the conventional object ceiling (AWS now allows more).
    /// <https://docs.aws.amazon.com/AmazonS3/latest/userguide/qfacts.html>
    pub const S3: Self = Self {
        cloud: "S3",
        default_part: 8 * MIB,
        min_part: 5 * MIB,
        max_part: 5 * GIB,
        max_object: 5 * TIB,
        max_parts: Some(10_000),
    };
    /// Azure block-blob limits: 8 MiB default block (matches the S3/GCS
    /// default, and the legacy connector's post-fix default —
    /// snowflakedb/snowflake-connector-python#2982), 100 MiB block ceiling,
    /// ~4.77 TiB object ceiling, 50 000 blocks. The 100 MiB block / 50 000 block
    /// limits are Azure's for the conservative `2016-05-31`..`2019-07-07` API
    /// versions (newer ones allow 4000 MiB blocks → ~190 TiB); `max_object` is
    /// the product of the two. [`compute_part_size`] grows the block past the
    /// 8 MiB default once a file would otherwise need more than 50 000 blocks
    /// (past ~391 GiB), so large blobs stay within the block-count limit instead
    /// of failing. (libsnowflakeclient grows the block the same way; JDBC keeps
    /// it fixed at 4 MiB.)
    /// <https://learn.microsoft.com/en-us/rest/api/storageservices/put-block>
    pub const AZURE: Self = Self {
        cloud: "Azure",
        default_part: 8 * MIB,
        min_part: 1,
        max_part: 100 * MIB,
        max_object: 100 * MIB * 50_000,
        max_parts: Some(50_000),
    };
    /// GCS XML-API resumable limits: 8 MiB default chunk, 5 TiB object ceiling.
    /// GCS resumable imposes **no chunk-count limit** (the binding cap is the
    /// 5 TiB object size), so `max_parts` is `None` and [`compute_part_size`]
    /// never grows the chunk — it stays at the 8 MiB default for every file.
    /// That default is 32 × 256 KiB, satisfying GCS's requirement that every
    /// non-final chunk be a 256-KiB multiple; `gcs_part_is_256kib_aligned` pins
    /// this.
    /// Object size: <https://cloud.google.com/storage/quotas>
    /// 256-KiB rule: <https://cloud.google.com/storage/docs/performing-resumable-uploads>
    pub const GCS: Self = Self {
        cloud: "GCS",
        default_part: 8 * MIB,
        min_part: 256 * KIB,
        max_part: 5 * GIB,
        max_object: 5 * TIB,
        max_parts: None,
    };

    /// Number of parts (upload) or ranges (download) a transfer of
    /// `file_size` bytes takes under this cloud's part-size policy —
    /// `ceil(file_size / compute_part_size(file_size, self))`. Test-only: the
    /// fields above are `pub(super)` on purpose, so rather than widening them
    /// for external callers, this gives live/e2e tests (reachable only via
    /// the `internal` test-utils module) the expected count directly.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn expected_part_count(&self, file_size: u64) -> Result<u64, FileTooLargeError> {
        let part_size = compute_part_size(file_size, self)?;
        Ok(file_size.div_ceil(part_size))
    }
}

/// Picks the part size for `file_size`: start from `default_part` and grow it
/// just enough that `ceil(file_size / part) <= max_parts`, never below
/// `min_part`. Errors when the file exceeds the cloud's `max_object`.
///
/// For S3 and Azure this mirrors Python's `_chunk_size_calculator`: the part
/// grows once a file would otherwise exceed the cloud's part-count limit (for
/// Azure, past the 8 MiB default block, keeping blobs larger than ~391 GiB
/// within the 50 000-block limit). GCS has no part-count limit
/// (`max_parts == None`), so the chunk never grows and stays at `default_part`.
///
/// `file_size` is the *on-cloud* byte count — ciphertext length for CSE,
/// source length for SSE — because that is what gets split into parts.
pub fn compute_part_size(file_size: u64, cfg: &MultipartConfig) -> Result<u64, FileTooLargeError> {
    if file_size > cfg.max_object {
        return FileTooLargeSnafu {
            actual_bytes: file_size,
            limit_bytes: cfg.max_object,
            cloud: cfg.cloud,
        }
        .fail();
    }
    // Smallest part size that keeps the part count within `max_parts` (when the
    // cloud imposes one), floored at the cloud minimum, then bumped to at least
    // the default. `div_ceil` rounds the part count up so the last (short) part
    // is always covered.
    let by_max_parts = match cfg.max_parts {
        Some(max_parts) => file_size.div_ceil(max_parts).max(cfg.min_part),
        None => cfg.min_part,
    };
    let chosen = by_max_parts.max(cfg.default_part);
    // Holds whenever `max_object <= max_parts * max_part` (and trivially when
    // there is no part limit); the assert guards against future config drift,
    // not runtime input.
    debug_assert!(chosen <= cfg.max_part, "part size exceeded max_part");
    Ok(chosen)
}

/// File-size threshold at or above which a transfer switches from a single
/// PUT/GET to multipart. Resolved from the server's `data.threshold`,
/// defaulting to 200 MiB (the JDBC `bigFileThreshold` / libsnowflakeclient
/// `DEFAULT_UPLOAD_DATA_SIZE_THRESHOLD`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultipartThreshold(u64);

impl MultipartThreshold {
    /// Reference-driver default (JDBC `bigFileThreshold`, libsnowflakeclient
    /// `DEFAULT_UPLOAD_DATA_SIZE_THRESHOLD`).
    pub const DEFAULT: u64 = 200 * MIB;

    /// Resolves the server-supplied `data.threshold`, falling back to
    /// [`DEFAULT`](Self::DEFAULT) when it is missing or non-positive.
    pub fn from_server(threshold: Option<i64>) -> Self {
        match threshold {
            Some(t) if t > 0 => Self(t as u64),
            _ => Self(Self::DEFAULT),
        }
    }

    pub fn bytes(self) -> u64 {
        self.0
    }
}

/// Resolves the per-file part-transfer concurrency from the server's
/// `data.parallel`, defaulting to [`DEFAULT_PART_CONCURRENCY`] and clamping
/// into `1..=MAX_PART_CONCURRENCY`.
pub(super) fn resolve_part_concurrency(parallel: Option<i64>) -> usize {
    let requested = match parallel {
        Some(p) if p > 0 => p as usize,
        _ => DEFAULT_PART_CONCURRENCY,
    };
    requested.clamp(1, MAX_PART_CONCURRENCY)
}

/// Server-resolved multipart knobs carried alongside each transfer request,
/// bundled so the plumbing through `UploadData`/`DownloadData` is one field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultipartParams {
    pub threshold: MultipartThreshold,
    /// Concurrent part transfers, already clamped to `1..=MAX_PART_CONCURRENCY`.
    pub concurrency: usize,
}

impl MultipartParams {
    /// Resolves both knobs from the raw `data.threshold` / `data.parallel`
    /// fields of a PUT/GET server response.
    pub fn from_server(threshold: Option<i64>, parallel: Option<i64>) -> Self {
        Self {
            threshold: MultipartThreshold::from_server(threshold),
            concurrency: resolve_part_concurrency(parallel),
        }
    }

    /// True when a `body_len`-byte transfer should take the multipart/chunked
    /// path rather than a single PUT/GET (i.e. it is at or above the resolved
    /// threshold). Hides the `threshold` accessor so call sites don't reach
    /// through to the raw byte count.
    pub(super) fn should_chunk(self, body_len: u64) -> bool {
        body_len >= self.threshold.bytes()
    }
}

impl Default for MultipartParams {
    fn default() -> Self {
        Self::from_server(None, None)
    }
}

/// Raised when a source file exceeds a cloud's `max_object` limit. The S3 /
/// Azure transfer modules wrap this into their own `Upload*Error` enums, which
/// are themselves `pub` (reached via `FileManagerError`), so this is `pub` too
/// to keep the error chain nameable end-to-end.
#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module, visibility(pub(crate)))]
pub enum FileTooLargeError {
    #[snafu(display(
        "File too large for {cloud} multipart upload: {actual_bytes} bytes exceeds limit {limit_bytes}"
    ))]
    FileTooLarge {
        actual_bytes: u64,
        limit_bytes: u64,
        cloud: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
}

// ---------------------------------------------------------------------------
// Upload: sequential part reader
// ---------------------------------------------------------------------------

/// On-cloud byte count of a prepared upload: the analytic ciphertext length
/// for CSE, or the source length for SSE. This is what gets split into parts,
/// so it (not the plaintext size) drives the multipart threshold and chunk
/// sizing on every cloud.
pub(super) async fn upload_body_len(prepared: &PreparedUpload) -> std::io::Result<u64> {
    if let Some(cse) = prepared.cse.as_ref() {
        return Ok(cse.encryptor.cipher_len() as u64);
    }
    match prepared.source.byte_source() {
        ByteSource::Bytes(b) => Ok(b.len() as u64),
        // Offload the stat to tokio's blocking pool: a slow stat on a networked
        // filesystem (NFS, EBS) must not stall the async runtime thread — mirrors
        // `ByteSource::open_async`.
        ByteSource::Path(p) => Ok(tokio::fs::metadata(p).await?.len()),
    }
}

/// One upload part: its 1-based number and the owned body bytes. Sized at the
/// resolved chunk size, except the final part which carries the remainder.
pub(super) struct UploadPart {
    /// 1-based part number (S3 `partNumber`; Azure derives a block id from it).
    pub number: i32,
    pub body: Bytes,
}

/// Reads `source` (lazily AES-CBC-encrypting it when `encryptor` is `Some`)
/// into `chunk_size`-byte [`UploadPart`]s, sending each over a bounded channel
/// as it is produced. The blocking read loop runs on `spawn_blocking` so file
/// I/O and AES-CBC encryption stay off the async runtime; the `EncryptingReader`
/// (and its non-`Send` OpenSSL `Crypter`) is built inside the task and never
/// crosses a thread boundary, mirroring `cloud_http::encrypting_body_stream`.
///
/// Reads are **sequential and in order**: the CSE body is an AES-CBC stream
/// whose part N only exists once parts `0..N` have been produced, so the
/// splitter cannot seek. Concurrency happens on the consumer (upload) side;
/// the channel bound (`concurrency`) caps read-ahead, keeping resident memory
/// at roughly `concurrency * chunk_size`. This mirrors libsnowflakeclient's
/// `StreamSplitter` (serial reads, parallel part uploads).
pub(super) fn spawn_part_reader(
    source: ByteSource,
    encryptor: Option<Encryptor>,
    chunk_size: usize,
    concurrency: usize,
) -> mpsc::Receiver<std::io::Result<UploadPart>> {
    let (tx, rx) = mpsc::channel(concurrency.max(1));
    tokio::task::spawn_blocking(move || {
        // Open (and, for CSE, wrap) the source inside the task so the reader
        // stays thread-local. A failure here surfaces as the first item.
        let mut reader: Box<dyn Read> = match open_part_source(source, encryptor) {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.blocking_send(Err(e));
                return;
            }
        };
        let mut number: i32 = 0;
        loop {
            let mut buf = vec![0u8; chunk_size];
            match fill_chunk(reader.as_mut(), &mut buf) {
                // Clean EOF exactly on a part boundary: nothing left to send.
                Ok(0) => break,
                Ok(n) => {
                    buf.truncate(n);
                    number += 1;
                    let part = UploadPart {
                        number,
                        body: Bytes::from(buf),
                    };
                    if tx.blocking_send(Ok(part)).is_err() {
                        break; // consumer dropped (an upload errored / aborted)
                    }
                    // A short read means the source hit EOF; the next read
                    // would return 0, so stop without spinning a final read.
                    if n < chunk_size {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.blocking_send(Err(e));
                    break;
                }
            }
        }
    });
    rx
}

/// Opens `source` for reading, wrapping it in an [`EncryptingReader`] when a
/// CSE `encryptor` is present. Runs on the blocking part-reader task.
fn open_part_source(
    source: ByteSource,
    encryptor: Option<Encryptor>,
) -> std::io::Result<Box<dyn Read>> {
    let base = source.open()?;
    match encryptor {
        Some(enc) => Ok(Box::new(
            enc.encrypting_reader(base).map_err(std::io::Error::other)?,
        )),
        None => Ok(base),
    }
}

/// Reads until `buf` is full or the source reaches EOF, coping with short
/// reads (and retrying on `Interrupted`). Returns the number of bytes placed
/// in `buf` — less than `buf.len()` only at EOF.
fn fill_chunk(reader: &mut dyn Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

// ---------------------------------------------------------------------------
// Download: ranged-GET planning + positioned writes
// ---------------------------------------------------------------------------

/// One ranged GET: an inclusive byte range `[start, end]`. `start` doubles as
/// the destination write offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DownloadRange {
    pub start: u64,
    /// Inclusive end offset, as required by the HTTP `Range: bytes=start-end`
    /// header and S3/Azure range semantics.
    pub end: u64,
}

/// Splits a `content_length`-byte object into inclusive byte ranges of at most
/// `chunk_size` (the last range carries the remainder). Mirrors the ranged-GET
/// planners in the Python and libsnowflakeclient connectors.
pub(super) fn plan_ranges(content_length: u64, chunk_size: u64) -> Vec<DownloadRange> {
    let mut ranges = Vec::new();
    let mut start = 0u64;
    while start < content_length {
        let end = (start + chunk_size).min(content_length) - 1;
        ranges.push(DownloadRange { start, end });
        start = end + 1;
    }
    ranges
}

/// Writes `data` at absolute `offset` in `file` without using or disturbing
/// the file cursor, so concurrent writers targeting disjoint ranges of a
/// pre-allocated file are safe. The positioned-write syscall (`pwrite` /
/// `seek_write`) is the cross-platform equivalent of the per-chunk
/// seek-then-write the Python connector performs.
#[cfg(unix)]
pub(super) fn write_at(file: &std::fs::File, offset: u64, data: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.write_all_at(data, offset)
}

#[cfg(windows)]
pub(super) fn write_at(file: &std::fs::File, offset: u64, data: &[u8]) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    let mut written = 0usize;
    while written < data.len() {
        written += file.seek_write(&data[written..], offset + written as u64)?;
    }
    Ok(())
}

/// Blocking `Read` over a ciphertext tempfile produced by a ranged download,
/// keeping the `TempPath` alive (the file is unlinked on drop) for the read's
/// duration. Shared by the S3 and Azure download paths; works on Windows,
/// which can't unlink an open file. `Send`, so it can hand off to the
/// `spawn_blocking` decrypt step.
pub(super) struct SpilledReader {
    file: std::fs::File,
    _temp: TempPath,
}

impl SpilledReader {
    /// Opens `temp` for reading and takes ownership of its unlink-on-drop guard.
    pub fn open(temp: TempPath) -> std::io::Result<Self> {
        let file = std::fs::File::open(&temp)?;
        Ok(Self { file, _temp: temp })
    }
}

impl Read for SpilledReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_size_is_default_below_recompute_threshold() {
        // Small files — and files up to default_part * max_parts — keep the
        // per-cloud default part size.
        assert_eq!(
            compute_part_size(MIB, &MultipartConfig::S3).unwrap(),
            MultipartConfig::S3.default_part
        );
        assert_eq!(
            compute_part_size(7 * GIB, &MultipartConfig::S3).unwrap(),
            MultipartConfig::S3.default_part
        );
        // Azure shares the same 8 MiB default as S3/GCS in this regime.
        assert_eq!(
            compute_part_size(MIB, &MultipartConfig::AZURE).unwrap(),
            MultipartConfig::AZURE.default_part
        );
    }

    #[test]
    fn part_size_grows_to_stay_within_max_parts() {
        // 8 MiB * 10 000 = 80 GiB is the boundary; one byte over forces the
        // recompute path to bump the part size up.
        let cfg = &MultipartConfig::S3;
        let max_parts = cfg.max_parts.expect("S3 has a part-count limit");
        let boundary = cfg.default_part * max_parts;
        assert_eq!(compute_part_size(boundary, cfg).unwrap(), cfg.default_part);

        let over = boundary + 1;
        let chosen = compute_part_size(over, cfg).unwrap();
        assert!(chosen > cfg.default_part, "part size must have grown");
        assert!(
            over.div_ceil(chosen) <= max_parts,
            "part count must stay within max_parts"
        );
    }

    #[test]
    fn part_size_stays_within_cloud_limits_at_max_object() {
        // At the exact max-object boundary the chosen part must respect the
        // floor, the per-part ceiling, and the part-count ceiling for every
        // cloud (clouds with no part-count limit only have the first two).
        for cfg in [
            &MultipartConfig::S3,
            &MultipartConfig::AZURE,
            &MultipartConfig::GCS,
        ] {
            let chosen = compute_part_size(cfg.max_object, cfg).unwrap();
            assert!(chosen >= cfg.min_part);
            assert!(chosen <= cfg.max_part);
            if let Some(max_parts) = cfg.max_parts {
                assert!(cfg.max_object.div_ceil(chosen) <= max_parts);
            }
        }
    }

    #[test]
    fn part_size_errors_above_max_object() {
        for cfg in [
            &MultipartConfig::S3,
            &MultipartConfig::AZURE,
            &MultipartConfig::GCS,
        ] {
            let over = cfg.max_object + 1;
            let err = compute_part_size(over, cfg).unwrap_err();
            let FileTooLargeError::FileTooLarge {
                actual_bytes,
                limit_bytes,
                cloud,
                ..
            } = err;
            assert_eq!(actual_bytes, over);
            assert_eq!(limit_bytes, cfg.max_object);
            assert_eq!(cloud, cfg.cloud);
        }
    }

    #[test]
    fn gcs_part_is_256kib_aligned() {
        // GCS resumable requires every non-final chunk to be a 256-KiB multiple.
        // The config is tuned so `compute_part_size` never grows the chunk past
        // the 8-MiB default for any file ≤ 5 TiB, so the resolved size is always
        // a 256-KiB multiple. Check across small / threshold / max-object sizes.
        const GRANULARITY: u64 = 256 * KIB;
        for size in [KIB, 64 * MIB, 8 * GIB, MultipartConfig::GCS.max_object] {
            let chunk = compute_part_size(size, &MultipartConfig::GCS).unwrap();
            assert_eq!(
                chunk % GRANULARITY,
                0,
                "GCS chunk {chunk} for file_size {size} must be a 256-KiB multiple"
            );
            assert_eq!(chunk, 8 * MIB, "GCS chunk should stay at the 8-MiB default");
        }
    }

    #[test]
    fn threshold_defaults_when_missing_or_non_positive() {
        assert_eq!(MultipartThreshold::from_server(None).bytes(), 200 * MIB);
        assert_eq!(MultipartThreshold::from_server(Some(0)).bytes(), 200 * MIB);
        assert_eq!(MultipartThreshold::from_server(Some(-5)).bytes(), 200 * MIB);
        assert_eq!(MultipartThreshold::from_server(Some(100)).bytes(), 100);
    }

    #[test]
    fn part_concurrency_defaults_and_clamps() {
        assert_eq!(resolve_part_concurrency(None), DEFAULT_PART_CONCURRENCY);
        assert_eq!(resolve_part_concurrency(Some(0)), DEFAULT_PART_CONCURRENCY);
        assert_eq!(resolve_part_concurrency(Some(1)), 1);
        assert_eq!(resolve_part_concurrency(Some(8)), 8);
        // Clamped to the ceiling.
        assert_eq!(resolve_part_concurrency(Some(1000)), MAX_PART_CONCURRENCY);
    }

    #[test]
    fn params_from_server_routes_each_field() {
        // Distinguishable values catch a future threshold/parallel arg swap
        // that the type system can't (both are `Option<i64>`).
        let params = MultipartParams::from_server(Some(100), Some(8));
        assert_eq!(params.threshold.bytes(), 100);
        assert_eq!(params.concurrency, 8);

        // Default routes through from_server, so the defaults live in one place.
        assert_eq!(
            MultipartParams::default(),
            MultipartParams::from_server(None, None)
        );
        assert_eq!(MultipartParams::default().threshold.bytes(), 200 * MIB);
        assert_eq!(
            MultipartParams::default().concurrency,
            DEFAULT_PART_CONCURRENCY
        );
    }

    /// Collects every part the reader produces, asserting numbering and that
    /// the concatenated bodies reproduce the source byte-for-byte.
    async fn collect_parts(data: Vec<u8>, chunk_size: usize) -> Vec<UploadPart> {
        let source = ByteSource::Bytes(Bytes::from(data));
        let mut rx = spawn_part_reader(source, None, chunk_size, 4);
        let mut parts = Vec::new();
        while let Some(part) = rx.recv().await {
            parts.push(part.expect("part read must not error"));
        }
        parts
    }

    #[tokio::test]
    async fn part_reader_splits_into_numbered_chunks() {
        let data: Vec<u8> = (0..25u8).collect();
        let parts = collect_parts(data.clone(), 10).await;

        // 25 bytes / 10 => parts of 10, 10, 5.
        let sizes: Vec<usize> = parts.iter().map(|p| p.body.len()).collect();
        assert_eq!(sizes, vec![10, 10, 5]);
        // 1-based, contiguous numbering.
        let numbers: Vec<i32> = parts.iter().map(|p| p.number).collect();
        assert_eq!(numbers, vec![1, 2, 3]);
        // Reassembly is byte-identical to the source.
        let joined: Vec<u8> = parts.iter().flat_map(|p| p.body.to_vec()).collect();
        assert_eq!(joined, data);
    }

    #[tokio::test]
    async fn part_reader_exact_multiple_has_no_trailing_empty_part() {
        // 20 bytes / 10 => exactly two full parts and no empty third part
        // (the libsnowflakeclient `+1` off-by-one is intentionally avoided).
        let parts = collect_parts((0..20u8).collect(), 10).await;
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|p| p.body.len() == 10));
    }

    #[test]
    fn plan_ranges_covers_object_with_inclusive_bounds() {
        let ranges = plan_ranges(25, 10);
        let pairs: Vec<(u64, u64)> = ranges.iter().map(|r| (r.start, r.end)).collect();
        assert_eq!(pairs, vec![(0, 9), (10, 19), (20, 24)]);
        // Ranges tile the object with no gaps or overlap, last one short.
        let total: u64 = ranges.iter().map(|r| r.end - r.start + 1).sum();
        assert_eq!(total, 25);
    }

    #[test]
    fn plan_ranges_exact_multiple() {
        let ranges = plan_ranges(20, 10);
        assert_eq!(ranges.len(), 2);
        assert_eq!((ranges[1].start, ranges[1].end), (10, 19));
    }

    #[test]
    fn write_at_places_disjoint_chunks_at_offsets() {
        let f = tempfile::NamedTempFile::new().unwrap();
        f.as_file().set_len(6).unwrap();
        // Write out of order; positioned writes must land at the right offset.
        write_at(f.as_file(), 3, b"DEF").unwrap();
        write_at(f.as_file(), 0, b"ABC").unwrap();
        assert_eq!(std::fs::read(f.path()).unwrap(), b"ABCDEF");
    }
}
