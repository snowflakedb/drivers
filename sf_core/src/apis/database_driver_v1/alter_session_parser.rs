/// Parser for ALTER SESSION SET statements to extract parameter changes.
///
/// This module provides functionality to parse ALTER SESSION SET SQL statements
/// and extract the parameter name and value. This allows for optimistic cache
/// updates before the query response is received, matching the behavior of the
/// legacy Python connector. Only wrappers that set
/// `WrapperPresets::optimistic_alter_session_param_cache` update their cache
/// this way.
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
    parse_next_alter_session(sql).map(|(param, _)| param)
}

fn parse_next_alter_session(sql: &str) -> Option<(AlterSessionParameter, &str)> {
    let sql = skip_leading_whitespace_and_comments(sql);
    let sql = skip_keyword(sql, "ALTER")?;
    let sql = skip_keyword(sql, "SESSION")?;
    let sql = skip_keyword(sql, "SET")?;

    let eq_pos = sql.find('=')?;
    let param_name = sql[..eq_pos].trim().to_uppercase();

    if param_name.is_empty() {
        return None;
    }

    let sql = sql[eq_pos + 1..].trim_start();
    let (value, rest) = extract_value(sql)?;

    Some((
        AlterSessionParameter {
            name: param_name,
            value,
        },
        rest,
    ))
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
        let upper = remaining.to_ascii_uppercase();
        if let Some(alter_pos) = upper.find("ALTER") {
            let candidate = &remaining[alter_pos..];
            if let Some((param, rest)) = parse_next_alter_session(candidate) {
                results.push(param);
                remaining = rest;
            } else {
                remaining = &remaining[alter_pos + 5..];
            }
        } else {
            break;
        }
    }

    results
}

fn extract_value(sql: &str) -> Option<(String, &str)> {
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

fn extract_single_quoted_value(sql: &str) -> Option<(String, &str)> {
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
            if chars.as_str().starts_with('\'') {
                chars.next();
                result.push('\'');
            } else {
                return Some((result, chars.as_str()));
            }
        } else {
            result.push(c);
        }
    }

    Some((result, ""))
}

fn extract_double_quoted_value(sql: &str) -> Option<(String, &str)> {
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
            if chars.as_str().starts_with('"') {
                chars.next();
                result.push('"');
            } else {
                return Some((result, chars.as_str()));
            }
        } else {
            result.push(c);
        }
    }

    Some((result, ""))
}

fn extract_unquoted_value(sql: &str) -> Option<(String, &str)> {
    let mut end = 0;
    let mut chars = sql.char_indices().peekable();

    while let Some(&(i, c)) = chars.peek() {
        match c {
            ';' | '\n' | '\r' => break,
            '-' if chars.clone().nth(1).map(|(_, ch)| ch) == Some('-') => break,
            '/' if chars.clone().nth(1).map(|(_, ch)| ch) == Some('*') => break,
            _ => {
                end = i + c.len_utf8();
                chars.next();
            }
        }
    }

    let result = sql[..end].trim().to_string();
    if result.is_empty() {
        None
    } else {
        Some((result, &sql[end..]))
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

fn skip_keyword<'a>(sql: &'a str, keyword: &str) -> Option<&'a str> {
    if !sql.to_ascii_uppercase().starts_with(keyword) {
        return None;
    }
    Some(skip_leading_whitespace_and_comments(&sql[keyword.len()..]))
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

    fn param(name: &str, value: &str) -> AlterSessionParameter {
        AlterSessionParameter {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    // `str::to_uppercase` is Unicode-aware and does not preserve UTF-8 length:
    // 'ı' (U+0131, 2 bytes) uppercases to 'I' (1 byte) and 'ﬁ' (U+FB01, 3 bytes)
    // to "FI" (2 bytes). A keyword offset taken from an uppercased copy is
    // therefore not an offset into the original once such a character precedes
    // it. Each of these queries is one the server accepts, so the parse runs.

    #[test]
    fn test_parse_all_shrinking_char_before_alter_in_literal_is_not_a_statement() {
        let results = parse_all_alter_sessions("SELECT 'ıalter ego'");
        assert!(
            results.is_empty(),
            "no ALTER SESSION SET is present, got {results:?}"
        );
    }

    #[test]
    fn test_parse_all_shrinking_char_before_alter_in_bind_value_is_not_a_statement() {
        let results = parse_all_alter_sessions("INSERT INTO t VALUES ('ﬁalter')");
        assert!(
            results.is_empty(),
            "no ALTER SESSION SET is present, got {results:?}"
        );
    }

    #[test]
    fn test_parse_all_finds_alter_after_shrinking_char_in_earlier_literal() {
        let results = parse_all_alter_sessions("SELECT 'ı';ALTER SESSION SET QUERY_TAG = 'v'");
        assert_eq!(results, vec![param("QUERY_TAG", "v")]);
    }

    #[test]
    fn test_parse_all_finds_alter_after_shrinking_char_in_leading_comment() {
        let results = parse_all_alter_sessions("/* ı */ALTER SESSION SET QUERY_TAG = 'v'");
        assert_eq!(results, vec![param("QUERY_TAG", "v")]);
    }

    #[test]
    fn test_unquoted_value_ends_at_end_of_line() {
        let result = parse_alter_session("ALTER SESSION SET TIMEZONE = UTC\nSELECT 1");
        assert_eq!(result, Some(param("TIMEZONE", "UTC")));
    }

    #[test]
    fn test_parse_all_skips_alter_nested_in_an_already_parsed_value() {
        let results = parse_all_alter_sessions(
            "ALTER SESSION SET QUERY_TAG = 'has alter session set x=1 inside'",
        );
        assert_eq!(
            results,
            vec![param("QUERY_TAG", "has alter session set x=1 inside")]
        );
    }
}
