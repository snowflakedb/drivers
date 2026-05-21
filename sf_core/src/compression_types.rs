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

/// Short-prefix magic-byte table mirroring libsnowflakeclient's
/// `FileCompressionType.cpp`. Each entry is matched with `starts_with` —
/// shorter than what the `infer` crate requires (e.g. infer's gzip matcher
/// needs the 3-byte `1F 8B 08`; ODBC accepts the 2-byte `1F 8B`). Used
/// only when `legacy_compression_autodetect_libsnowflakeclient_behavior`
/// is true; Python/JDBC do not consult this table.
///
/// `0x78 0x01 / 0x9C / 0xDA` are zlib stream headers. UD has no separate
/// `Zlib` variant, so they map to `Deflate` (matching how zlib-wrapped
/// streams are surfaced through the rest of the pipeline).
///
/// `0xCE 0xB2 0xCF 0x81` is the snowflake-specific brotli marker from
/// libsnowflakeclient's `FileCompressionType.cpp` brotli branch — brotli
/// has no IETF-defined magic, so the marker is a snowflake convention.
const SHORT_MAGIC_PREFIXES: &[(&[u8], CompressionType)] = &[
    (&[0xCE, 0xB2, 0xCF, 0x81], CompressionType::Brotli),
    (&[0x1F, 0x8B], CompressionType::Gzip),
    (&[0x78, 0x01], CompressionType::Deflate),
    (&[0x78, 0x9C], CompressionType::Deflate),
    (&[0x78, 0xDA], CompressionType::Deflate),
];

// Tries to guess the compression type based on the last extension of the filename
// If that fails, it tries to guess based on the file buffer content
// If both fail, it returns CompressionType::None
// Returns an error if the compression type is unsupported
//
// `legacy_compression_autodetect_libsnowflakeclient_behavior` enables a
// libsnowflakeclient-style short-prefix match (see `SHORT_MAGIC_PREFIXES`)
// ahead of the `infer` crate's full-buffer detection. ODBC sets this to
// true to keep parity with legacy behavior; Python / JDBC keep it false.
// (Error swallowing — the second half of the legacy behavior — is
// applied one layer up, in `auto_detect_source_compression`.)
pub fn try_guess_compression_type(
    filename: &str,
    file_buffer: &[u8],
    legacy_compression_autodetect_libsnowflakeclient_behavior: bool,
) -> Result<CompressionType, CompressionTypeError> {
    let compression_type = try_guess_compression_type_from_filename(filename)?;

    if let Some(compression_type) = compression_type {
        return Ok(compression_type);
    }

    if legacy_compression_autodetect_libsnowflakeclient_behavior
        && let Some(compression_type) = try_match_short_prefix(file_buffer)
    {
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

fn try_match_short_prefix(file_buffer: &[u8]) -> Option<CompressionType> {
    SHORT_MAGIC_PREFIXES
        .iter()
        .find_map(|(prefix, kind)| file_buffer.starts_with(prefix).then(|| kind.clone()))
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
        let result = try_guess_compression_type("file.gz", b"", false).unwrap();
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn extension_bz2_maps_to_bzip2() {
        let result = try_guess_compression_type("file.bz2", b"", false).unwrap();
        assert_eq!(result, CompressionType::Bzip2);
    }

    #[test]
    fn extension_br_maps_to_brotli() {
        let result = try_guess_compression_type("file.br", b"", false).unwrap();
        assert_eq!(result, CompressionType::Brotli);
    }

    #[test]
    fn extension_zst_maps_to_zstd() {
        let result = try_guess_compression_type("file.zst", b"", false).unwrap();
        assert_eq!(result, CompressionType::Zstd);
    }

    #[test]
    fn extension_deflate_maps_to_deflate() {
        let result = try_guess_compression_type("file.deflate", b"", false).unwrap();
        assert_eq!(result, CompressionType::Deflate);
    }

    #[test]
    fn extension_raw_deflate_maps_to_raw_deflate() {
        let result = try_guess_compression_type("file.raw_deflate", b"", false).unwrap();
        assert_eq!(result, CompressionType::RawDeflate);
    }

    // Unsupported extensions: each errors with `UnsupportedCompressionType`
    // and the right `type_name`.
    fn assert_unsupported_with_name(filename: &str, expected_type_name: &str) {
        let result = try_guess_compression_type(filename, b"", false);
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
        let result = try_guess_compression_type("foo.GZ", b"", false).unwrap();
        assert_eq!(result, CompressionType::None);
    }

    #[test]
    fn unknown_extension_with_empty_buffer_returns_none() {
        let result = try_guess_compression_type("file.unknownext", b"", false).unwrap();
        assert_eq!(result, CompressionType::None);
    }

    // Magic-byte sniffing detects gzip, bzip2, and zstd when the filename
    // has no recognized extension.
    #[test]
    fn magic_bytes_detect_gzip() {
        let gzip_magic: &[u8] = &[0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let result = try_guess_compression_type("noext", gzip_magic, false).unwrap();
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn magic_bytes_detect_bzip2() {
        let bzip2_magic: &[u8] = &[0x42, 0x5A, 0x68, 0x39, 0x31, 0x41, 0x59, 0x26, 0x53, 0x59];
        let result = try_guess_compression_type("noext", bzip2_magic, false).unwrap();
        assert_eq!(result, CompressionType::Bzip2);
    }

    #[test]
    fn magic_bytes_detect_zstd() {
        let zstd_magic: &[u8] = &[0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x00, 0x00, 0x00];
        let result = try_guess_compression_type("noext", zstd_magic, false).unwrap();
        assert_eq!(result, CompressionType::Zstd);
    }

    // Multi-dot filenames: only the last segment matters.
    #[test]
    fn multi_dot_filename_uses_last_segment_for_gzip() {
        let result = try_guess_compression_type("foo.tar.gz", b"", false).unwrap();
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn multi_dot_filename_with_unknown_last_segment_falls_through() {
        // `foo.gz.bak` — last segment is `bak`, not `gz` — extension
        // branch returns `None`, magic-bytes branch returns `None` for
        // empty buffer, overall result is `CompressionType::None`.
        let result = try_guess_compression_type("foo.gz.bak", b"", false).unwrap();
        assert_eq!(result, CompressionType::None);
    }

    // `rsplit('.').next()` returns the whole string for filenames with no
    // `.` — that string ends up not matching any known extension, so the
    // result is `Ok(None)` from the filename branch and the magic-byte
    // branch then takes over (here: empty buffer => Ok(None) overall).
    #[test]
    fn filename_with_no_dot_falls_through_to_magic() {
        let result = try_guess_compression_type("plainfile", b"", false).unwrap();
        assert_eq!(result, CompressionType::None);
    }

    // Buffer-detection branch surfaces unsupported magic bytes (e.g. xz
    // / 7z) as `UnsupportedCompressionType` — locks current behavior so
    // PR2 has a baseline to change against.
    #[test]
    fn magic_bytes_detect_xz_as_unsupported() {
        let xz_magic: &[u8] = b"\xFD7zXZ\x00\x00\x01\x69\x22\xDE\x36";
        let result = try_guess_compression_type("noext", xz_magic, false);
        match result {
            Err(CompressionTypeError::UnsupportedCompressionType { type_name, .. }) => {
                assert_eq!(type_name, "XZ");
            }
            other => panic!("Expected UnsupportedCompressionType for xz magic, got: {other:?}"),
        }
    }

    // Extension/magic conflict: filename says gzip, magic bytes say bzip2.
    // The filename branch runs first and short-circuits on the recognized
    // `.gz` extension, so the magic-byte buffer is never consulted —
    // result is `Gzip` regardless of the legacy flag. This matches Python
    // (`mimetypes.guess_type` first) and JDBC (`Files.probeContentType`
    // first); it diverges from libsnowflakeclient's ODBC, which iterates
    // magic bytes first and would return `Bzip2` here.
    #[test]
    fn extension_gz_with_bzip2_magic_resolves_via_extension_to_gzip_for_both_flag_values() {
        let bzip2_magic: &[u8] = &[0x42, 0x5A, 0x68, 0x39];
        for legacy in [false, true] {
            let result = try_guess_compression_type("file.gz", bzip2_magic, legacy).unwrap();
            assert_eq!(
                result,
                CompressionType::Gzip,
                "Extension must win over conflicting magic bytes (legacy={legacy})",
            );
        }
    }

    // 2-byte gzip prefix: shorter than `infer`'s 3-byte gzip matcher.
    // With the flag false (Python/JDBC default) `infer` returns `None` and
    // we fall through to `CompressionType::None`. With the flag true (ODBC
    // default), the short-prefix table matches first and returns `Gzip`,
    // mirroring `libsnowflakeclient`'s `m_magicBytes = 2` for gzip.
    #[test]
    fn magic_bytes_partial_gzip_2_bytes_undetected_when_legacy_flag_false() {
        let two_bytes: &[u8] = &[0x1F, 0x8B];
        let result = try_guess_compression_type("noext", two_bytes, false).unwrap();
        assert_eq!(result, CompressionType::None);
    }

    #[test]
    fn magic_bytes_partial_gzip_2_bytes_detected_when_legacy_flag_true() {
        let two_bytes: &[u8] = &[0x1F, 0x8B];
        let result = try_guess_compression_type("noext", two_bytes, true).unwrap();
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn magic_bytes_full_gzip_3_bytes_detected_for_both_flag_values() {
        let three_bytes: &[u8] = &[0x1F, 0x8B, 0x08];
        for legacy in [false, true] {
            let result = try_guess_compression_type("noext", three_bytes, legacy).unwrap();
            assert_eq!(
                result,
                CompressionType::Gzip,
                "Full gzip magic must always be detected (legacy={legacy})",
            );
        }
    }

    // zlib stream header (`78 9C` is the most common variant). `infer` has
    // no zlib matcher; the flag enables libsnowflakeclient's behavior of
    // surfacing zlib-wrapped streams as `Deflate` (UD has no separate
    // `Zlib` variant).
    #[test]
    fn magic_bytes_zlib_default_compressed_undetected_when_legacy_flag_false() {
        let zlib_magic: &[u8] = &[0x78, 0x9C, 0x00, 0x00];
        let result = try_guess_compression_type("noext", zlib_magic, false).unwrap();
        assert_eq!(result, CompressionType::None);
    }

    #[test]
    fn magic_bytes_zlib_default_compressed_detected_as_deflate_when_legacy_flag_true() {
        for header in [0x01u8, 0x9C, 0xDA] {
            let zlib_magic: &[u8] = &[0x78, header, 0x00, 0x00];
            let result = try_guess_compression_type("noext", zlib_magic, true).unwrap();
            assert_eq!(
                result,
                CompressionType::Deflate,
                "zlib header 0x78 0x{header:02X} must map to Deflate",
            );
        }
    }

    // Brotli has no IETF-defined magic; libsnowflakeclient defines a
    // 4-byte snowflake-specific marker (`CE B2 CF 81`). Flag-gated to
    // avoid breaking the Python/JDBC contract (they detect brotli purely
    // via `.br` extension, never magic).
    #[test]
    fn magic_bytes_brotli_marker_undetected_when_legacy_flag_false() {
        let brotli_magic: &[u8] = &[0xCE, 0xB2, 0xCF, 0x81, 0x00];
        let result = try_guess_compression_type("noext", brotli_magic, false).unwrap();
        assert_eq!(result, CompressionType::None);
    }

    #[test]
    fn magic_bytes_brotli_marker_detected_when_legacy_flag_true() {
        let brotli_magic: &[u8] = &[0xCE, 0xB2, 0xCF, 0x81, 0x00];
        let result = try_guess_compression_type("noext", brotli_magic, true).unwrap();
        assert_eq!(result, CompressionType::Brotli);
    }

    // Empty buffer: no extension matches, `infer` returns None, and the
    // short-prefix table never matches an empty buffer (`starts_with(&[])`
    // would be true but no entry has an empty prefix). Both flag values
    // return None — matches Python, JDBC, and ODBC.
    #[test]
    fn magic_bytes_empty_buffer_returns_none_for_both_flag_values() {
        for legacy in [false, true] {
            let result = try_guess_compression_type("noext", b"", legacy).unwrap();
            assert_eq!(
                result,
                CompressionType::None,
                "Empty buffer must report None (legacy={legacy})",
            );
        }
    }

    // Filename of only dots: `rsplit('.').next()` returns `""` — the
    // empty string doesn't match any known extension, so we fall through
    // to magic-byte detection. Locks the current behavior; reviewers
    // flagged this as untested.
    #[test]
    fn filename_only_dots_falls_through_to_magic() {
        let result = try_guess_compression_type("...", b"", false).unwrap();
        assert_eq!(result, CompressionType::None);
    }

    // `.gz` (leading-dot, no stem): `rsplit('.').next()` returns `"gz"`,
    // which matches the gzip extension branch — same outcome as `foo.gz`.
    // Matches Python's `mimetypes.guess_type(".gz")` and JDBC's
    // `endsWith(".gz")` fallback.
    #[test]
    fn filename_only_dot_extension_gz_treated_as_gzip() {
        let result = try_guess_compression_type(".gz", b"", false).unwrap();
        assert_eq!(result, CompressionType::Gzip);
    }

    // Very long filename ending in `.gz`: no length limit anywhere in the
    // detection path — this exercises the `rsplit('.').next()` allocation
    // pattern with 4096+ bytes and asserts it still returns the trailing
    // segment. None of the legacy connectors special-case length either.
    #[test]
    fn filename_4096_chars_with_gz_extension_detected_as_gzip() {
        let mut filename = "a".repeat(4096);
        filename.push_str(".gz");
        let result = try_guess_compression_type(&filename, b"", false).unwrap();
        assert_eq!(result, CompressionType::Gzip);
    }

    // Brotli / deflate / raw_deflate are never detected from magic bytes
    // when the flag is false: brotli has no IETF magic (Python/JDBC
    // detect it purely via extension), deflate / raw_deflate are
    // headerless. Locks this — `infer` has no matchers for these formats.
    #[test]
    fn magic_bytes_brotli_deflate_raw_deflate_never_detected_when_flag_false() {
        // No defined magic bytes for these formats — pass plausible
        // payload bytes that should remain unidentified.
        let arbitrary: &[u8] = b"plain brotli/deflate payload \x00\xFF";
        let result = try_guess_compression_type("noext", arbitrary, false).unwrap();
        assert_eq!(result, CompressionType::None);
    }
}
