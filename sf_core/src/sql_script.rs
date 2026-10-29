//! Lexical splitter for a SQL script.
//!
//! Finds statement boundaries while tracking quotes, `$$` bodies, and comments.
//! It does not parse statement grammar. An unterminated quote, block comment, or
//! dollar quote keeps the rest of the script in that statement.

use std::fmt;

/// How to split one script.
///
/// `delimiter` is fixed for the whole script. `remove_comments` drops comments
/// from [`StatementSpan::text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitOptions {
    pub delimiter: String,
    pub remove_comments: bool,
}

impl Default for SplitOptions {
    fn default() -> Self {
        Self {
            delimiter: ";".to_owned(),
            remove_comments: false,
        }
    }
}

/// `Sql` is ordinary statement text. `Command` is a `!` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementKind {
    Sql,
    Command,
}

/// One statement in a script.
///
/// `start` and `end` are Unicode scalar offsets into the original script, not
/// byte offsets. The range is the source extent before stripping. [`Self::text`]
/// rewrites the delimiter as `;`, applies comment removal, and strips
/// surrounding whitespace, so it is not always `script[start..end]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementSpan {
    pub start: usize,
    pub end: usize,
    pub kind: StatementKind,
    pub is_file_transfer: Option<bool>,
    pub text: String,
}

/// `SplitOptions::delimiter` was empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptySqlDelimiter;

impl fmt::Display for EmptySqlDelimiter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SQL delimiter must not be empty")
    }
}

impl std::error::Error for EmptySqlDelimiter {}

/// Split `sql` into statement spans.
///
/// Malformed input does not fail: an unterminated quote, block comment, or
/// dollar quote retains the rest of the script as one statement.
pub fn split_sql_script(
    sql: &str,
    options: &SplitOptions,
) -> Result<Vec<StatementSpan>, EmptySqlDelimiter> {
    let mut lexer = Lexer::new(options)?;
    let mut byte_start = 0;
    let mut char_base = 0usize;
    for (i, c) in sql.char_indices() {
        if c == '\n' {
            let line = &sql[byte_start..=i];
            lexer.push_line(char_base, line);
            char_base += line.chars().count();
            byte_start = i + c.len_utf8();
        }
    }
    if byte_start < sql.len() {
        lexer.push_line(char_base, &sql[byte_start..]);
    }
    lexer.finish();
    Ok(lexer.out)
}

struct Delimiter {
    chars: Vec<char>,
    raw: String,
    bound_start: bool,
    bound_end: bool,
}

impl Delimiter {
    fn parse(raw: &str) -> Result<Self, EmptySqlDelimiter> {
        let chars: Vec<char> = raw.chars().collect();
        let (Some(&first), Some(&last)) = (chars.first(), chars.last()) else {
            return Err(EmptySqlDelimiter);
        };
        Ok(Self {
            bound_start: is_word_char(first),
            bound_end: is_word_char(last),
            chars,
            raw: raw.to_owned(),
        })
    }

    fn matches(&self, line: &[char], col: usize) -> bool {
        let n = self.chars.len();
        let Some(found) = line.get(col..col + n) else {
            return false;
        };
        if found != self.chars.as_slice() {
            return false;
        }
        if self.bound_start
            && col > 0
            && let Some(&prev) = line.get(col - 1)
            && (is_word_char(prev) || prev == '$')
            && !self.chars.iter().all(|&c| c == prev)
        {
            return false;
        }
        if self.bound_end
            && let Some(next) = line.get(col + n)
            && (is_word_char(*next) || *next == '$')
        {
            return false;
        }
        true
    }
}

struct Lexer {
    remove_comments: bool,
    delimiter: Delimiter,
    in_quote: bool,
    quote_char: char,
    in_comment: bool,
    in_dollar: bool,
    pieces: Vec<(String, bool)>,
    origin_start: Option<usize>,
    origin_end: usize,
    out: Vec<StatementSpan>,
}

impl Lexer {
    fn new(options: &SplitOptions) -> Result<Self, EmptySqlDelimiter> {
        Ok(Self {
            remove_comments: options.remove_comments,
            delimiter: Delimiter::parse(&options.delimiter)?,
            in_quote: false,
            quote_char: '\'',
            in_comment: false,
            in_dollar: false,
            pieces: Vec::new(),
            origin_start: None,
            origin_end: 0,
            out: Vec::new(),
        })
    }

    fn push_line(&mut self, line_base: usize, line: &str) {
        let chars: Vec<char> = line.chars().collect();
        let mut col = 0usize;
        let mut col0 = 0usize;
        loop {
            if col >= chars.len() {
                self.finish_line(line_base, &chars, col0, col.min(chars.len()));
                break;
            }
            if self.in_comment {
                if has_prefix(&chars, col, &['*', '/']) {
                    self.in_comment = false;
                    if self.remove_comments {
                        self.note(line_base + col0, line_base + col + 2);
                    } else {
                        self.push_source(line_base, &chars, col0, col + 2, false);
                    }
                    col += 2;
                    col0 = col;
                } else {
                    col += 1;
                }
                continue;
            }
            if self.in_dollar {
                if has_prefix(&chars, col, &['$', '$']) {
                    self.in_dollar = false;
                    self.push_source(line_base, &chars, col0, col + 2, false);
                    col += 2;
                    col0 = col;
                } else {
                    col += 1;
                }
                continue;
            }
            if self.in_quote {
                if chars[col] == '\\'
                    && chars
                        .get(col + 1)
                        .is_some_and(|next| *next == self.quote_char || *next == '\\')
                {
                    col += 2;
                } else if chars[col] == self.quote_char {
                    let at_end = col + 1 == chars.len();
                    let next_is_quote = chars.get(col + 1) == Some(&self.quote_char);
                    if at_end || !next_is_quote {
                        self.in_quote = false;
                        self.push_source(line_base, &chars, col0, col + 1, true);
                        col += 1;
                        col0 = col;
                    } else {
                        col += 2;
                    }
                } else {
                    col += 1;
                }
                continue;
            }
            if chars[col] == '\'' || chars[col] == '"' {
                self.in_quote = true;
                self.quote_char = chars[col];
                col += 1;
                continue;
            }
            if chars[col] == ' ' || chars[col] == '\t' {
                self.push_source(line_base, &chars, col0, col + 1, true);
                col += 1;
                col0 = col;
                continue;
            }
            if is_inline_comment(&chars, col) && !window_has_scheme(&chars, col0, col) {
                self.push_source(line_base, &chars, col0, col, true);
                if self.remove_comments {
                    self.note(line_base + col, line_base + chars.len());
                    self.pieces.push(("\n".to_owned(), true));
                } else {
                    self.push_source(line_base, &chars, col, chars.len(), false);
                }
                break;
            }
            if has_prefix(&chars, col, &['/', '*'])
                && !has_prefix(&chars, col0, &['f', 'i', 'l', 'e', ':', '/', '/'])
            {
                if self.remove_comments {
                    self.push_source(line_base, &chars, col0, col, false);
                    self.note(line_base + col, line_base + col + 2);
                } else {
                    self.push_source(line_base, &chars, col0, col + 2, false);
                }
                col += 2;
                col0 = col;
                self.in_comment = true;
                continue;
            }
            if has_prefix(&chars, col, &['$', '$']) {
                self.push_source(line_base, &chars, col0, col + 2, true);
                col += 2;
                col0 = col;
                self.in_dollar = true;
                continue;
            }
            if self.delimiter.matches(&chars, col) {
                let mut text = chars
                    .get(col0..col)
                    .map(chars_to_string)
                    .unwrap_or_default();
                text.push(';');
                self.note(line_base + col0, line_base + col);
                let delim_end = col + self.delimiter.chars.len();
                self.note(line_base + col, line_base + delim_end);
                self.pieces.push((text, true));
                col = delim_end;
                if chars.get(col) == Some(&'>') {
                    self.note(line_base + col, line_base + col + 1);
                    if let Some((piece, _)) = self.pieces.last_mut() {
                        piece.push('>');
                    }
                    col += 1;
                }
                if rest_is_dash_comment(&chars, col) || rest_is_blank(&chars, col) {
                    if self.remove_comments {
                        self.note(line_base + col, line_base + chars.len());
                    } else {
                        self.push_source(line_base, &chars, col, chars.len(), false);
                    }
                    col = chars.len();
                }
                while chars.get(col).is_some_and(|c| *c == ' ' || *c == '\t') {
                    col += 1;
                }
                self.emit_sql();
                col0 = col;
                continue;
            }
            if col == 0 && chars[col] == '!' {
                if !self.pieces.is_empty() {
                    self.emit_sql();
                }
                let text = command_text(line, &self.delimiter.raw);
                self.note(line_base, line_base + chars.len());
                self.emit(StatementKind::Command, (text, Some(false)));
                break;
            }
            col += 1;
        }
    }

    fn finish_line(&mut self, line_base: usize, line: &[char], col0: usize, col: usize) {
        if col0 >= col {
            return;
        }
        if !self.in_comment {
            self.push_source(line_base, line, col0, col, true);
        } else if !self.remove_comments {
            self.push_source(line_base, line, col0, col, false);
        } else {
            self.note(line_base + col0, line_base + col);
        }
    }

    fn finish(&mut self) {
        if !self.pieces.is_empty() {
            self.emit_sql();
        }
    }

    fn emit_sql(&mut self) {
        let produced = concatenate(&self.pieces);
        self.emit(StatementKind::Sql, produced);
    }

    fn push_source(
        &mut self,
        line_base: usize,
        line: &[char],
        from: usize,
        to: usize,
        is_statement: bool,
    ) {
        self.note(line_base + from, line_base + to);
        let text = line.get(from..to).map(chars_to_string).unwrap_or_default();
        self.pieces.push((text, is_statement));
    }

    fn note(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        if self.origin_start.is_none() {
            self.origin_start = Some(start);
        }
        self.origin_end = self.origin_end.max(end);
    }

    fn emit(&mut self, kind: StatementKind, text_and_flag: (String, Option<bool>)) {
        let (text, is_file_transfer) = text_and_flag;
        let start = self.origin_start.unwrap_or(self.origin_end);
        let end = self.origin_end.max(start);
        self.out.push(StatementSpan {
            start,
            end,
            kind,
            is_file_transfer,
            text,
        });
        self.pieces.clear();
        self.origin_start = None;
        self.origin_end = 0;
    }
}

fn concatenate(pieces: &[(String, bool)]) -> (String, Option<bool>) {
    let mut joined = String::new();
    let mut is_file_transfer = None;
    for (text, is_statement) in pieces {
        joined.push_str(text);
        if is_file_transfer.is_none() && *is_statement && python_trim(text).chars().count() >= 3 {
            let prefix: String = text.chars().take(3).collect();
            let prefix = prefix.to_uppercase();
            is_file_transfer = Some(prefix == "PUT" || prefix == "GET");
        }
    }
    (python_trim(&joined).to_owned(), is_file_transfer)
}

fn command_text(line: &str, delimiter: &str) -> String {
    let stripped = python_trim(line);
    let without = stripped.strip_suffix(delimiter).unwrap_or(stripped);
    python_trim(without).to_owned()
}

fn chars_to_string(chars: &[char]) -> String {
    chars.iter().collect()
}

fn has_prefix(line: &[char], col: usize, prefix: &[char]) -> bool {
    line.get(col..).is_some_and(|rest| rest.starts_with(prefix))
}

fn is_inline_comment(line: &[char], col: usize) -> bool {
    has_prefix(line, col, &['-', '-']) || has_prefix(line, col, &['/', '/'])
}

fn window_has_scheme(line: &[char], col0: usize, col: usize) -> bool {
    let end = (col + 3).min(line.len());
    let start = col0.min(end);
    line.get(start..end)
        .is_some_and(|window| window.windows(3).any(|w| w == [':', '/', '/']))
}

fn rest_is_dash_comment(line: &[char], col: usize) -> bool {
    let i = skip_python_whitespace(line, col);
    has_prefix(line, i, &['-', '-'])
}

fn rest_is_blank(line: &[char], col: usize) -> bool {
    skip_python_whitespace(line, col) == line.len()
}

fn skip_python_whitespace(line: &[char], mut i: usize) -> usize {
    while line.get(i).is_some_and(|c| is_python_whitespace(*c)) {
        i += 1;
    }
    i
}

/// Python 3 `str.isspace` / `re` `\s`: Unicode `White_Space` plus the four
/// information separators that Python counts and Rust `is_whitespace` does not.
fn is_python_whitespace(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{1f}')
}

fn python_trim(text: &str) -> &str {
    text.trim_matches(is_python_whitespace)
}

/// Python 3 `re` `\w` for the delimiter boundary: alphanumeric or `_`.
fn is_word_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(sql: &str, options: &SplitOptions) -> Vec<(String, Option<bool>)> {
        split_sql_script(sql, options)
            .expect("delimiter")
            .into_iter()
            .map(|span| (span.text, span.is_file_transfer))
            .collect()
    }

    fn default_texts(sql: &str) -> Vec<(String, Option<bool>)> {
        texts(sql, &SplitOptions::default())
    }

    fn pair(text: &str, flag: Option<bool>) -> (String, Option<bool>) {
        (text.to_owned(), flag)
    }

    #[test]
    fn splits_on_semicolons_outside_quotes_comments_and_dollar_quotes() {
        assert_eq!(
            default_texts("select 1; select 2;"),
            vec![
                pair("select 1;", Some(false)),
                pair("select 2;", Some(false))
            ]
        );
        assert_eq!(
            default_texts("select 'a;b';"),
            vec![pair("select 'a;b';", Some(false))]
        );
        assert_eq!(
            default_texts("select 'a''b';"),
            vec![pair("select 'a''b';", Some(false))]
        );
        assert_eq!(
            default_texts("select \"a;b\";"),
            vec![pair("select \"a;b\";", Some(false))]
        );
        assert_eq!(
            default_texts("select \"a\"\"b\";"),
            vec![pair("select \"a\"\"b\";", Some(false))]
        );
        assert_eq!(
            default_texts("select 'a\\'b';"),
            vec![pair("select 'a\\'b';", Some(false))]
        );
        assert_eq!(
            default_texts("select $$a;b$$;"),
            vec![pair("select $$a;b$$;", Some(false))]
        );
        assert_eq!(
            default_texts("select $$\n;$$\n;"),
            vec![pair("select $$\n;$$\n;", Some(false))]
        );
        assert_eq!(
            default_texts("select /* c; */ 1;"),
            vec![pair("select /* c; */ 1;", Some(false))]
        );
        assert_eq!(
            default_texts("select 1; /*\n; */ select 2;"),
            vec![
                pair("select 1;", Some(false)),
                pair("/*\n; */ select 2;", Some(false)),
            ]
        );
    }

    #[test]
    fn unterminated_quote_comment_or_dollar_quote_keeps_the_rest() {
        assert_eq!(
            default_texts("select 'unterminated"),
            vec![pair("select 'unterminated", Some(false))]
        );
        assert_eq!(
            default_texts("/* unterminated"),
            vec![pair("/* unterminated", None)]
        );
        assert_eq!(
            default_texts("select $$unterminated"),
            vec![pair("select $$unterminated", Some(false))]
        );
        assert_eq!(default_texts(""), Vec::<(String, Option<bool>)>::new());
    }

    #[test]
    fn comments_and_urls() {
        assert_eq!(
            default_texts("select 1; -- tail\nselect 2;"),
            vec![
                pair("select 1; -- tail", Some(false)),
                pair("select 2;", Some(false)),
            ]
        );
        assert_eq!(
            default_texts("select 1; // tail\nselect 2;"),
            vec![
                pair("select 1;", Some(false)),
                pair("// tail\nselect 2;", Some(false)),
            ]
        );
        assert_eq!(
            default_texts("select s3://bucket/key;"),
            vec![pair("select s3://bucket/key;", Some(false))]
        );
        assert_eq!(
            default_texts("select 'https://snowflake.com/path';"),
            vec![pair("select 'https://snowflake.com/path';", Some(false))]
        );
        assert_eq!(
            default_texts("file://foo/* not comment */ select 1;"),
            vec![pair("file://foo/* not comment */ select 1;", Some(false))]
        );
        let removed = SplitOptions {
            remove_comments: true,
            ..SplitOptions::default()
        };
        assert_eq!(
            texts("select 1; -- c\nselect /*x*/ 2;", &removed),
            vec![
                pair("select 1;", Some(false)),
                pair("select  2;", Some(false))
            ]
        );
        assert_eq!(
            texts("select 1; /*\n; */ select 2;", &removed),
            vec![
                pair("select 1;", Some(false)),
                pair("select 2;", Some(false))
            ]
        );
        assert_eq!(texts("/* only */", &removed), vec![pair("", None)]);
        assert_eq!(
            texts(
                "select\nc1,\nc2, -- issue's here?\nc3\nfrom test;",
                &removed
            ),
            vec![pair("select\nc1,\nc2, \nc3\nfrom test;", Some(false))]
        );
    }

    #[test]
    fn commands_put_get_and_custom_delimiter() {
        assert_eq!(default_texts("!quit"), vec![pair("!quit", Some(false))]);
        assert_eq!(
            default_texts("select 1\n!quit\n"),
            vec![pair("select 1", Some(false)), pair("!quit", Some(false))]
        );
        assert_eq!(
            default_texts("!set delimiter ;\nselect 1;"),
            vec![
                pair("!set delimiter", Some(false)),
                pair("select 1;", Some(false)),
            ]
        );
        assert_eq!(
            default_texts("PUT file://a @s;"),
            vec![pair("PUT file://a @s;", Some(true))]
        );
        assert_eq!(
            default_texts("  get file://a @s;"),
            vec![pair("get file://a @s;", Some(true))]
        );
        assert_eq!(
            default_texts("PUTS file://a @s;"),
            vec![pair("PUTS file://a @s;", Some(true))]
        );
        assert_eq!(
            default_texts("/* c */ PUT x;"),
            vec![pair("/* c */ PUT x;", Some(true))]
        );
        assert_eq!(default_texts(";"), vec![pair(";", None)]);
        assert_eq!(default_texts(";;"), vec![pair(";", None), pair(";", None)]);
        assert_eq!(
            default_texts("select 1;> select 2;"),
            vec![
                pair("select 1;>", Some(false)),
                pair("select 2;", Some(false))
            ]
        );

        let go = SplitOptions {
            delimiter: "GO".to_owned(),
            ..SplitOptions::default()
        };
        assert_eq!(
            texts("select 1 GO\nselect 2 GO", &go),
            vec![
                pair("select 1 ;", Some(false)),
                pair("select 2 ;", Some(false))
            ]
        );
        assert_eq!(
            texts("select 1GO select 2", &go),
            vec![pair("select 1GO select 2", Some(false))]
        );
        assert_eq!(
            texts("select $GO x GO", &go),
            vec![pair("select $GO x ;", Some(false))]
        );
        assert_eq!(
            texts("select café GO", &go),
            vec![pair("select café ;", Some(false))]
        );
        let word = |delimiter: &str| SplitOptions {
            delimiter: delimiter.to_owned(),
            ..SplitOptions::default()
        };
        assert_eq!(texts("AA", &word("A")), vec![pair("A;", None)]);
        assert_eq!(texts("AAA", &word("A")), vec![pair("AA;", Some(false))]);
        assert_eq!(texts("AAB", &word("A")), vec![pair("AAB", Some(false))]);
        assert_eq!(texts("BA", &word("A")), vec![pair("BA", None)]);
        assert_eq!(texts("BAA", &word("A")), vec![pair("BA;", Some(false))]);
        assert_eq!(texts("0__", &word("_")), vec![pair("0_;", Some(false))]);
        assert_eq!(texts("11", &word("1")), vec![pair("1;", None)]);
        assert_eq!(texts("GGG", &word("GG")), vec![pair("G;", None)]);
        assert_eq!(texts("éé", &word("é")), vec![pair("é;", None)]);
        assert_eq!(texts("caféé", &word("é")), vec![pair("café;", Some(false))]);
        assert_eq!(
            texts("A A", &word("A")),
            vec![pair(";", None), pair(";", None)]
        );
        assert_eq!(texts("GO", &go), vec![pair(";", None)]);
        assert_eq!(
            texts(
                "a|b|",
                &SplitOptions {
                    delimiter: "|".to_owned(),
                    ..SplitOptions::default()
                }
            ),
            vec![pair("a;", None), pair("b;", None)]
        );
    }

    #[test]
    fn spans_are_unicode_scalar_offsets() {
        let sql = "select '雪'; select 2;";
        let spans = split_sql_script(sql, &SplitOptions::default()).expect("delimiter");
        assert_eq!(spans.len(), 2);
        assert_eq!(
            scalar_slice(sql, spans[0].start, spans[0].end),
            "select '雪';"
        );
        assert_eq!(scalar_slice(sql, spans[1].start, spans[1].end), "select 2;");
        assert!(spans[0].end < sql.len());
        assert_eq!(spans[0].kind, StatementKind::Sql);
        assert_eq!(spans[1].kind, StatementKind::Sql);
    }

    #[test]
    fn default_sql_text_matches_stripped_source_span() {
        for sql in [
            "select 1; select 2;",
            "  select 1;  ",
            "select 1;\nselect 2;",
            "select 1;\r\nselect 2;",
            "select 1; -- tail\nselect 2;",
            "select 1; // tail\nselect 2;",
            ";;",
            "select 1;> select 2;",
            "   \n!cmd\n",
            "select '雪';",
            "/* only */",
            "-- only\n",
        ] {
            let spans = split_sql_script(sql, &SplitOptions::default()).expect("delimiter");
            let mut prev_end = 0usize;
            for span in &spans {
                assert!(span.start <= span.end, "{sql:?} {span:?}");
                assert!(span.start >= prev_end, "{sql:?} {span:?}");
                prev_end = span.end;
                if span.kind == StatementKind::Sql {
                    let slice = scalar_slice(sql, span.start, span.end);
                    assert_eq!(python_trim(&slice), span.text, "{sql:?}");
                }
            }
        }
    }

    #[test]
    fn empty_delimiter_is_rejected() {
        let options = SplitOptions {
            delimiter: String::new(),
            ..SplitOptions::default()
        };
        assert_eq!(
            split_sql_script("select 1;", &options).unwrap_err(),
            EmptySqlDelimiter
        );
    }

    fn scalar_slice(sql: &str, start: usize, end: usize) -> String {
        sql.chars().skip(start).take(end - start).collect()
    }
}
