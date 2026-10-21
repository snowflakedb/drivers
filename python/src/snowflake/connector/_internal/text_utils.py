from __future__ import annotations

import re

from collections.abc import Iterator
from io import StringIO


COMMENT_PATTERN_RE = re.compile(r"^\s*\-\-")
EMPTY_LINE_RE = re.compile(r"^\s*$")
INLINE_COMMENT_PREFIXES = ("--", "//")

_VALUES_CLAUSE_RE = re.compile(r"\bVALUES\s*\(", re.IGNORECASE)

# Snowflake Scripting support: semicolons inside BEGIN...END blocks don't end the statement.
_WORD_RE = re.compile(r"\w+")
# Keywords after which BEGIN opens a block (CREATE PROCEDURE ... AS BEGIN, IF ... THEN BEGIN, etc.).
_BLOCK_OPENERS = frozenset({"AS", "DO", "THEN", "ELSE", "LOOP"})
# Tokens which, directly following BEGIN, show that it doesn't open a block (BEGIN TRANSACTION, `x AS begin`).
_NOT_BLOCK_AFTER_BEGIN = frozenset({"TRANSACTION", "WORK", "FROM", ",", ")"})
# END followed by one of these closes a compound statement rather than a BEGIN block.
_COMPOUND_END_KEYWORDS = frozenset({"IF", "LOOP", "FOR", "WHILE", "CASE", "REPEAT"})

# Matches only the tokens that affect parser state.  Two-char tokens are
# listed first so the alternation is greedy and consumes them whole:
#   $$   — dollar-quote delimiter
#   ''   — escaped single-quote inside a single-quoted string
#   ""   — escaped double-quote inside a double-quoted string
#   ( ) ' "  — individual state-change characters
# Everything between tokens is skipped at C speed by the regex engine,
# avoiding a Python-level loop over every character.
_SQL_TOKENS_RE = re.compile(r"\$\$|''|\"\"|[()'\"]")


def extract_values_clause(sql: str) -> str | None:
    """Extract the VALUES clause from an INSERT SQL statement.

    Uses a balanced-parentheses parser rather than a greedy regex so that
    nested function calls (e.g. ``PARSE_JSON(%(col)s)``) and string literals
    that contain parentheses are handled correctly.

    Examples that a greedy regex gets wrong but this function handles:

    * ``INSERT INTO t (raw) (SELECT PARSE_JSON(c) as raw FROM VALUES (%(raw)s))``
      → returns ``(%(raw)s)``  (greedy regex returns ``(%(raw)s))``)
    * ``INSERT INTO t (col) VALUES (PARSE_JSON(%(col)s))``
      → returns ``(PARSE_JSON(%(col)s))``

    Returns:
        The VALUES clause including its outer parentheses, e.g. ``(%(col)s)``,
        or ``None`` when no balanced VALUES clause can be found.
    """
    m = _VALUES_CLAUSE_RE.search(sql)
    if not m:
        return None

    start = m.end() - 1  # position of the opening '('
    depth = 0
    in_single_quote = False
    in_double_quote = False
    in_dollar_quote = False

    for tok in _SQL_TOKENS_RE.finditer(sql, start):
        t = tok.group()

        if in_dollar_quote:
            if t == "$$":
                in_dollar_quote = False
        elif in_single_quote:
            if t == "'":
                in_single_quote = False
            # t == "''" → escaped quote, stay in string (fall through, do nothing)
        elif in_double_quote:
            if t == '"':
                in_double_quote = False
            # t == '""' → escaped quote, stay in string (fall through, do nothing)
        else:
            if t == "(":
                depth += 1
            elif t == ")":
                depth -= 1
                if depth == 0:
                    return sql[start : tok.end()]
            elif t == "'":
                in_single_quote = True
            elif t == '"':
                in_double_quote = True
            elif t == "$$":
                in_dollar_quote = True

    return None  # unbalanced or no VALUES clause found


class SQLDelimiter:
    """Class that wraps a SQL delimiter string.

    Since split_statements is a generator this mutable object will allow it change while executing.
    """

    def __str__(self) -> str:
        return self.sql_delimiter

    def __init__(self, sql_delimiter: str = ";") -> None:
        """Initialize SQLDelimiter with a string."""
        self.sql_delimiter = sql_delimiter


def split_statements(
    buf: StringIO,
    remove_comments: bool = False,
    delimiter: SQLDelimiter | None = None,
) -> Iterator[tuple[str, bool | None]]:
    """Split a stream into SQL statements (ends with a semicolon) or commands (!...).

    Delimiters inside Snowflake Scripting BEGIN...END blocks (including a preceding DECLARE
    section) don't split, so a block is returned as a single statement. If a block is still open
    at the end of the input, the text from the start of that statement is split as if blocks
    weren't tracked.

    Args:
        buf: Unicode data stream.
        remove_comments: Whether or not to remove all comments (Default value = False).
        delimiter: The delimiter string that separates SQL commands from each other.

    Yields:
        A SQL statement or a command.
    """
    if delimiter is None:
        delimiter = SQLDelimiter()  # Use default delimiter if none was given.
    yield from _split_statements(buf, remove_comments, delimiter, track_blocks=True)


def _split_statements(
    buf: StringIO,
    remove_comments: bool,
    delimiter: SQLDelimiter,
    track_blocks: bool,
) -> Iterator[tuple[str, bool | None]]:
    """Split statements as split_statements does, optionally without BEGIN...END tracking."""
    in_quote = False
    ch_quote = None
    in_comment = False
    in_double_dollars = False
    previous_delimiter = None
    # BEGIN...END block tracking. `blocks` is the stack of open BEGIN blocks and CASEs, since
    # a CASE expression's END would otherwise close the enclosing BEGIN. A BEGIN is tentatively
    # counted as a block opener and undone if the next token shows otherwise (e.g. BEGIN; or
    # BEGIN TRANSACTION). Likewise, what an END closes depends on the next token (END; vs.
    # END IF), which may be on a later line.
    blocks: list[str] = []
    pending_begin = False
    pending_end = False
    last_token = ""  # last significant token in the current statement ("" at statement start)
    in_declare = False  # inside the DECLARE section that precedes a block's BEGIN
    # Source text of the current top-level statement, so it can be re-split if a block is never closed
    raw_lines: list[str] = []
    raw_start = 0

    line = buf.readline()
    statement = []
    while line != "":
        raw_lines.append(line)
        col = 0
        col0 = 0
        len_line = len(line)
        sql_delimiter = delimiter.sql_delimiter
        if not previous_delimiter or sql_delimiter != previous_delimiter:
            # Only (re)compile new Regexes if they should be
            escaped_delim = re.escape(sql_delimiter)
            # Special characters possible in the sql delimiter are '_', '/' and ';'. If a delimiter does not end, or
            # start with a special character then look for word separation with \b regex.
            if re.match(r"\w", sql_delimiter[0]):
                RE_START = re.compile(rf"^[^\w$]?{escaped_delim}")
            else:
                RE_START = re.compile(rf"^.?{escaped_delim}")
            if re.match(r"\w", sql_delimiter[-1]):
                RE_END = re.compile(rf"{escaped_delim}[^\w$]?$")
            else:
                RE_END = re.compile(rf"{escaped_delim}.?$")
            previous_delimiter = sql_delimiter
        while True:
            if col >= len_line:
                if col0 < col:
                    if not in_comment and not in_quote and not in_double_dollars:
                        statement.append((line[col0:col], True))
                        if len(statement) == 1 and statement[0][0] == "":
                            statement = []
                        break
                    elif not in_comment and (in_quote or in_double_dollars):
                        statement.append((line[col0:col], True))
                    elif not remove_comments:
                        statement.append((line[col0:col], False))
                break
            elif in_comment:
                if line[col:].startswith("*/"):
                    in_comment = False
                    if not remove_comments:
                        statement.append((line[col0 : col + 2], False))
                    col += 2
                    col0 = col
                else:
                    col += 1
            elif in_double_dollars:
                if line[col:].startswith("$$"):
                    in_double_dollars = False
                    statement.append((line[col0 : col + 2], False))
                    col += 2
                    col0 = col
                else:
                    col += 1
            elif in_quote:
                if line[col] == "\\" and col < len_line - 1 and line[col + 1] in (ch_quote, "\\"):
                    col += 2
                elif line[col] == ch_quote:
                    if (col < len_line - 1 and line[col + 1] != ch_quote) or col == len_line - 1:
                        # exits quote
                        in_quote = False
                        statement.append((line[col0 : col + 1], True))
                        col += 1
                        col0 = col
                    else:
                        # escaped quote and still in quote
                        col += 2
                else:
                    col += 1
            else:
                if line[col] in ("'", '"'):
                    in_quote = True
                    ch_quote = line[col]
                    col += 1
                elif line[col] in (" ", "\t"):
                    statement.append((line[col0 : col + 1], True))
                    col += 1
                    col0 = col
                elif line[col:].startswith(INLINE_COMMENT_PREFIXES) and "://" not in line[col0 : col + 3]:
                    statement.append((line[col0:col], True))
                    if not remove_comments:
                        # keep the comment
                        statement.append((line[col:], False))
                    else:
                        statement.append(("\n", True))
                    col = len_line + 1
                    col0 = col
                elif line[col:].startswith("/*") and not line[col0:].startswith("file://"):
                    if not remove_comments:
                        statement.append((line[col0 : col + 2], False))
                    else:
                        statement.append((line[col0:col], False))
                    col += 2
                    col0 = col
                    in_comment = True
                elif line[col:].startswith("$$"):
                    statement.append((line[col0 : col + 2], True))
                    col += 2
                    col0 = col
                    in_double_dollars = True
                elif (
                    RE_START.match(line[col - 1 : col + len(sql_delimiter)])
                    if col > 0
                    else (RE_START.match(line[col : col + len(sql_delimiter)]))
                ) and (RE_END.match(line[col : col + len(sql_delimiter) + 1])):
                    if pending_begin:
                        # BEGIN; is a transaction, not a block
                        pending_begin = False
                        blocks.pop()
                    if pending_end:
                        pending_end = False
                        blocks.pop()
                    if "BEGIN" in blocks or in_declare:
                        # inside a block: the delimiter ends a nested statement, not this one
                        last_token = ";"
                        col += len(sql_delimiter)
                        continue
                    last_token = ""
                    blocks.clear()  # discard any unclosed CASE
                    statement.append((line[col0:col] + ";", True))
                    col += len(sql_delimiter)
                    try:
                        if line[col] == ">":
                            col += 1
                            statement[-1] = (statement[-1][0] + ">", statement[-1][1])
                    except IndexError:
                        pass
                    if COMMENT_PATTERN_RE.match(line[col:]) or EMPTY_LINE_RE.match(line[col:]):
                        if not remove_comments:
                            # keep the comment
                            statement.append((line[col:], False))
                        col = len_line
                    while col < len_line and line[col] in (" ", "\t"):
                        col += 1
                    yield _concatenate_statements(statement)
                    col0 = col
                    statement = []
                    raw_lines = [line]
                    raw_start = col
                elif col == 0 and line[col] == "!":  # command
                    if len(statement) > 0:
                        yield _concatenate_statements(statement)
                        statement = []
                    yield (
                        (
                            line.strip()[: -len(sql_delimiter)]
                            if line.strip().endswith(sql_delimiter)
                            else line.strip()
                        ).strip(),
                        False,
                    )
                    raw_lines = []
                    raw_start = 0
                    break
                elif track_blocks and _is_word_start(line, col):
                    word_match = _WORD_RE.match(line, col)
                    word = word_match.group().upper()
                    closes_compound = pending_end and word in _COMPOUND_END_KEYWORDS
                    if pending_end:
                        pending_end = False
                        # END CASE closes a CASE statement; END IF etc. close constructs we don't track
                        if not closes_compound or word == "CASE":
                            blocks.pop()
                    if pending_begin:
                        pending_begin = False
                        if word in _NOT_BLOCK_AFTER_BEGIN:
                            blocks.pop()
                    if word == "CASE" and not closes_compound:
                        blocks.append("CASE")
                    elif word == "DECLARE" and last_token in ("", ";", "AS"):
                        in_declare = True
                    elif word == "BEGIN" and (in_declare or last_token in ("", ";") or last_token in _BLOCK_OPENERS):
                        blocks.append("BEGIN")
                        pending_begin = not in_declare
                        in_declare = False
                    elif word == "END" and blocks:
                        pending_end = True
                    last_token = word
                    col = word_match.end()
                else:
                    if not line[col].isspace():
                        if pending_end:
                            pending_end = False
                            blocks.pop()
                        if pending_begin:
                            pending_begin = False
                            if line[col] in _NOT_BLOCK_AFTER_BEGIN:
                                blocks.pop()
                        last_token = line[col]
                    col += 1
        line = buf.readline()

    if ("BEGIN" in blocks or in_declare) and len(statement) > 0:
        # The block was never closed (malformed, or a BEGIN that wasn't a block opener after all),
        # so split the rest of the input as if blocks weren't tracked rather than swallowing it.
        remainder = StringIO("".join(raw_lines)[raw_start:])
        yield from _split_statements(remainder, remove_comments, delimiter, track_blocks=False)
    elif len(statement) > 0:
        yield _concatenate_statements(statement)


def _is_word_start(line: str, col: int) -> bool:
    """Whether a word (identifier or keyword) starts at line[col]."""
    if not (line[col].isalnum() or line[col] == "_"):
        return False
    return col == 0 or not (line[col - 1].isalnum() or line[col - 1] in ("_", "$"))


def _concatenate_statements(statement_list: list[tuple[str, bool]]) -> tuple[str, bool | None]:
    """Concatenate statements.

    Each statement should be a tuple of statement and is_put_or_get.

    The is_put_or_get is set to True if the statement is PUT or GET otherwise False for valid statement.
    None is set if the statement is empty or comment only.

    Args:
        statement_list: List of statement parts.

    Returns:
        Tuple of statements and whether they are PUT or GET.
    """
    valid_statement_list = []
    is_put_or_get = None
    for text, is_statement in statement_list:
        valid_statement_list.append(text)
        if is_put_or_get is None and is_statement and len(text.strip()) >= 3:
            is_put_or_get = text[:3].upper() in ("PUT", "GET")
    return "".join(valid_statement_list).strip(), is_put_or_get
