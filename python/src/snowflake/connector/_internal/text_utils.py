from __future__ import annotations

import re

from collections.abc import Iterator
from io import StringIO

from snowflake.connector._core import sf_core_python


_VALUES_CLAUSE_RE = re.compile(r"\bVALUES\s*\(", re.IGNORECASE)

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
    """Delimiter read from ``sql_delimiter`` when :func:`split_statements` is called."""

    def __str__(self) -> str:
        return self.sql_delimiter

    def __init__(self, sql_delimiter: str = ";") -> None:
        self.sql_delimiter = sql_delimiter


def split_statements(
    buf: StringIO,
    remove_comments: bool = False,
    delimiter: SQLDelimiter | None = None,
) -> Iterator[tuple[str, bool | None]]:
    """Yield SQL statements and ``!`` commands from ``buf``.

    A delimiter inside a quote, ``$$`` body, or comment does not end a statement.
    An unterminated quote, block comment, or dollar quote stays open through the
    rest of the script. Each statement's delimiter is rewritten as ``;``. The
    second item is ``True`` for ``PUT`` and ``GET``, ``False`` for other
    statements and commands, and ``None`` when the text is empty or comment-only.
    """
    if delimiter is None:
        delimiter = SQLDelimiter()
    yield from sf_core_python.split_sql_script(
        buf.read(),
        delimiter.sql_delimiter,
        remove_comments,
    )
