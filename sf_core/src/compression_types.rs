use snafu::{Location, Snafu};

// Tries to guess the compression type based on the last extension of the filename
// If that fails, it tries to guess based on the first four bytes of the file buffer content
// If both fail, it returns CompressionType::None
// Returns an error if the compression type is unsupported
pub fn try_guess_compression_type(
    filename: &str,
    file_buffer: &[u8],
) -> Result<SupportedCompressionType, CompressionTypeError> {
    let compression_type = try_guess_compression_type_from_filename(filename)?;

    if let Some(compression_type) = compression_type {
        return Ok(compression_type);
    }

    let compression_type = try_guess_compression_type_from_buffer(file_buffer)?;

    if let Some(compression_type) = compression_type {
        return Ok(compression_type);
    }

    Ok(SupportedCompressionType::None)
}

fn try_guess_compression_type_from_filename(
    filename: &str,
) -> Result<Option<SupportedCompressionType>, CompressionTypeError> {
    // Check if the filename has an extension
    match filename.rsplit('.').next() {
        Some(file_extension) => get_compression_type_from_extension(file_extension),
        None => Ok(None),
    }
}

fn try_guess_compression_type_from_buffer(
    file_buffer: &[u8],
) -> Result<Option<SupportedCompressionType>, CompressionTypeError> {
    // Read first 4 bytes of the file buffer
    let magic_bytes = file_buffer.get(0..4);
    match magic_bytes {
        Some(magic_bytes) => get_compression_type_from_first_four_bytes(magic_bytes),
        None => Ok(None),
    }
}

fn get_compression_type_from_extension(
    file_extension: &str,
) -> Result<Option<SupportedCompressionType>, CompressionTypeError> {
    // Check if we can find the compression type in the supported types
    let compression_type = SUPPORTED_COMPRESSION_TYPE_DATA
        .iter()
        .find(|data| data.extension == Some(file_extension))
        .map(|data| data.type_.clone());
    if let Some(compression_type) = compression_type {
        return Ok(Some(compression_type));
    }

    // If not, check if the compression type is unsupported
    let unsupported_type_name = UNSUPPORTED_COMPRESSION_TYPE_DATA
        .iter()
        .find(|data| data.extension == Some(file_extension))
        .map(|data| data.name);
    if let Some(unsupported_type_name) = unsupported_type_name {
        return UnsupportedCompressionTypeSnafu {
            type_name: unsupported_type_name.to_string(),
        }
        .fail();
    }

    // If we still don't find the compression type, return None
    Ok(None)
}

fn get_compression_type_from_first_four_bytes(
    bytes: &[u8],
) -> Result<Option<SupportedCompressionType>, CompressionTypeError> {
    // Check if we can find the compression type in the supported types
    // Check if magic bytes starts with the magic bytes of the supported types
    let compression_type = SUPPORTED_COMPRESSION_TYPE_DATA
        .iter()
        .find(
            |data| matches!(data.magic_bytes, Some(magic_bytes) if bytes.starts_with(magic_bytes)),
        )
        .map(|data| data.type_.clone());
    if let Some(compression_type) = compression_type {
        return Ok(Some(compression_type));
    }

    // If not, check if the compression type is unsupported
    let unsupported_type_name = UNSUPPORTED_COMPRESSION_TYPE_DATA
        .iter()
        .find(
            |data| matches!(data.magic_bytes, Some(magic_bytes) if bytes.starts_with(magic_bytes)),
        )
        .map(|data| data.name);
    if let Some(unsupported_type_name) = unsupported_type_name {
        return UnsupportedCompressionTypeSnafu {
            type_name: unsupported_type_name.to_string(),
        }
        .fail();
    }

    // If we still don't find the compression type, return None
    Ok(None)
}

pub const GZIP_NAME: &str = "GZIP";
pub const BZIP2_NAME: &str = "BZIP2";
pub const BROTLI_NAME: &str = "BROTLI";
pub const ZSTD_NAME: &str = "ZSTD";
pub const DEFLATE_NAME: &str = "DEFLATE";
pub const RAW_DEFLATE_NAME: &str = "RAW_DEFLATE";
pub const PARQUET_NAME: &str = "PARQUET";
pub const ORC_NAME: &str = "ORC";
pub const NONE_NAME: &str = "NONE";

#[derive(Debug, Clone, PartialEq)]
pub enum SupportedCompressionType {
    Gzip,
    Bzip2,
    Brotli,
    Zstd,
    Deflate,
    RawDeflate,
    Parquet,
    Orc,
    None,
}

impl SupportedCompressionType {
    pub fn get_name(&self) -> &'static str {
        match self {
            SupportedCompressionType::Gzip => GZIP_NAME,
            SupportedCompressionType::Bzip2 => BZIP2_NAME,
            SupportedCompressionType::Brotli => BROTLI_NAME,
            SupportedCompressionType::Zstd => ZSTD_NAME,
            SupportedCompressionType::Deflate => DEFLATE_NAME,
            SupportedCompressionType::RawDeflate => RAW_DEFLATE_NAME,
            SupportedCompressionType::Parquet => PARQUET_NAME,
            SupportedCompressionType::Orc => ORC_NAME,
            SupportedCompressionType::None => NONE_NAME,
        }
    }
}

const SUPPORTED_COMPRESSION_TYPE_DATA: [SupportedCompressionTypeData<'static>; 9] = [
    SupportedCompressionTypeData {
        _name: GZIP_NAME,
        extension: Some("gz"),
        magic_bytes: Some(&[0x1F, 0x8B]),
        type_: SupportedCompressionType::Gzip,
    },
    SupportedCompressionTypeData {
        _name: BZIP2_NAME,
        extension: Some("bz2"),
        magic_bytes: Some(&[0x42, 0x5A, 0x68]),
        type_: SupportedCompressionType::Bzip2,
    },
    SupportedCompressionTypeData {
        _name: BROTLI_NAME,
        extension: Some("br"),
        magic_bytes: Some(&[0x02, 0x21, 0x4C, 0x18]),
        type_: SupportedCompressionType::Brotli,
    },
    SupportedCompressionTypeData {
        _name: ZSTD_NAME,
        extension: Some("zst"),
        magic_bytes: Some(&[0x28, 0xB5, 0x2F, 0xFD]),
        type_: SupportedCompressionType::Zstd,
    },
    SupportedCompressionTypeData {
        _name: DEFLATE_NAME,
        extension: None,
        magic_bytes: Some(&[0x08, 0x00]),
        type_: SupportedCompressionType::Deflate,
    },
    SupportedCompressionTypeData {
        _name: RAW_DEFLATE_NAME,
        extension: None,
        magic_bytes: None,
        type_: SupportedCompressionType::RawDeflate,
    },
    SupportedCompressionTypeData {
        _name: PARQUET_NAME,
        extension: Some("parquet"),
        magic_bytes: Some(&[0x50, 0x41, 0x52, 0x31]),
        type_: SupportedCompressionType::Parquet,
    },
    SupportedCompressionTypeData {
        _name: ORC_NAME,
        extension: Some("orc"),
        magic_bytes: Some(&[0x4F, 0x52, 0x43]),
        type_: SupportedCompressionType::Orc,
    },
    SupportedCompressionTypeData {
        _name: NONE_NAME,
        extension: None,
        magic_bytes: None,
        type_: SupportedCompressionType::None,
    },
];

const UNSUPPORTED_COMPRESSION_TYPE_DATA: [UnsupportedCompressionTypeData<'static>; 5] = [
    UnsupportedCompressionTypeData {
        name: "LZ",
        extension: Some("lz"),
        magic_bytes: None,
    },
    UnsupportedCompressionTypeData {
        name: "LZMA",
        extension: Some("lzma"),
        magic_bytes: None,
    },
    UnsupportedCompressionTypeData {
        name: "LZO",
        extension: Some("lzo"),
        magic_bytes: None,
    },
    UnsupportedCompressionTypeData {
        name: "XZ",
        extension: Some("xz"),
        magic_bytes: None,
    },
    UnsupportedCompressionTypeData {
        name: "COMPRESS",
        extension: Some("Z"),
        magic_bytes: None,
    },
];

struct SupportedCompressionTypeData<'a> {
    _name: &'a str,
    extension: Option<&'a str>,
    magic_bytes: Option<&'a [u8]>,
    type_: SupportedCompressionType,
}

struct UnsupportedCompressionTypeData<'a> {
    name: &'a str,
    extension: Option<&'a str>,
    magic_bytes: Option<&'a [u8]>,
}

#[derive(Snafu, Debug)]
pub enum CompressionTypeError {
    #[snafu(display("Unsupported compression type: {type_name}"))]
    UnsupportedCompressionType {
        type_name: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unknown compression type: {type_name}"))]
    UnknownCompressionType {
        type_name: String,
        #[snafu(implicit)]
        location: Location,
    },
}
