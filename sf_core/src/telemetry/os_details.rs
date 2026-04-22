use std::collections::HashMap;
use std::path::Path;

use crate::fs_adapter::FsAdapter;

const OS_RELEASE_PATH: &str = "/etc/os-release";
const ALLOWED_KEYS: &[&str] = &[
    "NAME",
    "PRETTY_NAME",
    "ID",
    "BUILD_ID",
    "IMAGE_ID",
    "IMAGE_VERSION",
    "VERSION",
    "VERSION_ID",
];

/// Detect OS details for telemetry.
///
/// Returns `Some(map)` on Linux when `/etc/os-release` can be read and
/// contains at least one allow-listed key; otherwise returns `None`.
pub fn detect_os_details(fs: &dyn FsAdapter) -> Option<HashMap<String, String>> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    match fs.read_to_string(Path::new(OS_RELEASE_PATH)) {
        Ok(contents) => {
            let parsed = parse_os_release(&contents);
            if parsed.is_empty() {
                None
            } else {
                Some(parsed)
            }
        }
        Err(e) => {
            tracing::debug!(
                path = OS_RELEASE_PATH,
                error = %e,
                "Failed to read OS release file for telemetry"
            );
            None
        }
    }
}

/// Parse an `os-release(5)` file body and return the allow-listed keys.
///
/// Each line is expected to be `KEY=value`, `KEY="value"`, or `KEY='value'`.
/// Lines that do not match or whose key is not in [`ALLOWED_KEYS`] are
/// ignored. Inside double quotes the shell-style escapes `\$`, `\"`, `\\`,
/// and `` \` `` are honored (per `os-release(5)`); single-quoted values are
/// taken literally. Malformed lines (e.g. unterminated quotes) are skipped.
fn parse_os_release(contents: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        if !is_valid_key(key) || !ALLOWED_KEYS.contains(&key) {
            continue;
        }
        let Some(value) = unquote_value(raw_value) else {
            continue;
        };
        result.insert(key.to_string(), value);
    }
    result
}

fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Unquote an `os-release(5)` value.
///
/// Returns `None` when the value is malformed (e.g. an unterminated quote or
/// trailing garbage after a closing quote).
fn unquote_value(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    match bytes.first() {
        Some(&b'"') => unquote_double(&value[1..]),
        Some(&b'\'') => unquote_single(&value[1..]),
        _ => Some(value.to_string()),
    }
}

/// Parse the remainder of a double-quoted value (opening `"` already stripped).
///
/// Honors `\\`, `\"`, `\$`, and `` \` `` escapes. Any other backslash is kept
/// verbatim, matching the behavior of common `os-release` producers.
fn unquote_double(inner: &str) -> Option<String> {
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                return if chars.next().is_none() {
                    Some(out)
                } else {
                    None
                };
            }
            '\\' => match chars.next() {
                Some(next @ ('\\' | '"' | '$' | '`')) => out.push(next),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => return None,
            },
            other => out.push(other),
        }
    }
    None
}

/// Parse the remainder of a single-quoted value (opening `'` already stripped).
///
/// Single-quoted values are literal: no escape sequences are interpreted.
fn unquote_single(inner: &str) -> Option<String> {
    let (content, rest) = inner.split_once('\'')?;
    if rest.is_empty() {
        Some(content.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_adapter::mock::MockFs;

    const MOCK_OS_RELEASE: &str = r#"
NAME="Arch Linux"
PRETTY_NAME="Arch Linux"
ID=arch
BUILD_ID=rolling
VERSION_ID=20251019.0.436919
ANSI_COLOR="38;2;23;147;209"
HOME_URL="https://archlinux.org/"
DOCUMENTATION_URL="https://wiki.archlinux.org/"
SUPPORT_URL="https://bbs.archlinux.org/"
BUG_REPORT_URL="https://gitlab.archlinux.org/groups/archlinux/-/issues"
PRIVACY_POLICY_URL="https://terms.archlinux.org/docs/privacy-policy/"
LOGO=archlinux-logo
"#;

    #[test]
    fn parses_allowed_keys_from_os_release() {
        let parsed = parse_os_release(MOCK_OS_RELEASE);
        assert_eq!(parsed.get("NAME").map(String::as_str), Some("Arch Linux"));
        assert_eq!(
            parsed.get("PRETTY_NAME").map(String::as_str),
            Some("Arch Linux")
        );
        assert_eq!(parsed.get("ID").map(String::as_str), Some("arch"));
        assert_eq!(parsed.get("BUILD_ID").map(String::as_str), Some("rolling"));
        assert_eq!(
            parsed.get("VERSION_ID").map(String::as_str),
            Some("20251019.0.436919")
        );
        assert_eq!(parsed.len(), 5, "unexpected extra keys: {parsed:?}");
    }

    #[test]
    fn ignores_disallowed_keys_and_unquotes() {
        let parsed = parse_os_release(MOCK_OS_RELEASE);
        assert!(!parsed.contains_key("ANSI_COLOR"));
        assert!(!parsed.contains_key("HOME_URL"));
        assert!(!parsed.contains_key("LOGO"));
    }

    #[test]
    fn unquoted_values_are_preserved() {
        let contents = "ID=arch\nBUILD_ID=rolling\n";
        let parsed = parse_os_release(contents);
        assert_eq!(parsed.get("ID").map(String::as_str), Some("arch"));
        assert_eq!(parsed.get("BUILD_ID").map(String::as_str), Some("rolling"));
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let contents = "NOT_A_LINE_WITHOUT_EQUALS\n#NAME=comment\n=no-key\nNAME=\"Fine\"\n";
        let parsed = parse_os_release(contents);
        assert_eq!(parsed.get("NAME").map(String::as_str), Some("Fine"));
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn lowercase_keys_are_rejected() {
        let contents = "name=\"lowercase\"\nNAME=\"Upper\"\n";
        let parsed = parse_os_release(contents);
        assert_eq!(parsed.get("NAME").map(String::as_str), Some("Upper"));
        assert!(!parsed.contains_key("name"));
    }

    #[test]
    fn single_quoted_values_are_literal() {
        let contents = "NAME='Arch Linux'\nID='a\\$b'\n";
        let parsed = parse_os_release(contents);
        assert_eq!(parsed.get("NAME").map(String::as_str), Some("Arch Linux"));
        assert_eq!(parsed.get("ID").map(String::as_str), Some("a\\$b"));
    }

    #[test]
    fn double_quoted_escapes_and_malformed_quotes() {
        let contents = concat!(
            "NAME=\"a\\\"b\\\\c\\$d\\`e\"\n",
            "PRETTY_NAME=\"unterminated\n",
            "VERSION=\"ok\"junk\n",
            "ID=arch\n",
        );
        let parsed = parse_os_release(contents);
        assert_eq!(parsed.get("NAME").map(String::as_str), Some("a\"b\\c$d`e"));
        assert!(!parsed.contains_key("PRETTY_NAME"));
        assert!(!parsed.contains_key("VERSION"));
        assert_eq!(parsed.get("ID").map(String::as_str), Some("arch"));
    }

    #[test]
    fn detect_returns_none_on_non_linux() {
        if cfg!(target_os = "linux") {
            return;
        }
        let fs = MockFs::new().with_file(OS_RELEASE_PATH, MOCK_OS_RELEASE);
        assert!(detect_os_details(&fs).is_none());
    }

    #[test]
    fn detect_parses_allowed_keys_on_linux() {
        if !cfg!(target_os = "linux") {
            return;
        }
        let fs = MockFs::new().with_file(OS_RELEASE_PATH, MOCK_OS_RELEASE);
        let details = detect_os_details(&fs).expect("expected Some on Linux with valid file");
        assert_eq!(details.get("ID").map(String::as_str), Some("arch"));
        assert_eq!(details.get("NAME").map(String::as_str), Some("Arch Linux"));
        assert_eq!(details.len(), 5);
    }

    #[test]
    fn detect_returns_none_when_file_missing_on_linux() {
        if !cfg!(target_os = "linux") {
            return;
        }
        let fs = MockFs::new();
        assert!(detect_os_details(&fs).is_none());
    }
}
