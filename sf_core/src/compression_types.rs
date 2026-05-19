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

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum CompressionTypeError {
    #[snafu(display("Unsupported compression type: {type_name}"))]
    UnsupportedCompressionType {
        type_name: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // Supported extensions: each maps to the matching `CompressionType`
    // variant via `try_guess_compression_type` (filename branch).
    #[test]
    fn extension_gz_maps_to_gzip() {
        let result = try_guess_compression_type("file.gz", b"").unwrap();
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn extension_bz2_maps_to_bzip2() {
        let result = try_guess_compression_type("file.bz2", b"").unwrap();
        assert_eq!(result, CompressionType::Bzip2);
    }

    #[test]
    fn extension_br_maps_to_brotli() {
        let result = try_guess_compression_type("file.br", b"").unwrap();
        assert_eq!(result, CompressionType::Brotli);
    }

    #[test]
    fn extension_zst_maps_to_zstd() {
        let result = try_guess_compression_type("file.zst", b"").unwrap();
        assert_eq!(result, CompressionType::Zstd);
    }

    #[test]
    fn extension_deflate_maps_to_deflate() {
        let result = try_guess_compression_type("file.deflate", b"").unwrap();
        assert_eq!(result, CompressionType::Deflate);
    }

    #[test]
    fn extension_raw_deflate_maps_to_raw_deflate() {
        let result = try_guess_compression_type("file.raw_deflate", b"").unwrap();
        assert_eq!(result, CompressionType::RawDeflate);
    }

    // Unsupported extensions: each errors with `UnsupportedCompressionType`
    // and the right `type_name`.
    fn assert_unsupported_with_name(filename: &str, expected_type_name: &str) {
        let result = try_guess_compression_type(filename, b"");
        match result {
            Err(CompressionTypeError::UnsupportedCompressionType { type_name, .. }) => {
                assert_eq!(
                    type_name, expected_type_name,
                    "Wrong type_name for {filename}",
                );
            }
            other => panic!("Expected UnsupportedCompressionType for {filename}, got: {other:?}",),
        }
    }

    #[test]
    fn extension_lz_errors_with_lz_type_name() {
        assert_unsupported_with_name("file.lz", "LZ");
    }

    #[test]
    fn extension_lzma_errors_with_lzma_type_name() {
        assert_unsupported_with_name("file.lzma", "LZMA");
    }

    #[test]
    fn extension_lzo_errors_with_lzo_type_name() {
        assert_unsupported_with_name("file.lzo", "LZO");
    }

    #[test]
    fn extension_xz_errors_with_xz_type_name() {
        assert_unsupported_with_name("file.xz", "XZ");
    }

    #[test]
    fn extension_capital_z_errors_with_compress_type_name() {
        assert_unsupported_with_name("file.Z", "COMPRESS");
    }

    #[test]
    fn extension_parquet_errors_with_parquet_type_name() {
        assert_unsupported_with_name("file.parquet", "PARQUET");
    }

    #[test]
    fn extension_orc_errors_with_orc_type_name() {
        assert_unsupported_with_name("file.orc", "ORC");
    }

    // Extension match is case-sensitive: `foo.GZ` does not match by
    // extension and must fall through to the magic-byte branch (here:
    // empty buffer => None).
    #[test]
    fn extension_match_is_case_sensitive() {
        let result = try_guess_compression_type("foo.GZ", b"").unwrap();
        assert_eq!(result, CompressionType::None);
    }

    #[test]
    fn unknown_extension_with_empty_buffer_returns_none() {
        let result = try_guess_compression_type("file.unknownext", b"").unwrap();
        assert_eq!(result, CompressionType::None);
    }

    // Magic-byte sniffing detects gzip, bzip2, and zstd when the filename
    // has no recognized extension.
    #[test]
    fn magic_bytes_detect_gzip() {
        let gzip_magic: &[u8] = &[0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let result = try_guess_compression_type("noext", gzip_magic).unwrap();
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn magic_bytes_detect_bzip2() {
        let bzip2_magic: &[u8] = &[0x42, 0x5A, 0x68, 0x39, 0x31, 0x41, 0x59, 0x26, 0x53, 0x59];
        let result = try_guess_compression_type("noext", bzip2_magic).unwrap();
        assert_eq!(result, CompressionType::Bzip2);
    }

    #[test]
    fn magic_bytes_detect_zstd() {
        let zstd_magic: &[u8] = &[0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x00, 0x00, 0x00];
        let result = try_guess_compression_type("noext", zstd_magic).unwrap();
        assert_eq!(result, CompressionType::Zstd);
    }

    // Multi-dot filenames: only the last segment matters.
    #[test]
    fn multi_dot_filename_uses_last_segment_for_gzip() {
        let result = try_guess_compression_type("foo.tar.gz", b"").unwrap();
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn multi_dot_filename_with_unknown_last_segment_falls_through() {
        // `foo.gz.bak` — last segment is `bak`, not `gz` — extension
        // branch returns `None`, magic-bytes branch returns `None` for
        // empty buffer, overall result is `CompressionType::None`.
        let result = try_guess_compression_type("foo.gz.bak", b"").unwrap();
        assert_eq!(result, CompressionType::None);
    }

    // `rsplit('.').next()` returns the whole string for filenames with no
    // `.` — that string ends up not matching any known extension, so the
    // result is `Ok(None)` from the filename branch and the magic-byte
    // branch then takes over (here: empty buffer => Ok(None) overall).
    #[test]
    fn filename_with_no_dot_falls_through_to_magic() {
        let result = try_guess_compression_type("plainfile", b"").unwrap();
        assert_eq!(result, CompressionType::None);
    }

    // Buffer-detection branch surfaces unsupported magic bytes (e.g. xz
    // / 7z) as `UnsupportedCompressionType` — locks current behavior so
    // PR2 has a baseline to change against.
    #[test]
    fn magic_bytes_detect_xz_as_unsupported() {
        let xz_magic: &[u8] = b"\xFD7zXZ\x00\x00\x01\x69\x22\xDE\x36";
        let result = try_guess_compression_type("noext", xz_magic);
        match result {
            Err(CompressionTypeError::UnsupportedCompressionType { type_name, .. }) => {
                assert_eq!(type_name, "XZ");
            }
            other => panic!("Expected UnsupportedCompressionType for xz magic, got: {other:?}"),
        }
    }
}
