extern crate infer;
use snafu::{Location, Snafu};

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

impl CompressionType {
    pub fn get_snowflake_representation(&self) -> &str {
        match self {
            CompressionType::Gzip => "GZIP",
            CompressionType::Bzip2 => "BZIP2",
            CompressionType::Brotli => "BROTLI",
            CompressionType::Zstd => "ZSTD",
            CompressionType::Deflate => "DEFLATE",
            CompressionType::RawDeflate => "RAW_DEFLATE",
            CompressionType::None => "NONE",
        }
    }
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
        "lz" => UnsupportedCompressionTypeSnafu {
            type_name: "LZ".to_string(),
        }
        .fail(),
        "lzma" => UnsupportedCompressionTypeSnafu {
            type_name: "LZMA".to_string(),
        }
        .fail(),
        "lzo" => UnsupportedCompressionTypeSnafu {
            type_name: "LZO".to_string(),
        }
        .fail(),
        "xz" => UnsupportedCompressionTypeSnafu {
            type_name: "XZ".to_string(),
        }
        .fail(),
        "Z" => UnsupportedCompressionTypeSnafu {
            type_name: "COMPRESS".to_string(),
        }
        .fail(),
        "parquet" => UnsupportedCompressionTypeSnafu {
            type_name: "PARQUET".to_string(),
        }
        .fail(),
        "orc" => UnsupportedCompressionTypeSnafu {
            type_name: "ORC".to_string(),
        }
        .fail(),
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

// TODO: DEFLATE cannot be detected by the infer crate - we might need a custom implementation for that
fn try_guess_compression_type_from_buffer(
    file_buffer: &[u8],
) -> Result<Option<CompressionType>, CompressionTypeError> {
    // Use the infer crate to guess the file type based on content
    match infer::get(file_buffer) {
        Some(kind) => get_compression_type_from_extension(kind.extension()),
        None => Ok(None),
    }
}

#[derive(Snafu, Debug)]
pub enum CompressionTypeError {
    #[snafu(display("Unsupported compression type: {type_name}"))]
    UnsupportedCompressionType {
        type_name: String,
        #[snafu(implicit)]
        location: Location,
    },
}
