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
/// Each line is expected to be `KEY=value` or `KEY="value"`. Lines that do
/// not match or whose key is not in [`ALLOWED_KEYS`] are ignored.
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
        let value = strip_surrounding_double_quotes(raw_value);
        result.insert(key.to_string(), value.to_string());
    }
    result
}

fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn strip_surrounding_double_quotes(value: &str) -> &str {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        &value[1..value.len() - 1]
    } else {
        value
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
