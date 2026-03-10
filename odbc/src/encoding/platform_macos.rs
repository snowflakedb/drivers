use encoding_rs::Encoding;

pub fn detect_locale_encoding() -> &'static Encoding {
    encoding_rs::UTF_8
}
