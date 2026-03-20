//! DSN (Data Source Name) configuration loading.
//!
//! Reads DSN attributes from platform-specific ODBC configuration storage:
//! - Unix: odbc.ini files (via $ODBCINI, ~/.odbc.ini, /etc/odbc.ini)
//! - Windows: ODBC registry entries under HKCU/HKLM

use std::collections::HashMap;

/// Load DSN attributes for the named data source.
///
/// Returns `Some(attrs)` if the DSN is found, `None` if not found.
/// Attribute keys are returned in uppercase.
pub fn load_dsn_config(dsn_name: &str) -> Option<HashMap<String, String>> {
    tracing::debug!("dsn: loading config for {:?}", dsn_name);
    platform::load(dsn_name)
}

// ── Unix implementation ───────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
mod platform {
    use std::collections::HashMap;

    pub fn load(dsn_name: &str) -> Option<HashMap<String, String>> {
        for path in odbc_ini_paths() {
            tracing::debug!("dsn: searching {:?}", path);
            if let Some(attrs) = parse_ini_section(&path, dsn_name) {
                tracing::debug!(
                    "dsn: found {:?} in {:?} ({} keys)",
                    dsn_name,
                    path,
                    attrs.len()
                );
                return Some(attrs);
            }
        }
        tracing::debug!("dsn: {:?} not found", dsn_name);
        None
    }

    /// Ordered list of odbc.ini file paths to search.
    /// When $ODBCINI is set it is the only source (matches unixODBC behaviour).
    fn odbc_ini_paths() -> Vec<String> {
        if let Ok(p) = std::env::var("ODBCINI") {
            return vec![p];
        }
        let mut paths = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{home}/.odbc.ini"));
        }
        paths.push("/etc/odbc.ini".to_string());
        paths
    }

    /// Parse a single odbc.ini file and return the key/value pairs for
    /// `dsn_name` (case-insensitive section match).  Keys are uppercased.
    pub(super) fn parse_ini_section(path: &str, dsn_name: &str) -> Option<HashMap<String, String>> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                tracing::trace!("dsn: cannot read {:?}: {}", path, e);
                return None;
            }
        };

        let mut in_section = false;
        let mut attrs: HashMap<String, String> = HashMap::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip blank lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let section = &line[1..line.len() - 1];
                let entering = section.eq_ignore_ascii_case(dsn_name);
                // If we already collected data and are moving to a new section,
                // we're done.
                if !entering && !attrs.is_empty() {
                    break;
                }
                in_section = entering;
                continue;
            }

            if in_section && let Some(eq) = line.find('=') {
                let key = line[..eq].trim().to_uppercase();
                let val = line[eq + 1..].trim().to_string();
                attrs.insert(key, val);
            }
        }

        if attrs.is_empty() { None } else { Some(attrs) }
    }
}

// ── Windows implementation ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod platform {
    use std::collections::HashMap;

    pub fn load(dsn_name: &str) -> Option<HashMap<String, String>> {
        // DSN entries live at:
        //   HKCU\SOFTWARE\ODBC\ODBC.INI\<dsn>  (user DSN)
        //   HKLM\SOFTWARE\ODBC\ODBC.INI\<dsn>  (system DSN)
        //
        // TODO: implement Windows registry lookup
        tracing::warn!(
            "dsn: Windows registry DSN loading not yet implemented for {:?}",
            dsn_name
        );
        None
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(not(target_os = "windows"))]
mod tests {
    use super::platform::parse_ini_section;

    #[test]
    fn returns_none_for_missing_dsn() {
        let content = "[ODBC Data Sources]\n\n[AnotherDSN]\nSERVER=host\n";
        let path = write_tmp("dsn_test_missing", content);
        assert!(parse_ini_section(&path, "NotThere").is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn loads_basic_dsn() {
        let content = "[ODBC Data Sources]\nMyDSN=driver\n\n[MyDSN]\nSERVER=myhost\nUID=myuser\n";
        let path = write_tmp("dsn_test_basic", content);
        let attrs = parse_ini_section(&path, "MyDSN").expect("DSN should be found");
        assert_eq!(attrs.get("SERVER").map(String::as_str), Some("myhost"));
        assert_eq!(attrs.get("UID").map(String::as_str), Some("myuser"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn keys_are_uppercased() {
        let content = "[testdsn]\nserver=host\nuid=user\n";
        let path = write_tmp("dsn_test_upper", content);
        let attrs = parse_ini_section(&path, "testdsn").expect("DSN should be found");
        assert!(attrs.contains_key("SERVER"));
        assert!(attrs.contains_key("UID"));
        assert!(!attrs.contains_key("server"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn section_match_is_case_insensitive() {
        let content = "[MyDSN]\nSERVER=host\n";
        let path = write_tmp("dsn_test_case", content);
        assert!(parse_ini_section(&path, "mydsn").is_some());
        assert!(parse_ini_section(&path, "MYDSN").is_some());
        assert!(parse_ini_section(&path, "MyDsN").is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn stops_at_next_section() {
        let content = "[FirstDSN]\nSERVER=first\n\n[SecondDSN]\nSERVER=second\n";
        let path = write_tmp("dsn_test_stop", content);
        let a = parse_ini_section(&path, "FirstDSN").unwrap();
        assert_eq!(a.get("SERVER").map(String::as_str), Some("first"));
        assert_eq!(a.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ignores_blank_and_comment_lines() {
        let content = "[MyDSN]\n# comment\n; another comment\n\nSERVER=host\n";
        let path = write_tmp("dsn_test_comments", content);
        let attrs = parse_ini_section(&path, "MyDSN").unwrap();
        assert_eq!(attrs.get("SERVER").map(String::as_str), Some("host"));
        assert_eq!(attrs.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    /// Write `content` to a temp file and return its path.
    fn write_tmp(name: &str, content: &str) -> String {
        let path = format!("/tmp/{}_{}.ini", name, std::process::id());
        std::fs::write(&path, content).expect("write temp file");
        path
    }
}
