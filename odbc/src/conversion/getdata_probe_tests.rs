//! Regression tests for SNOW-3758323: a zero-length `SQLGetData` probe
//! (`BufferLength == 0`) must report the value's length without consuming the
//! column, so a subsequent `SQLGetData` returns the data.
//!
//! The statement layer (`api/data.rs`) maps the post-conversion
//! `get_data_offset` back to per-column state: `Some(n)` → `Partial` (more to
//! read), `None` → `Completed` (next call returns `SQL_NO_DATA`). A fresh
//! column starts at `None`, so a probe that leaves the offset untouched would
//! be recorded as `Completed` and drop the value on the next read. These tests
//! pin that the probe leaves the cursor at `Some(start)` instead.
#[cfg(test)]
mod tests {
    use crate::api::CDataType;
    use crate::api::encoding::{WideChar, wchar_byte_size};
    use crate::conversion::test_utils::helpers::{
        binding_for_char_buffer, binding_for_wchar_buffer,
    };
    use crate::conversion::warning::Warning;
    use odbc_sys as sql;

    fn is_truncated(warnings: &[Warning]) -> bool {
        warnings
            .iter()
            .any(|w| matches!(w, Warning::StringDataTruncated))
    }

    #[test]
    fn char_string_zero_length_probe_preserves_cursor() {
        let mut str_len: sql::Len = 0;
        // Empty slice => buffer_length 0 => length-only probe.
        let mut empty: [u8; 0] = [];
        let probe = binding_for_char_buffer(CDataType::Char, &mut empty, &mut str_len);

        let mut offset: Option<usize> = None;
        let warnings = probe.write_char_string("hello", &mut offset);

        assert_eq!(str_len, 5, "probe must report the full byte length");
        assert!(is_truncated(&warnings));
        assert_eq!(
            offset,
            Some(0),
            "probe must leave the read cursor at 0, not None (None => Completed => SQL_NO_DATA)"
        );

        // The subsequent real read returns the whole value and completes.
        let mut buf = vec![0u8; 16];
        let read = binding_for_char_buffer(CDataType::Char, &mut buf, &mut str_len);
        let warnings = read.write_char_string("hello", &mut offset);

        assert!(warnings.is_empty());
        assert_eq!(&buf[..5], b"hello");
        assert_eq!(buf[5], 0);
        assert_eq!(str_len, 5);
        assert_eq!(offset, None, "value fully read => column complete");
    }

    #[test]
    fn char_string_zero_length_probe_on_empty_value_completes() {
        let mut str_len: sql::Len = 0;
        let mut empty: [u8; 0] = [];
        let probe = binding_for_char_buffer(CDataType::Char, &mut empty, &mut str_len);

        let mut offset: Option<usize> = None;
        let warnings = probe.write_char_string("", &mut offset);

        assert_eq!(str_len, 0);
        assert!(warnings.is_empty(), "empty value is not truncated");
        assert_eq!(
            offset, None,
            "nothing to return => column complete on the probe"
        );
    }

    #[test]
    fn char_from_fn_zero_length_probe_preserves_cursor() {
        let src = b"12345";
        let mut str_len: sql::Len = 0;
        let mut empty: [u8; 0] = [];
        let probe = binding_for_char_buffer(CDataType::Char, &mut empty, &mut str_len);

        let mut offset: Option<usize> = None;
        let warnings =
            probe.write_char_from_fn(|i| src.get(i).copied(), src.len() as sql::Len, &mut offset);

        assert_eq!(str_len, 5);
        assert!(is_truncated(&warnings));
        assert_eq!(offset, Some(0));

        let mut buf = vec![0u8; 16];
        let read = binding_for_char_buffer(CDataType::Char, &mut buf, &mut str_len);
        let warnings =
            read.write_char_from_fn(|i| src.get(i).copied(), src.len() as sql::Len, &mut offset);

        assert!(warnings.is_empty());
        assert_eq!(&buf[..5], b"12345");
        assert_eq!(offset, None);
    }

    #[test]
    fn wchar_string_zero_length_probe_preserves_cursor() {
        let mut str_len: sql::Len = 0;
        // Empty wide buffer => buffer_length 0 < unit_size => length-only probe.
        let mut empty: [WideChar; 0] = [];
        let probe = binding_for_wchar_buffer(&mut empty, &mut str_len);

        let mut offset: Option<usize> = None;
        let warnings = probe.write_wchar_string("hi", &mut offset);

        // Indicator is in bytes: 2 code units * unit width.
        assert_eq!(str_len, 2 * wchar_byte_size() as sql::Len);
        assert!(is_truncated(&warnings));
        assert_eq!(offset, Some(0));

        let mut buf = vec![0 as WideChar; 8];
        let read = binding_for_wchar_buffer(&mut buf, &mut str_len);
        let warnings = read.write_wchar_string("hi", &mut offset);

        assert!(warnings.is_empty());
        assert_eq!(buf[0], b'h' as WideChar);
        assert_eq!(buf[1], b'i' as WideChar);
        assert_eq!(offset, None);
    }

    #[test]
    fn wchar_from_fn_zero_length_probe_preserves_cursor() {
        let units: [WideChar; 2] = [b'h' as WideChar, b'i' as WideChar];
        let mut str_len: sql::Len = 0;
        let mut empty: [WideChar; 0] = [];
        let probe = binding_for_wchar_buffer(&mut empty, &mut str_len);

        let mut offset: Option<usize> = None;
        let warnings = probe.write_wchar_from_fn(
            |i| units.get(i).copied(),
            units.len() as sql::Len,
            &mut offset,
        );

        assert_eq!(str_len, 2 * wchar_byte_size() as sql::Len);
        assert!(is_truncated(&warnings));
        assert_eq!(offset, Some(0));

        let mut buf = vec![0 as WideChar; 8];
        let read = binding_for_wchar_buffer(&mut buf, &mut str_len);
        let warnings = read.write_wchar_from_fn(
            |i| units.get(i).copied(),
            units.len() as sql::Len,
            &mut offset,
        );

        assert!(warnings.is_empty());
        assert_eq!(buf[0], b'h' as WideChar);
        assert_eq!(buf[1], b'i' as WideChar);
        assert_eq!(offset, None);
    }
}
