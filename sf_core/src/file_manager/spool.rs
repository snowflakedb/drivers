//! Hybrid in-memory/on-disk spool for chunked upload streaming.
//!
//! The upload path needs a re-readable, seekable source (digest is computed
//! once, but the body is re-streamed on every retry). `SpooledBuffer` buffers
//! chunks in memory below [`SPOOL_MEM_THRESHOLD`], then spills to a
//! `NamedTempFile` past it — mirroring JDBC's `FileBackedOutputStream` (same
//! 128 MiB threshold).

use std::io::{self, Write};

use tempfile::{NamedTempFile, TempPath};

use super::types::ByteSource;

/// Threshold above which [`SpooledBuffer`] spills from memory to a temp file.
/// Matches JDBC's `FileBackedOutputStream.MAX_BUFFER_SIZE` (128 MiB).
pub(crate) const SPOOL_MEM_THRESHOLD: usize = 1 << 27;

/// Reassembles chunked upload data into a re-readable source: starts as an
/// in-memory buffer, spills to a `NamedTempFile` once a chunk would push the
/// total past the spill threshold. Never reverts from `File` back to `Mem`.
pub(crate) enum SpooledBuffer {
    Mem(Vec<u8>),
    File(NamedTempFile),
}

impl Default for SpooledBuffer {
    fn default() -> Self {
        SpooledBuffer::Mem(Vec::new())
    }
}

impl SpooledBuffer {
    /// Appends `chunk`, spilling to a temp file first if it would push the
    /// buffer past `threshold`. Threshold is a parameter (not baked in) so
    /// tests can exercise the mem-to-file flip without allocating
    /// `SPOOL_MEM_THRESHOLD` bytes.
    pub(crate) fn write_all_with_threshold(
        &mut self,
        chunk: &[u8],
        threshold: usize,
    ) -> io::Result<()> {
        match self {
            SpooledBuffer::File(file) => return file.write_all(chunk),
            SpooledBuffer::Mem(buf) if buf.len() + chunk.len() <= threshold => {
                buf.extend_from_slice(chunk);
                return Ok(());
            }
            SpooledBuffer::Mem(_) => {}
        }

        // `self` is `Mem` and over threshold here — spill it plus `chunk`
        // into a fresh temp file.
        let SpooledBuffer::Mem(buf) = std::mem::replace(self, SpooledBuffer::Mem(Vec::new()))
        else {
            unreachable!("checked above: self is Mem at this point");
        };
        let mut file = NamedTempFile::new()?;
        file.write_all(&buf)?;
        file.write_all(chunk)?;
        *self = SpooledBuffer::File(file);
        Ok(())
    }

    /// Consumes the buffer into a [`ByteSource`] for the upload path. `Mem`
    /// becomes `ByteSource::Bytes` directly. `File` returns a `TempPath`
    /// guard the caller must keep alive until upload completes (it unlinks
    /// on drop), alongside the wrapped `ByteSource::Path`.
    pub(crate) fn into_source(self) -> (ByteSource, Option<TempPath>) {
        match self {
            SpooledBuffer::Mem(buf) => (ByteSource::Bytes(buf.into()), None),
            SpooledBuffer::File(file) => {
                let temp_path = file.into_temp_path();
                let path = temp_path.to_path_buf();
                (ByteSource::Path(path), Some(temp_path))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn stays_in_memory_under_threshold() {
        let mut buffer = SpooledBuffer::default();
        buffer.write_all_with_threshold(b"hello", 16).unwrap();
        buffer.write_all_with_threshold(b"world", 16).unwrap();
        assert!(matches!(buffer, SpooledBuffer::Mem(ref b) if b == b"helloworld"));
    }

    #[test]
    fn flips_to_file_when_threshold_exceeded() {
        let mut buffer = SpooledBuffer::default();
        buffer.write_all_with_threshold(b"0123456789", 16).unwrap();
        assert!(matches!(buffer, SpooledBuffer::Mem(_)));

        // Running total 10 + 10 = 20, past the 16-byte threshold.
        buffer.write_all_with_threshold(b"0123456789", 16).unwrap();
        assert!(matches!(buffer, SpooledBuffer::File(_)));
    }

    // Exercises the `<=` boundary exactly (landing on 16, not stepping past
    // it like `flips_to_file_when_threshold_exceeded` does).
    #[test]
    fn stays_in_memory_when_running_total_lands_exactly_on_threshold() {
        let mut buffer = SpooledBuffer::default();
        buffer.write_all_with_threshold(b"0123456789", 16).unwrap();
        assert!(matches!(buffer, SpooledBuffer::Mem(_)));

        // Running total is now exactly 16 — must NOT spill.
        buffer.write_all_with_threshold(b"012345", 16).unwrap();
        assert!(
            matches!(buffer, SpooledBuffer::Mem(ref b) if b == b"0123456789012345"),
            "landing exactly on the threshold must stay in memory"
        );
    }

    // Complement of the test above: one byte past exactly-the-threshold must
    // spill.
    #[test]
    fn spills_to_file_when_running_total_lands_one_byte_past_threshold() {
        let mut buffer = SpooledBuffer::default();
        buffer.write_all_with_threshold(b"0123456789", 16).unwrap();
        assert!(matches!(buffer, SpooledBuffer::Mem(_)));

        // Running total would be 17 — one byte past the 16-byte threshold.
        buffer.write_all_with_threshold(b"0123456", 16).unwrap();
        assert!(
            matches!(buffer, SpooledBuffer::File(_)),
            "one byte past the threshold must spill to disk"
        );
    }

    #[test]
    fn never_reverts_from_file_to_mem() {
        let mut buffer = SpooledBuffer::default();
        buffer.write_all_with_threshold(&[0u8; 20], 16).unwrap();
        assert!(matches!(buffer, SpooledBuffer::File(_)));

        // Small writes after spilling must still land in the file.
        buffer.write_all_with_threshold(b"x", 16).unwrap();
        assert!(matches!(buffer, SpooledBuffer::File(_)));
    }

    #[test]
    fn into_source_mem_arm_returns_bytes_with_no_temp_path() {
        let mut buffer = SpooledBuffer::default();
        buffer.write_all_with_threshold(b"in memory", 1024).unwrap();

        let (source, temp_path) = buffer.into_source();
        assert!(temp_path.is_none());
        match source {
            ByteSource::Bytes(b) => assert_eq!(b.as_ref(), b"in memory"),
            ByteSource::Path(_) => panic!("expected ByteSource::Bytes"),
        }
    }

    #[test]
    fn into_source_file_arm_returns_path_with_temp_path_guard() {
        let mut buffer = SpooledBuffer::default();
        buffer.write_all_with_threshold(&[7u8; 20], 16).unwrap();
        assert!(matches!(buffer, SpooledBuffer::File(_)));

        let (source, temp_path) = buffer.into_source();
        let temp_path = temp_path.expect("File arm must return a TempPath guard");
        match source {
            ByteSource::Path(path) => {
                assert_eq!(path, temp_path.to_path_buf());
                let mut contents = Vec::new();
                std::fs::File::open(&path)
                    .unwrap()
                    .read_to_end(&mut contents)
                    .unwrap();
                assert_eq!(contents, vec![7u8; 20]);
            }
            ByteSource::Bytes(_) => panic!("expected ByteSource::Path"),
        }
    }

    #[test]
    fn reassembled_bytes_are_byte_identical_across_many_small_chunks() {
        // Varying chunk sizes, crossing the (small, test-only) threshold.
        let chunks: Vec<Vec<u8>> = (0u8..50).map(|i| vec![i; (i as usize % 7) + 1]).collect();
        let expected: Vec<u8> = chunks.iter().flatten().copied().collect();

        let mut buffer = SpooledBuffer::default();
        for chunk in &chunks {
            buffer.write_all_with_threshold(chunk, 64).unwrap();
        }
        assert!(
            matches!(buffer, SpooledBuffer::File(_)),
            "expected the payload to have spilled to disk given the small threshold"
        );

        let (source, _temp_path) = buffer.into_source();
        let reassembled = source.into_bytes().unwrap();
        assert_eq!(reassembled, expected);
    }

    #[test]
    fn reassembled_bytes_are_byte_identical_when_kept_in_memory() {
        let chunks: Vec<Vec<u8>> = (0u8..10).map(|i| vec![i; 3]).collect();
        let expected: Vec<u8> = chunks.iter().flatten().copied().collect();

        let mut buffer = SpooledBuffer::default();
        for chunk in &chunks {
            buffer.write_all_with_threshold(chunk, 4096).unwrap();
        }
        assert!(matches!(buffer, SpooledBuffer::Mem(_)));

        let (source, _temp_path) = buffer.into_source();
        let reassembled = source.into_bytes().unwrap();
        assert_eq!(reassembled, expected);
    }
}
