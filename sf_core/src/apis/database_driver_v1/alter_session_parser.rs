/// Parser for ALTER SESSION SET statements to extract parameter changes.
///
/// This module provides functionality to parse ALTER SESSION SET SQL statements
/// and extract the parameter name and value. This allows for optimistic cache
/// updates before the query response is received, matching the behavior of
/// existing Python and other drivers.
/// Represents a parsed ALTER SESSION SET statement
#[derive(Debug, Clone, PartialEq)]
pub struct AlterSessionParameter {
    pub name: String,
    pub value: String,
}

/// Parse an ALTER SESSION SET statement and extract the parameter name and value.
///
/// Supports various SQL formats:
/// - ALTER SESSION SET QUERY_TAG = 'value'
/// - ALTER SESSION SET QUERY_TAG='value'
/// - alter session set query_tag = 'value'
/// - ALTER SESSION SET TIMEZONE = America/Los_Angeles
/// - ALTER SESSION SET PARAM = "value with spaces"
///
/// Returns None if:
/// - Not an ALTER SESSION SET statement
/// - Cannot parse parameter name or value
/// - Statement is malformed
pub fn parse_alter_session(sql: &str) -> Option<AlterSessionParameter> {
    let sql = skip_leading_whitespace_and_comments(sql);

    // Check if this is an ALTER SESSION statement
    if !sql.to_uppercase().starts_with("ALTER") {
        return None;
    }

    // Skip "ALTER"
    let sql = skip_token_and_whitespace(&sql[5..]);

    // Check for "SESSION"
    if !sql.to_uppercase().starts_with("SESSION") {
        return None;
    }

    // Skip "SESSION"
    let sql = skip_token_and_whitespace(&sql[7..]);

    // Check for "SET"
    if !sql.to_uppercase().starts_with("SET") {
        return None;
    }

    // Skip "SET"
    let sql = skip_token_and_whitespace(&sql[3..]);

    // Extract parameter name (everything until '=')
    let eq_pos = sql.find('=')?;
    let param_name = sql[..eq_pos].trim().to_uppercase();

    if param_name.is_empty() {
        return None;
    }

    // Skip '=' and whitespace
    let sql = sql[eq_pos + 1..].trim_start();

    // Extract value (handle quoted and unquoted values)
    let value = extract_value(sql)?;

    Some(AlterSessionParameter {
        name: param_name,
        value,
    })
}

/// Extract the value from the SQL, handling quoted and unquoted values
fn extract_value(sql: &str) -> Option<String> {
    if sql.is_empty() {
        return None;
    }

    let first_char = sql.chars().next()?;

    match first_char {
        '\'' => extract_single_quoted_value(sql),
        '"' => extract_double_quoted_value(sql),
        _ => extract_unquoted_value(sql),
    }
}

/// Extract a single-quoted value, handling escaped quotes
fn extract_single_quoted_value(sql: &str) -> Option<String> {
    if !sql.starts_with('\'') {
        return None;
    }

    let mut result = String::new();
    let mut chars = sql[1..].chars();
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped {
            result.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '\'' {
            // Check for doubled single quote (SQL escape)
            if chars.as_str().starts_with('\'') {
                chars.next(); // Skip the second quote
                result.push('\'');
            } else {
                // End of string
                return Some(result);
            }
        } else {
            result.push(c);
        }
    }

    // Unterminated string - return what we have
    Some(result)
}

/// Extract a double-quoted value, handling escaped quotes
fn extract_double_quoted_value(sql: &str) -> Option<String> {
    if !sql.starts_with('"') {
        return None;
    }

    let mut result = String::new();
    let mut chars = sql[1..].chars();
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped {
            result.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            // Check for doubled double quote (SQL escape)
            if chars.as_str().starts_with('"') {
                chars.next(); // Skip the second quote
                result.push('"');
            } else {
                // End of string
                return Some(result);
            }
        } else {
            result.push(c);
        }
    }

    // Unterminated string - return what we have
    Some(result)
}

/// Extract an unquoted value (everything until end of statement or semicolon/comment)
fn extract_unquoted_value(sql: &str) -> Option<String> {
    let mut result = String::new();

    for c in sql.chars() {
        if c == ';' || c == '-' || c == '/' {
            // End of value (semicolon or start of comment)
            break;
        }
        result.push(c);
    }

    let result = result.trim().to_string();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Skip leading whitespace and comments
fn skip_leading_whitespace_and_comments(s: &str) -> &str {
    let mut s = s;
    loop {
        s = s.trim_start();

        // Skip line comments: -- ... \n
        if s.starts_with("--") {
            match s.find('\n') {
                Some(pos) => s = &s[pos + 1..],
                None => return "", // Comment extends to end
            }
            continue;
        }

        // Skip block comments: /* ... */
        if s.starts_with("/*") {
            match s.find("*/") {
                Some(pos) => s = &s[pos + 2..],
                None => return "", // Unterminated comment
            }
            continue;
        }

        break;
    }
    s
}

/// Skip a token and following whitespace/comments
fn skip_token_and_whitespace(s: &str) -> &str {
    skip_leading_whitespace_and_comments(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_alter_session() {
        let result = parse_alter_session("ALTER SESSION SET QUERY_TAG = 'test_value'");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test_value".to_string(),
            })
        );
    }

    #[test]
    fn test_alter_session_no_spaces() {
        let result = parse_alter_session("ALTER SESSION SET QUERY_TAG='test_value'");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test_value".to_string(),
            })
        );
    }

    #[test]
    fn test_alter_session_lowercase() {
        let result = parse_alter_session("alter session set query_tag = 'test_value'");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test_value".to_string(),
            })
        );
    }

    #[test]
    fn test_alter_session_double_quotes() {
        let result = parse_alter_session("ALTER SESSION SET QUERY_TAG = \"test_value\"");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test_value".to_string(),
            })
        );
    }

    #[test]
    fn test_alter_session_unquoted() {
        let result = parse_alter_session("ALTER SESSION SET TIMEZONE = America/Los_Angeles");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "TIMEZONE".to_string(),
                value: "America/Los_Angeles".to_string(),
            })
        );
    }

    #[test]
    fn test_alter_session_with_semicolon() {
        let result = parse_alter_session("ALTER SESSION SET QUERY_TAG = 'test_value';");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test_value".to_string(),
            })
        );
    }

    #[test]
    fn test_alter_session_with_spaces_in_value() {
        let result = parse_alter_session("ALTER SESSION SET QUERY_TAG = 'test with spaces'");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test with spaces".to_string(),
            })
        );
    }

    #[test]
    fn test_alter_session_with_escaped_quotes() {
        let result = parse_alter_session("ALTER SESSION SET QUERY_TAG = 'test''s value'");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test's value".to_string(),
            })
        );
    }

    #[test]
    fn test_alter_session_with_leading_comments() {
        let result = parse_alter_session("-- comment\nALTER SESSION SET QUERY_TAG = 'test_value'");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test_value".to_string(),
            })
        );
    }

    #[test]
    fn test_alter_session_with_block_comment() {
        let result =
            parse_alter_session("/* comment */ ALTER SESSION SET QUERY_TAG = 'test_value'");
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test_value".to_string(),
            })
        );
    }

    #[test]
    fn test_not_alter_session() {
        assert_eq!(parse_alter_session("SELECT * FROM table"), None);
        assert_eq!(parse_alter_session("INSERT INTO table VALUES (1)"), None);
        assert_eq!(parse_alter_session("UPDATE table SET col = 1"), None);
    }

    #[test]
    fn test_alter_but_not_session() {
        assert_eq!(parse_alter_session("ALTER TABLE t ADD COLUMN c INT"), None);
    }

    #[test]
    fn test_alter_session_without_set() {
        assert_eq!(parse_alter_session("ALTER SESSION UNSET QUERY_TAG"), None);
    }

    #[test]
    fn test_malformed_alter_session() {
        assert_eq!(parse_alter_session("ALTER SESSION SET"), None);
        assert_eq!(parse_alter_session("ALTER SESSION SET ="), None);
        assert_eq!(parse_alter_session("ALTER SESSION SET PARAM"), None);
    }
}
