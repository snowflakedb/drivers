use crate::file_manager::types::ByteSource;
use flate2::{Compression, GzBuilder};
use snafu::{Location, ResultExt, Snafu};
use std::io::{BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::{NamedTempFile, TempPath};

// 64 KiB read buffer for streaming the source into the gzip encoder.
const GZIP_CHUNK_SIZE_IN_BYTES: usize = 64 * 1024;

const MAX_GZIP_COMPRESS_LEVEL: u32 = 9;

/// Maps a connection-level `put_compress_level` onto a gzip level. Unset and
/// out-of-range values use `default`.
pub fn clamp_gzip_compress_level(level: Option<i64>, default: u32) -> u32 {
    match level {
        Some(n) if (0..=i64::from(MAX_GZIP_COMPRESS_LEVEL)).contains(&n) => n as u32,
        _ => default,
    }
}

/// Streams the gzip-compressed form of `source` into a `NamedTempFile` and
/// returns its path plus the `TempPath` guard. The caller owns the `TempPath`
/// and must keep it alive while the path is in use (the file is unlinked when
/// it drops). `gzip_level` is a gzip compression level in 0–9.
///
/// Peak **heap** is `O(GZIP_CHUNK_SIZE_IN_BYTES)` regardless of input size. The
/// tempfile lives in `temp_dir` when that is `Some`, otherwise
/// `std::env::temp_dir()`. Nested directories in `temp_dir` are created. A
/// writable temp dir is required — even for an in-memory `Bytes` source (a new
/// failure mode for in-memory auto-compress) — and on a RAM-backed tmpfs the
/// compressed output still occupies ~its own size in RAM (the heap bound is
/// not a total-memory bound).
pub fn compress_to_tempfile(
    source: &ByteSource,
    gzip_level: u32,
    temp_dir: Option<&Path>,
) -> Result<(PathBuf, TempPath), CompressionError> {
    // `Box<dyn Read + '_>` borrows from `source` for the `Bytes` arm so we
    // don't clone the buffer just to feed the encoder.
    let mut reader: Box<dyn Read + '_> = match source {
        ByteSource::Path(p) => Box::new(std::fs::File::open(p).context(IoFailedSnafu {
            operation: "opening source file for compression",
        })?),
        ByteSource::Bytes(b) => Box::new(Cursor::new(b.as_ref())),
    };

    let temp_file = match temp_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir).context(IoFailedSnafu {
                operation: "creating PUT compression temp directory",
            })?;
            NamedTempFile::new_in(dir).context(IoFailedSnafu {
                operation: "creating gzip tempfile",
            })?
        }
        None => NamedTempFile::new().context(IoFailedSnafu {
            operation: "creating gzip tempfile",
        })?,
    };

    let mut buf = vec![0u8; GZIP_CHUNK_SIZE_IN_BYTES];
    {
        let writer = BufWriter::new(temp_file.as_file());
        // `mtime=0` pins the gzip header timestamp so the output is byte-identical
        // across runs over the same input (deterministic digests).
        let mut encoder = GzBuilder::new()
            .mtime(0)
            .write(writer, Compression::new(gzip_level));

        loop {
            let n = reader.read(&mut buf).context(IoFailedSnafu {
                operation: "reading source for compression",
            })?;
            if n == 0 {
                break;
            }
            encoder.write_all(&buf[..n]).context(IoFailedSnafu {
                operation: "writing compressed chunk to tempfile",
            })?;
        }

        let mut writer = encoder.finish().context(IoFailedSnafu {
            operation: "finalizing gzip encoder",
        })?;
        writer.flush().context(IoFailedSnafu {
            operation: "flushing gzip tempfile",
        })?;
    }

    let temp_path = temp_file.into_temp_path();
    Ok((temp_path.to_path_buf(), temp_path))
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum CompressionError {
    #[snafu(display("I/O error during {operation}"))]
    IoFailed {
        operation: &'static str,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn read_compressed(path: &PathBuf) -> Vec<u8> {
        std::fs::read(path).expect("read compressed tempfile")
    }

    /// Test-only gunzip: `compress_to_tempfile` is the only production
    /// compression entry point left in this module (the streaming GET path
    /// now decompresses inline via `open_*_download_stream`), so round-trip
    /// coverage here inflates the compressed tempfile itself rather than
    /// calling back into production code.
    fn gunzip(compressed: &[u8]) -> Vec<u8> {
        use flate2::bufread::GzDecoder;
        let mut decoder = GzDecoder::new(compressed);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).expect("gunzip");
        decompressed
    }

    fn compress_bytes(payload: &[u8], level: u32) -> (PathBuf, tempfile::TempPath) {
        compress_to_tempfile(&ByteSource::Bytes(payload.to_vec().into()), level, None)
            .expect("compress bytes")
    }

    fn roundtrip(payload: &[u8]) {
        let (path, _guard) = compress_bytes(payload, 9);
        let compressed = read_compressed(&path);
        let decompressed = gunzip(&compressed);
        assert_eq!(
            decompressed,
            payload,
            "round-trip must reproduce input of length {}",
            payload.len(),
        );
    }

    #[test]
    fn roundtrip_empty() {
        roundtrip(b"");
    }

    #[test]
    fn roundtrip_small_payload() {
        roundtrip(b"hello world, this is a test payload");
    }

    #[test]
    fn roundtrip_at_chunk_boundaries() {
        // Boundary triple around a single chunk-size read; multi-chunk size.
        for size in [
            GZIP_CHUNK_SIZE_IN_BYTES - 1,
            GZIP_CHUNK_SIZE_IN_BYTES,
            GZIP_CHUNK_SIZE_IN_BYTES + 1,
            4 * GZIP_CHUNK_SIZE_IN_BYTES,
        ] {
            let payload: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
            roundtrip(&payload);
        }
    }

    #[test]
    fn roundtrip_path_source() {
        let payload: Vec<u8> = (0..100 * 1024).map(|i| (i % 251) as u8).collect();
        let mut tf = NamedTempFile::new().expect("input tempfile");
        tf.write_all(&payload).expect("write input");
        tf.flush().expect("flush input");

        let (path, _guard) =
            compress_to_tempfile(&ByteSource::Path(tf.path().to_path_buf()), 9, None)
                .expect("compress");
        let compressed = read_compressed(&path);
        let decompressed = gunzip(&compressed);
        assert_eq!(decompressed, payload);
    }

    /// `mtime=0` pinning means two runs over the same input produce
    /// byte-identical compressed output. Without it, the gzip header carries
    /// the current time and breaks reproducibility (and download digests).
    #[test]
    fn output_is_deterministic() {
        let payload: Vec<u8> = (0..32 * 1024).map(|i| (i % 251) as u8).collect();
        let (path_a, _guard_a) = compress_bytes(&payload, 9);
        let (path_b, _guard_b) = compress_bytes(&payload, 9);
        assert_eq!(
            read_compressed(&path_a),
            read_compressed(&path_b),
            "deterministic gzip output (mtime=0) required for stable digests",
        );
    }

    /// Dropping the `TempPath` unlinks the file. The `(PathBuf, TempPath)`
    /// pair pattern lets the caller own the keep-alive explicitly.
    #[test]
    fn tempfile_unlinks_when_guard_drops() {
        let (path, guard) = compress_bytes(b"unlink test", 9);
        assert!(path.exists(), "tempfile must exist while guard is held");
        drop(guard);
        assert!(!path.exists(), "tempfile must be unlinked once guard drops");
    }

    #[test]
    fn clamp_gzip_compress_level_keeps_valid_range() {
        assert_eq!(clamp_gzip_compress_level(Some(0), 9), 0);
        assert_eq!(clamp_gzip_compress_level(Some(1), 9), 1);
        assert_eq!(clamp_gzip_compress_level(Some(9), 9), 9);
    }

    #[test]
    fn clamp_gzip_compress_level_defaults_unset_and_invalid() {
        assert_eq!(clamp_gzip_compress_level(None, 9), 9);
        assert_eq!(clamp_gzip_compress_level(Some(-1), 9), 9);
        assert_eq!(clamp_gzip_compress_level(Some(10), 9), 9);
        assert_eq!(clamp_gzip_compress_level(None, 6), 6);
        assert_eq!(clamp_gzip_compress_level(Some(10), 6), 6);
    }

    #[test]
    fn gzip_level_one_is_larger_than_level_nine_for_compressible_input() {
        let payload: Vec<u8> = (0..32 * 1024).map(|i| (i % 13) as u8).collect();
        let (path_one, _guard_one) = compress_bytes(&payload, 1);
        let (path_nine, _guard_nine) = compress_bytes(&payload, 9);
        let level_one = read_compressed(&path_one);
        let level_nine = read_compressed(&path_nine);
        assert!(
            level_nine.len() < level_one.len(),
            "gzip level 9 ({}) must compress more than level 1 ({})",
            level_nine.len(),
            level_one.len()
        );
        assert_eq!(gunzip(&level_one), payload);
        assert_eq!(gunzip(&level_nine), payload);
    }

    #[test]
    fn tempfile_is_created_in_nested_configured_directory() {
        let root = tempfile::tempdir().expect("root tempdir");
        let nested = root.path().join("a").join("b").join("c");
        let (path, _guard) = compress_to_tempfile(
            &ByteSource::Bytes(b"nested".to_vec().into()),
            9,
            Some(&nested),
        )
        .expect("compress into nested dir");
        assert!(
            path.starts_with(&nested),
            "gzip tempfile {} must live under {}",
            path.display(),
            nested.display()
        );
        assert!(nested.is_dir());
    }

    #[test]
    fn configured_temp_dir_that_is_a_file_returns_io_error() {
        let root = tempfile::tempdir().expect("root tempdir");
        let not_a_dir = root.path().join("not-a-dir");
        std::fs::write(&not_a_dir, b"x").expect("write file");
        let err = compress_to_tempfile(
            &ByteSource::Bytes(b"payload".to_vec().into()),
            9,
            Some(&not_a_dir),
        )
        .expect_err("file path cannot be a temp dir");
        assert!(matches!(err, CompressionError::IoFailed { .. }));
    }

    #[test]
    fn tempfile_in_configured_dir_unlinks_when_guard_drops() {
        let root = tempfile::tempdir().expect("root tempdir");
        let (path, guard) = compress_to_tempfile(
            &ByteSource::Bytes(b"unlink configured".to_vec().into()),
            9,
            Some(root.path()),
        )
        .expect("compress");
        assert!(path.exists(), "tempfile must exist while guard is held");
        drop(guard);
        assert!(!path.exists(), "tempfile must be unlinked once guard drops");
    }

    #[test]
    fn shell_metacharacters_in_temp_dir_are_treated_as_path_components() {
        let root = tempfile::tempdir().expect("root tempdir");
        let sentinel = root.path().join("pwned");
        let injection_dir = root.path().join("dir;touch pwned");
        let (path, _guard) = compress_to_tempfile(
            &ByteSource::Bytes(b"payload".to_vec().into()),
            9,
            Some(&injection_dir),
        )
        .expect("compress into metacharacter path");
        assert!(
            injection_dir.is_dir(),
            "metacharacters must be part of the directory name"
        );
        assert!(
            path.starts_with(&injection_dir),
            "gzip tempfile {} must live under {}",
            path.display(),
            injection_dir.display()
        );
        assert!(
            !sentinel.exists(),
            "shell metacharacters must not be executed"
        );
    }
}
