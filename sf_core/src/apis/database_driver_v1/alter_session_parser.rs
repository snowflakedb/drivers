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

/// Parse all ALTER SESSION SET statements from a multistatement query.
///
/// This function searches the entire SQL string for ALTER SESSION SET statements,
/// similar to the Python driver's regex `finditer()` behavior. It handles queries like:
/// - "ALTER SESSION SET QUERY_TAG = 'test'; ALTER SESSION SET TIMEZONE = 'UTC'"
/// - "SELECT 1; ALTER SESSION SET AUTOCOMMIT = false; SELECT 'a'"
///
/// Returns a vector of all ALTER SESSION parameters found in the query, in order.
pub fn parse_all_alter_sessions(sql: &str) -> Vec<AlterSessionParameter> {
    let mut results = Vec::new();
    let mut remaining = sql;

    while !remaining.is_empty() {
        // Try to find the next ALTER SESSION statement
        let upper = remaining.to_uppercase();
        if let Some(alter_pos) = upper.find("ALTER") {
            // Check if this is actually an ALTER SESSION SET statement
            let candidate = &remaining[alter_pos..];
            if let Some(param) = parse_alter_session(candidate) {
                results.push(param);

                // Move past this ALTER SESSION statement to find the next one
                // Find the end of this statement (semicolon or end of string)
                let after_alter = &candidate[5..]; // Skip "ALTER"
                if let Some(semi_pos) = after_alter.find(';') {
                    remaining = &after_alter[semi_pos + 1..];
                } else {
                    // No semicolon found, but we might have more statements
                    // Try to find the next ALTER keyword
                    if let Some(next_alter) = after_alter.to_uppercase().find("ALTER") {
                        remaining = &after_alter[next_alter..];
                    } else {
                        break;
                    }
                }
            } else {
                // Not an ALTER SESSION SET, skip past this ALTER
                remaining = &remaining[alter_pos + 5..];
            }
        } else {
            // No more ALTER keywords found
            break;
        }
    }

    results
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
    let mut chars = sql.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            // Semicolon always terminates the value
            ';' => break,
            // '--' starts a line comment, but a lone '-' is part of the value
            '-' if chars.clone().nth(1) == Some('-') => break,
            // '/*' starts a block comment, but a lone '/' is part of the value
            '/' if chars.clone().nth(1) == Some('*') => break,
            _ => {
                result.push(c);
                chars.next();
            }
        }
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

    #[test]
    fn test_multistatement_alter_session_simple() {
        // Test two ALTER SESSION statements separated by semicolon
        let sql = "ALTER SESSION SET QUERY_TAG = 'test'; ALTER SESSION SET TIMEZONE = 'UTC'";

        // parse_alter_session should only parse the first statement
        let result = parse_alter_session(sql);
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "test".to_string(),
            })
        );
    }

    #[test]
    fn test_multistatement_with_mixed_queries() {
        // Test ALTER SESSION mixed with other query types
        let sql = "SELECT 1; ALTER SESSION SET AUTOCOMMIT = false; SELECT 'a'; ALTER SESSION SET JSON_INDENT = 4";

        // parse_alter_session should return None since first statement is not ALTER SESSION
        let result = parse_alter_session(sql);
        assert_eq!(result, None);
    }

    #[test]
    fn test_multistatement_three_alters() {
        // Test three ALTER SESSION statements
        let sql = "ALTER SESSION SET QUERY_TAG = 'tag1'; ALTER SESSION SET TIMEZONE = 'America/Los_Angeles'; ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'YYYY-MM-DD'";

        // Should parse first statement
        let result = parse_alter_session(sql);
        assert_eq!(
            result,
            Some(AlterSessionParameter {
                name: "QUERY_TAG".to_string(),
                value: "tag1".to_string(),
            })
        );
    }

    // Tests for parse_all_alter_sessions

    #[test]
    fn test_parse_all_single_alter() {
        let sql = "ALTER SESSION SET QUERY_TAG = 'test'";
        let results = parse_all_alter_sessions(sql);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "QUERY_TAG");
        assert_eq!(results[0].value, "test");
    }

    #[test]
    fn test_parse_all_two_alters_with_semicolon() {
        let sql = "ALTER SESSION SET QUERY_TAG = 'test'; ALTER SESSION SET TIMEZONE = 'UTC'";
        let results = parse_all_alter_sessions(sql);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "QUERY_TAG");
        assert_eq!(results[0].value, "test");
        assert_eq!(results[1].name, "TIMEZONE");
        assert_eq!(results[1].value, "UTC");
    }

    #[test]
    fn test_parse_all_three_alters() {
        let sql = "ALTER SESSION SET QUERY_TAG = 'tag1'; ALTER SESSION SET TIMEZONE = 'America/Los_Angeles'; ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'YYYY-MM-DD'";
        let results = parse_all_alter_sessions(sql);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "QUERY_TAG");
        assert_eq!(results[0].value, "tag1");
        assert_eq!(results[1].name, "TIMEZONE");
        assert_eq!(results[1].value, "America/Los_Angeles");
        assert_eq!(results[2].name, "TIMESTAMP_OUTPUT_FORMAT");
        assert_eq!(results[2].value, "YYYY-MM-DD");
    }

    #[test]
    fn test_parse_all_mixed_with_select() {
        // Match Python driver test case
        let sql = "SELECT 1; ALTER SESSION SET AUTOCOMMIT = false; SELECT 'a'; ALTER SESSION SET JSON_INDENT = 4; ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_TZ'";
        let results = parse_all_alter_sessions(sql);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "AUTOCOMMIT");
        assert_eq!(results[0].value, "false");
        assert_eq!(results[1].name, "JSON_INDENT");
        assert_eq!(results[1].value, "4");
        assert_eq!(results[2].name, "CLIENT_TIMESTAMP_TYPE_MAPPING");
        assert_eq!(results[2].value, "TIMESTAMP_TZ");
    }

    #[test]
    fn test_parse_all_no_alters() {
        let sql = "SELECT * FROM table; INSERT INTO table VALUES (1)";
        let results = parse_all_alter_sessions(sql);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_all_empty_string() {
        let sql = "";
        let results = parse_all_alter_sessions(sql);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_all_alter_table_not_session() {
        let sql = "ALTER TABLE t ADD COLUMN c INT; ALTER SESSION SET QUERY_TAG = 'test'";
        let results = parse_all_alter_sessions(sql);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "QUERY_TAG");
        assert_eq!(results[0].value, "test");
    }

    #[test]
    fn test_parse_all_with_comments() {
        let sql = "-- First statement\nALTER SESSION SET QUERY_TAG = 'test';\n/* Block comment */\nALTER SESSION SET TIMEZONE = 'UTC'";
        let results = parse_all_alter_sessions(sql);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "QUERY_TAG");
        assert_eq!(results[0].value, "test");
        assert_eq!(results[1].name, "TIMEZONE");
        assert_eq!(results[1].value, "UTC");
    }

    #[test]
    fn test_parse_all_same_parameter_multiple_times() {
        // Test that the last value wins
        let sql = "ALTER SESSION SET QUERY_TAG = 'first'; ALTER SESSION SET QUERY_TAG = 'second'; ALTER SESSION SET QUERY_TAG = 'third'";
        let results = parse_all_alter_sessions(sql);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].value, "first");
        assert_eq!(results[1].value, "second");
        assert_eq!(results[2].value, "third");
    }
}
