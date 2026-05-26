use flate2::{Compression, GzBuilder, bufread::GzDecoder};
use snafu::{Location, ResultExt, Snafu};
use std::io::{BufReader, Read, Write};

// PUT/GET compression
pub fn compress_data(input_data: Vec<u8>) -> Result<Vec<u8>, CompressionError> {
    // Use GzBuilder to create gzip with a zeroed timestamp for consistent normalization
    let mut encoder = GzBuilder::new()
        .mtime(0) // Set timestamp to 0 for consistent normalization
        .write(Vec::new(), Compression::best());

    encoder.write_all(&input_data).context(DataWritingSnafu)?;
    let compressed_data = encoder.finish().context(DataWritingSnafu)?;

    Ok(compressed_data)
}

/// Returns a streaming gzip encoder wrapping `reader`.
///
/// The gzip header is produced with `mtime = 0` (no timestamp), identical to
/// `compress_data`. For a given input the byte output is therefore bit-for-bit
/// the same whether you use `compress_data` or drain this reader — the
/// underlying `GzBuilder` is the same either way.
///
/// This function is the foundation for the wrapper-facing streaming API that
/// lands in PR-3/PR-4. There are no call sites in this crate yet.
#[allow(dead_code)]
pub fn compress_reader<R: Read>(reader: R) -> impl Read {
    GzBuilder::new()
        .mtime(0) // zeroed timestamp, matches compress_data
        .read(BufReader::new(reader), Compression::best())
}

#[allow(unused)]
pub fn decompress_data(input_data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut decoder = GzDecoder::new(input_data);
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .context(DataReadingSnafu)?;
    Ok(decompressed_data)
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum CompressionError {
    #[snafu(display("Failed to write data during compression"))]
    DataWriting {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read data during decompression"))]
    DataReading {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_decompress_roundtrip() {
        let payload = b"hello world, this is a test payload".to_vec();
        let compressed = compress_data(payload.clone()).expect("compression succeeds");
        let decompressed = decompress_data(&compressed).expect("decompression succeeds");
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn decompress_invalid_data_fails() {
        let garbage = b"not valid gzip data".to_vec();
        let result = decompress_data(&garbage);
        assert!(result.is_err());
    }

    #[test]
    fn compress_empty_data() {
        let empty = Vec::new();
        let compressed = compress_data(empty.clone()).expect("compression succeeds");
        let decompressed = decompress_data(&compressed).expect("decompression succeeds");
        assert_eq!(decompressed, empty);
    }

    #[test]
    fn compress_large_payload() {
        let payload: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        let compressed = compress_data(payload.clone()).expect("compression succeeds");
        assert!(compressed.len() < payload.len());
        let decompressed = decompress_data(&compressed).expect("decompression succeeds");
        assert_eq!(decompressed, payload);
    }

    /// `compress_reader` must produce byte-identical output to `compress_data`
    /// for the same input — both use `GzBuilder::mtime(0)`.
    #[test]
    fn compress_reader_output_matches_compress_data() {
        let payload = b"hello, streaming gzip with mtime=0".to_vec();

        let expected = compress_data(payload.clone()).expect("compress_data succeeds");

        let mut reader_output = Vec::new();
        compress_reader(payload.as_slice())
            .read_to_end(&mut reader_output)
            .expect("compress_reader succeeds");

        assert_eq!(
            reader_output, expected,
            "compress_reader output must be bit-identical to compress_data"
        );
    }
}
