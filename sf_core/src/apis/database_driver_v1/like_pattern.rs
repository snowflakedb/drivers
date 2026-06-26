//! SQL-LIKE pattern matcher with `\` escape extension.
//!
//! Semantics:
//! - `%` = zero or more characters
//! - `_` = exactly one character
//! - `\%`, `\_`, `\\` = literal `%`, `_`, `\`
//! - Matching is case-insensitive (both sides uppercased).
//! - An empty pattern `""` never matches anything (no Snowflake identifier is named "").
//!
//! This is a superset of ADBC GetObjects filter semantics (which defines no escape).
//!
//! The escape character `\` is the canonical constant shared across all drivers and
//! advertised via `SQLGetInfo(SQL_SEARCH_PATTERN_ESCAPE)`.

/// The escape character for LIKE patterns in catalog functions.
/// Matches `SQLGetInfo(SQL_SEARCH_PATTERN_ESCAPE)` in the ODBC wrapper.
pub const ESCAPE_CHAR: char = '\\';

/// Returns `Some(literal)` when `pattern` is an exact-value pattern (no unescaped `%`/`_`
/// remain after un-escaping). The returned string is the un-escaped literal, suitable for
/// pushing into `IN DATABASE "…"` / `IN SCHEMA "…"."…"`.
///
/// Returns `None` when `pattern` contains unescaped wildcards.
pub fn is_exact(pattern: &str) -> Option<String> {
    if pattern.is_empty() {
        // Empty string is an exact value (it matches only ""), but since no Snowflake
        // identifier is "", callers should treat this as "no match". Still return Some
        // so callers that want to push exact values can detect it.
        return Some(String::new());
    }
    let mut result = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ESCAPE_CHAR {
            match chars.next() {
                Some('%') => result.push('%'),
                Some('_') => result.push('_'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    // Unrecognised escape sequence — treat literally
                    result.push(ESCAPE_CHAR);
                    result.push(other);
                }
                None => result.push(ESCAPE_CHAR),
            }
        } else if c == '%' || c == '_' {
            return None; // wildcard found
        } else {
            result.push(c);
        }
    }
    Some(result)
}

/// Strips `\` escape sequences for coarse Snowflake `SHOW … LIKE` pushdown.
///
/// Snowflake's LIKE does not honor ODBC escape sequences. The backend receives
/// the stripped pattern (e.g. `MY\_TABLE` → `MY_TABLE`) for coarse narrowing;
/// client-side [`matches`] applies the original pattern for correctness.
pub fn strip_escapes_for_show_like(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ESCAPE_CHAR {
            match chars.peek().copied() {
                Some(escaped @ ('%' | '_' | '\\' | '"')) => {
                    chars.next(); // consume the peeked escape target
                    result.push(escaped);
                }
                _ => result.push(ESCAPE_CHAR),
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Returns `true` when `value` matches `pattern` (case-insensitive).
///
/// An empty `pattern` never matches (returns `false`).
pub fn matches(pattern: &str, value: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    let pat: Vec<char> = pattern.chars().collect();
    let val_upper: Vec<char> = value.to_uppercase().chars().collect();
    matches_inner(&pat, &val_upper)
}

/// Two-pointer linear LIKE matcher (no backtracking via recursion blowup).
fn matches_inner(pat: &[char], val: &[char]) -> bool {
    let mut pi = 0usize;
    let mut vi = 0usize;
    let mut star_pi: Option<usize> = None;
    let mut star_vi = 0usize;

    while vi < val.len() {
        if pi < pat.len() && pat[pi] == ESCAPE_CHAR {
            // Escaped character: consume escape + next literal
            let escaped = if pi + 1 < pat.len() {
                let e = pat[pi + 1];
                pi += 2;
                e
            } else {
                pi += 1;
                ESCAPE_CHAR
            };
            let vc = val[vi].to_uppercase().next().unwrap_or(val[vi]);
            let pc = escaped.to_uppercase().next().unwrap_or(escaped);
            if vc == pc {
                vi += 1;
            } else if let Some(sp) = star_pi {
                pi = sp;
                star_vi += 1;
                vi = star_vi;
            } else {
                return false;
            }
        } else if pi < pat.len() && pat[pi] == '%' {
            // Record position after %
            star_pi = Some(pi + 1);
            star_vi = vi;
            pi += 1;
        } else if pi < pat.len()
            && (pat[pi] == '_' || {
                let vc = val[vi].to_uppercase().next().unwrap_or(val[vi]);
                let pc = pat[pi].to_uppercase().next().unwrap_or(pat[pi]);
                vc == pc
            })
        {
            pi += 1;
            vi += 1;
        } else if let Some(sp) = star_pi {
            pi = sp;
            star_vi += 1;
            vi = star_vi;
        } else {
            return false;
        }
    }
    // Consume trailing % in pattern
    while pi < pat.len() && pat[pi] == '%' {
        pi += 1;
    }
    pi == pat.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_matches_any_sequence() {
        assert!(matches("%", "hello"));
        assert!(matches("%", ""));
        assert!(matches("h%", "hello"));
        assert!(matches("%o", "hello"));
        assert!(matches("h%o", "hello"));
        assert!(matches("h%o", "helloo"));
    }

    #[test]
    fn underscore_matches_single_char() {
        assert!(matches("h_llo", "hello"));
        assert!(!matches("h_llo", "hllo"));
        assert!(!matches("h_llo", "heello"));
    }

    #[test]
    fn escaped_percent_is_literal() {
        assert!(matches("100\\%", "100%"));
        assert!(!matches("100\\%", "100X"));
    }

    #[test]
    fn escaped_underscore_is_literal() {
        assert!(matches("MY\\_TABLE", "MY_TABLE"));
        assert!(!matches("MY\\_TABLE", "MY1TABLE"));
    }

    #[test]
    fn escaped_backslash_is_literal() {
        assert!(matches("a\\\\b", "a\\b"));
    }

    #[test]
    fn case_insensitive() {
        assert!(matches("HELLO", "hello"));
        assert!(matches("hello", "HELLO"));
        assert!(matches("%world%", "Hello World"));
    }

    #[test]
    fn empty_pattern_never_matches() {
        assert!(!matches("", "anything"));
        assert!(!matches("", ""));
    }

    #[test]
    fn is_exact_no_wildcards() {
        assert_eq!(is_exact("MYTABLE"), Some("MYTABLE".to_string()));
        assert_eq!(is_exact(""), Some("".to_string()));
    }

    #[test]
    fn is_exact_with_wildcards_returns_none() {
        assert_eq!(is_exact("MY%"), None);
        assert_eq!(is_exact("MY_"), None);
    }

    #[test]
    fn is_exact_escaped_wildcards_are_literal() {
        assert_eq!(is_exact("100\\%"), Some("100%".to_string()));
        assert_eq!(is_exact("MY\\_TABLE"), Some("MY_TABLE".to_string()));
        assert_eq!(
            is_exact("SNOWFLAKE\\_SAMPLE\\_DATA"),
            Some("SNOWFLAKE_SAMPLE_DATA".to_string())
        );
        assert_eq!(is_exact("A\\\\B"), Some("A\\B".to_string()));
    }

    #[test]
    fn strip_escapes_for_show_like_removes_escape_before_wildcards() {
        assert_eq!(strip_escapes_for_show_like("MY\\_TABLE"), "MY_TABLE");
        assert_eq!(strip_escapes_for_show_like("100\\%"), "100%");
    }

    #[test]
    fn strip_escapes_for_show_like_removes_escape_before_backslash_and_quote() {
        assert_eq!(strip_escapes_for_show_like("A\\\\B"), "A\\B");
        assert_eq!(strip_escapes_for_show_like("a\\\"b"), "a\"b");
    }

    #[test]
    fn strip_escapes_for_show_like_is_identity_without_escapes() {
        assert_eq!(strip_escapes_for_show_like("MY%TABLE"), "MY%TABLE");
        assert_eq!(strip_escapes_for_show_like("plain"), "plain");
    }

    #[test]
    fn strip_escapes_for_show_like_keeps_lone_backslash() {
        assert_eq!(strip_escapes_for_show_like("foo\\"), "foo\\");
        assert_eq!(strip_escapes_for_show_like("foo\\x"), "foo\\x");
    }
}
