use flate2::{Compression, GzBuilder, bufread::GzDecoder};
use snafu::{Location, ResultExt, Snafu};
use std::io::{Read, Write};

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

// Chunks decompression
pub fn decompress_data(input_data: Vec<u8>) -> Result<Vec<u8>, CompressionError> {
    let mut decoder = GzDecoder::new(input_data.as_slice());
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .context(DataReadingSnafu)?;
    Ok(decompressed_data)
}

#[derive(Snafu, Debug)]
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
