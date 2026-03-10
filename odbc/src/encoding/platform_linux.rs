use encoding_rs::Encoding;

pub fn detect_locale_encoding() -> &'static Encoding {
    let locale = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_CTYPE"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();

    encoding_from_locale_string(&locale).unwrap_or(encoding_rs::UTF_8)
}

fn encoding_from_locale_string(locale: &str) -> Option<&'static Encoding> {
    let lower = locale.to_ascii_lowercase();
    if lower.is_empty() || lower == "c" || lower == "posix" {
        return Some(encoding_rs::UTF_8);
    }

    // Extract charset portion after the dot (e.g. "en_US.UTF-8" -> "UTF-8")
    let charset = lower.split('.').nth(1).unwrap_or(&lower);
    let charset = charset.split('@').next().unwrap_or(charset);

    Encoding::for_label(charset.as_bytes())
}
