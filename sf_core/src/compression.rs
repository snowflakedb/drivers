use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use flate2::{Compression, GzBuilder, bufread::GzDecoder};
use snafu::{Location, ResultExt, Snafu};
use std::io::{Read, Write};

// Mirror libsnowflakeclient/deps/zlib-1.3.1/zutil.h::OS_CODE.
//
// Legacy `compressWithGzip` writes whatever its libz build sets `OS_CODE` to,
// baked in at compile time. We replicate that compile-time selection so that
// UD-ODBC byte-matches legacy ODBC on every supported platform.
//
// zlib's full ladder names many more architectures (Amiga, VMS, OS/2, BeOS,
// OS/400, RISCOS, Atari, ...). We only branch on the targets UD ships on; if
// UD ever extends to one of those platforms, add a matching `#[cfg(target_os
// = ...)]` arm rather than relying on the Unix default.
#[cfg(target_os = "macos")]
const ZLIB_OS_CODE: u8 = 19; // zutil.h: __APPLE__
#[cfg(target_os = "windows")]
const ZLIB_OS_CODE: u8 = 10; // zutil.h: WIN32 && !__CYGWIN__
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const ZLIB_OS_CODE: u8 = 3; // zutil.h: default "assume Unix" (Linux, *BSD, AIX, ...)

/// Compress `input_data` into a gzip stream whose header bytes faithfully
/// reproduce the legacy driver's wire shape for the requested `flavor`.
///
/// | Flavor   | FLG  | FNAME                     | MTIME | XFL | OS              |
/// |----------|------|---------------------------|-------|-----|-----------------|
/// | `Python` | 0x08 | `len(basename)+2` spaces  | 0     | 2   | 255             |
/// | `Odbc`   | 0x00 | (no field)                | 0     | 0   | `ZLIB_OS_CODE`  |
///
/// XFL is derived from the deflate level by `flate2::GzBuilder` at
/// header-emit time (`>= best` → 2, `<= fast` → 4, else 0). It cannot be set
/// directly — XFL=2 requires `Compression::best()` (level 9) and XFL=0
/// requires `Compression::default()` (level 6).
///
/// The Python branch matches the legacy file-PUT shape produced by
/// `compress_file_with_gzip()` followed by `normalize_gzip_header()`: the
/// helper first writes `"<basename>_c\0"` into FNAME via
/// `gzip.GzipFile("<basename>_c.gz", "wb")` (Python's gzip strips the
/// trailing `.gz` before writing FNAME), then `normalize_gzip_header`
/// overwrites every byte of the FNAME slot with `0x20` spaces while
/// preserving the NUL terminator. We collapse the two-step legacy sequence
/// into a single `.filename(spaces)` call. CPython hardcodes OS=255
/// (`b'\xff'`) in `gzip.py` regardless of host, so we set 255 unconditionally
/// for Python.
///
/// The ODBC branch matches `libsnowflakeclient`'s `compressWithGzip`, which
/// never calls `deflateSetHeader` and so emits the minimum 10-byte header
/// (FLG=0x00, no FNAME). zlib's MTIME default when `deflateSetHeader` is not
/// called is 0, XFL is 0 because `compressWithGzip` passes
/// `Z_DEFAULT_COMPRESSION` (level 6), and OS is whatever the build's libz
/// sets `OS_CODE` to — mirrored here via `ZLIB_OS_CODE`.
///
/// NOTE for future stream-PUT implementers: when the source is an in-memory
/// stream rather than a file on disk, the legacy Python connector calls
/// `gzip.compress(data)`, which clears the FNAME bit (FLG bit 3 = 0) and
/// emits no FNAME field, but otherwise keeps the same XFL=2 / OS=255
/// metadata. UD does not yet support stream PUT; when added, the `Python`
/// branch must split into file-PUT (this implementation) and stream-PUT
/// (omit `.filename(...)`) rather than reusing this code path with a
/// synthetic basename.
pub fn compress_data(
    input_data: Vec<u8>,
    flavor: &PutGetResultsetFlavor,
    source_basename: &str,
) -> Result<Vec<u8>, CompressionError> {
    let mut encoder = match flavor {
        PutGetResultsetFlavor::Python => {
            // FNAME is `len(basename) + 2` 0x20 spaces, NUL-terminated by
            // GzBuilder. Matches `normalize_gzip_header`'s post-processing
            // of `compress_file_with_gzip`'s `<basename>_c\0` write.
            let blanked = vec![b' '; source_basename.len() + 2];
            GzBuilder::new()
                .mtime(0)
                .operating_system(255)
                .filename(blanked)
                .write(Vec::new(), Compression::best())
        }
        PutGetResultsetFlavor::Odbc => GzBuilder::new()
            .mtime(0)
            .operating_system(ZLIB_OS_CODE)
            .write(Vec::new(), Compression::default()),
    };
    encoder.write_all(&input_data).context(DataWritingSnafu)?;
    encoder.finish().context(DataWritingSnafu)
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

    /// Bit 3 of the FLG byte. When set, the gzip header carries a
    /// NUL-terminated original filename right after the fixed 10-byte
    /// preamble.
    const GZIP_FLG_FNAME: u8 = 0x08;
    /// Offsets into the fixed 10-byte gzip header preamble
    /// (`ID1`, `ID2`, `CM`, `FLG`, `MTIME[4]`, `XFL`, `OS`).
    const GZIP_FLG_OFFSET: usize = 3;
    const GZIP_MTIME_RANGE: std::ops::Range<usize> = 4..8;
    const GZIP_XFL_OFFSET: usize = 8;
    const GZIP_OS_OFFSET: usize = 9;
    /// Single fixture filename reused across the compression tests.
    const TEST_FILENAME: &str = "data.csv";

    #[test]
    fn compress_decompress_roundtrip_python() {
        let payload = b"hello world, this is a test payload".to_vec();
        let compressed = compress_data(
            payload.clone(),
            &PutGetResultsetFlavor::Python,
            TEST_FILENAME,
        )
        .expect("compression succeeds");
        let decompressed = decompress_data(&compressed).expect("decompression succeeds");
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn compress_decompress_roundtrip_odbc() {
        let payload = b"hello world, this is a test payload".to_vec();
        let compressed =
            compress_data(payload.clone(), &PutGetResultsetFlavor::Odbc, TEST_FILENAME)
                .expect("compression succeeds");
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
    fn compress_empty_data_python() {
        let empty = Vec::new();
        let compressed =
            compress_data(empty.clone(), &PutGetResultsetFlavor::Python, TEST_FILENAME)
                .expect("compression succeeds");
        let decompressed = decompress_data(&compressed).expect("decompression succeeds");
        assert_eq!(decompressed, empty);
    }

    #[test]
    fn compress_empty_data_odbc() {
        let empty = Vec::new();
        let compressed = compress_data(empty.clone(), &PutGetResultsetFlavor::Odbc, TEST_FILENAME)
            .expect("compression succeeds");
        let decompressed = decompress_data(&compressed).expect("decompression succeeds");
        assert_eq!(decompressed, empty);
    }

    #[test]
    fn compress_large_payload_python() {
        let payload: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        let compressed = compress_data(
            payload.clone(),
            &PutGetResultsetFlavor::Python,
            TEST_FILENAME,
        )
        .expect("compression succeeds");
        assert!(compressed.len() < payload.len());
        let decompressed = decompress_data(&compressed).expect("decompression succeeds");
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn compress_large_payload_odbc() {
        let payload: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        let compressed =
            compress_data(payload.clone(), &PutGetResultsetFlavor::Odbc, TEST_FILENAME)
                .expect("compression succeeds");
        assert!(compressed.len() < payload.len());
        let decompressed = decompress_data(&compressed).expect("decompression succeeds");
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn compress_python_emits_legacy_file_put_header() {
        let compressed = compress_data(
            b"abc".to_vec(),
            &PutGetResultsetFlavor::Python,
            TEST_FILENAME,
        )
        .expect("compression succeeds");

        let flg = compressed[GZIP_FLG_OFFSET];
        assert_ne!(
            flg & GZIP_FLG_FNAME,
            0,
            "FLG byte 0x{flg:02x} should have FNAME (0x08) bit set",
        );
        assert_eq!(
            &compressed[GZIP_MTIME_RANGE],
            &[0, 0, 0, 0],
            "MTIME should be zeroed (matches normalize_gzip_header)",
        );
        assert_eq!(
            compressed[GZIP_XFL_OFFSET], 2,
            "XFL should be 2 (derived from Compression::best(), level 9)",
        );
        assert_eq!(
            compressed[GZIP_OS_OFFSET], 255,
            "OS should be 255 (CPython gzip.py hardcodes b'\\xff')",
        );

        let decoder = GzDecoder::new(compressed.as_slice());
        let header_filename = decoder
            .header()
            .and_then(|h| h.filename())
            .expect("gzip header should carry a FNAME field");
        let expected_blanked = vec![b' '; TEST_FILENAME.len() + 2];
        assert_eq!(
            header_filename,
            expected_blanked.as_slice(),
            "FNAME should be `len(basename) + 2` spaces (matches normalize_gzip_header)",
        );
    }

    #[test]
    fn compress_odbc_emits_legacy_compresswithgzip_header() {
        let compressed =
            compress_data(b"abc".to_vec(), &PutGetResultsetFlavor::Odbc, TEST_FILENAME)
                .expect("compression succeeds");

        assert_eq!(
            compressed[GZIP_FLG_OFFSET], 0x00,
            "FLG byte should be 0 (compressWithGzip never calls deflateSetHeader)",
        );
        assert_eq!(
            &compressed[GZIP_MTIME_RANGE],
            &[0, 0, 0, 0],
            "MTIME should be zeroed (zlib default when deflateSetHeader is not called)",
        );
        assert_eq!(
            compressed[GZIP_XFL_OFFSET], 0,
            "XFL should be 0 (derived from Compression::default(), level 6)",
        );
        assert_eq!(
            compressed[GZIP_OS_OFFSET], ZLIB_OS_CODE,
            "OS byte should match the build target's zlib OS_CODE",
        );

        let decoder = GzDecoder::new(compressed.as_slice());
        assert!(
            decoder.header().and_then(|h| h.filename()).is_none(),
            "gzip header should not carry a FNAME field for ODBC flavor",
        );
    }
}
