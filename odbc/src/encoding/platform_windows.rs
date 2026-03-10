use encoding_rs::Encoding;

pub fn detect_locale_encoding() -> &'static Encoding {
    let code_page = unsafe { windows_sys::Win32::Globalization::GetACP() };
    codepage::to_encoding(code_page as u16).unwrap_or(encoding_rs::UTF_8)
}
