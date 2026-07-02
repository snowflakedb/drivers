//! Foundation for cloud multipart upload + ranged download (S3, Azure, GCS).
//!
//! All three clouds get chunked transfer through one shared policy.
//!
//! Only the cloud-agnostic policy lives here — per-cloud limits
//! ([`MultipartConfig`]), the part-size formula ([`compute_part_size`]), and
//! the server-resolved knobs ([`MultipartThreshold`] / [`MultipartParams`]).
//! The streaming part-reader (upload) and ranged-GET planner (download) are
//! added by the per-cloud consumer PRs. S3 and Azure upload parts
//! concurrently; GCS uses an XML-API resumable session (sequential chunks),
//! so it drives the part-reader at concurrency 1.
#![allow(dead_code)] // Consumed by the S3 / Azure multipart PRs stacked on this one.

use snafu::{Location, Snafu};

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
pub(super) struct MultipartConfig {
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
    pub(super) const S3: Self = Self {
        cloud: "S3",
        default_part: 8 * MIB,
        min_part: 5 * MIB,
        max_part: 5 * GIB,
        max_object: 5 * TIB,
        max_parts: Some(10_000),
    };
    /// Azure block-blob limits: 4 MiB default block, 100 MiB block ceiling,
    /// ~4.77 TiB object ceiling, 50 000 blocks. The 100 MiB block / 50 000 block
    /// limits are Azure's for the conservative `2016-05-31`..`2019-07-07` API
    /// versions (newer ones allow 4000 MiB blocks → ~190 TiB); `max_object` is
    /// the product of the two. [`compute_part_size`] grows the block past the
    /// 4 MiB default once a file would otherwise need more than 50 000 blocks
    /// (past ~195 GiB), so large blobs stay within the block-count limit instead
    /// of failing. (libsnowflakeclient grows the block the same way; Python/JDBC
    /// keep it fixed at 4 MiB.)
    /// <https://learn.microsoft.com/en-us/rest/api/storageservices/put-block>
    pub(super) const AZURE: Self = Self {
        cloud: "Azure",
        default_part: 4 * MIB,
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
    /// this. `max_part` (5 GiB) is inert — the chunk never grows to reach it —
    /// and only backs the `compute_part_size` debug assertion.
    /// Object size: <https://cloud.google.com/storage/quotas>
    /// 256-KiB rule: <https://cloud.google.com/storage/docs/performing-resumable-uploads>
    pub(super) const GCS: Self = Self {
        cloud: "GCS",
        default_part: 8 * MIB,
        min_part: 256 * KIB,
        max_part: 5 * GIB,
        max_object: 5 * TIB,
        max_parts: None,
    };
}

/// Picks the part size for `file_size`: start from `default_part` and grow it
/// just enough that `ceil(file_size / part) <= max_parts`, never below
/// `min_part`. Errors when the file exceeds the cloud's `max_object`.
///
/// For S3 and Azure this mirrors Python's `_chunk_size_calculator`: the part
/// grows once a file would otherwise exceed the cloud's part-count limit (for
/// Azure, past the fixed 4 MiB block Python/JDBC use, keeping blobs larger than
/// ~195 GiB within the 50 000-block limit). GCS has no part-count limit
/// (`max_parts == None`), so the chunk never grows and stays at `default_part`.
///
/// `file_size` is the *on-cloud* byte count — ciphertext length for CSE,
/// source length for SSE — because that is what gets split into parts.
pub(super) fn compute_part_size(
    file_size: u64,
    cfg: &MultipartConfig,
) -> Result<u64, FileTooLargeError> {
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
}

impl Default for MultipartParams {
    fn default() -> Self {
        Self::from_server(None, None)
    }
}

/// Raised when a source file exceeds a cloud's `max_object` limit. The per-cloud
/// transfer modules wrap this into their own `Upload*Error` enums.
#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module, visibility(pub(crate)))]
pub(crate) enum FileTooLargeError {
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
        // Azure keeps its smaller 4 MiB default in the same regime.
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
}
