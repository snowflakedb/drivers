extern crate infer;

use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum CompressionType {
    Gzip,
    Bzip2,
    Brotli,
    Zstd,
    Deflate,
    RawDeflate,
    None,
}

impl fmt::Display for CompressionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressionType::Gzip => write!(f, "GZIP"),
            CompressionType::Bzip2 => write!(f, "BZ2"),
            CompressionType::Brotli => write!(f, "BROTLI"),
            CompressionType::Zstd => write!(f, "ZSTD"),
            CompressionType::Deflate => write!(f, "DEFLATE"),
            CompressionType::RawDeflate => write!(f, "RAW_DEFLATE"),
            CompressionType::None => write!(f, "NONE"),
        }
    }
}

#[derive(Error, Debug)]
pub enum CompressionTypeError {
    #[error("Unsupported compression type: {0}")]
    UnsupportedCompressionType(String),
}

fn get_compression_type_from_extension(
    file_extension: &str,
) -> Result<Option<CompressionType>, CompressionTypeError> {
    match file_extension {
        "gz" => Ok(Some(CompressionType::Gzip)),
        "bz2" => Ok(Some(CompressionType::Bzip2)),
        "br" => Ok(Some(CompressionType::Brotli)),
        "zst" => Ok(Some(CompressionType::Zstd)),
        "deflate" => Ok(Some(CompressionType::Deflate)),
        "raw_deflate" => Ok(Some(CompressionType::RawDeflate)),
        "lz" => Err(CompressionTypeError::UnsupportedCompressionType(
            "LZIP".to_string(),
        )),
        "lzma" => Err(CompressionTypeError::UnsupportedCompressionType(
            "LZMA".to_string(),
        )),
        "lzo" => Err(CompressionTypeError::UnsupportedCompressionType(
            "LZO".to_string(),
        )),
        "xz" => Err(CompressionTypeError::UnsupportedCompressionType(
            "XZ".to_string(),
        )),
        "Z" => Err(CompressionTypeError::UnsupportedCompressionType(
            "COMPRESS".to_string(),
        )),
        "parquet" => Err(CompressionTypeError::UnsupportedCompressionType(
            "PARQUET".to_string(),
        )),
        "orc" => Err(CompressionTypeError::UnsupportedCompressionType(
            "ORC".to_string(),
        )),
        _ => Ok(None),
    }
}

// Tries to guess the compression type based on the last extension of the filename
// If that fails, it tries to guess based on the file buffer content
// If both fail, it returns CompressionType::None
// Returns an error if the compression type is unsupported
pub fn try_guess_compression_type(
    filename: &str,
    file_buffer: &[u8],
) -> Result<CompressionType, CompressionTypeError> {
    let compression_type = try_guess_compression_type_from_filename(filename)?;

    if let Some(compression_type) = compression_type {
        return Ok(compression_type);
    }

    let compression_type = try_guess_compression_type_from_buffer(file_buffer)?;

    if let Some(compression_type) = compression_type {
        return Ok(compression_type);
    }

    Ok(CompressionType::None)
}

fn try_guess_compression_type_from_filename(
    filename: &str,
) -> Result<Option<CompressionType>, CompressionTypeError> {
    // Check if the filename has an extension
    match filename.rsplit('.').next() {
        Some(file_extension) => get_compression_type_from_extension(file_extension),
        None => Ok(None),
    }
}

fn try_guess_compression_type_from_buffer(
    file_buffer: &[u8],
) -> Result<Option<CompressionType>, CompressionTypeError> {
    // Use the infer crate to guess the file type based on content
    match infer::get(file_buffer) {
        Some(kind) => get_compression_type_from_extension(kind.extension()),
        None => Ok(None),
    }
}
