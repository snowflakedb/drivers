from io import StringIO
from textwrap import dedent

import pytest

from snowflake.connector._internal.text_utils import SQLDelimiter, split_statements


def split(sql, **kwargs):
    """Return the full (statement, is_put_or_get) output of split_statements."""
    return list(split_statements(StringIO(sql), **kwargs))


def split_sql(sql, **kwargs):
    """Return only the non-empty statement strings."""
    return [s for s, _ in split(sql, **kwargs) if s]


@pytest.mark.parametrize("comment_prefix", ["--", "//"])
@pytest.mark.parametrize("remove_comments", [True, False])
def test_split_with_comment(comment_prefix, remove_comments):
    query = dedent(
        f"""
        use database test_db;
        use schema public;
        select
        c1,
        c2, {comment_prefix} issue's here?
        c3
        from test;
        select current_timestamp() as ts;
        """
    )

    statements = list(split_statements(StringIO(query), remove_comments=remove_comments))

    expected_comment = "" if remove_comments else f"{comment_prefix} issue's here?"
    assert statements == [
        ("use database test_db;", False),
        ("use schema public;", False),
        (f"select\nc1,\nc2, {expected_comment}\nc3\nfrom test;", False),
        ("select current_timestamp() as ts;", False),
    ]


@pytest.mark.parametrize(
    "query",
    [
        "select s3://bucket/key;",
        "select 'https://snowflake.com/path';",
    ],
)
def test_double_slash_in_url_is_not_comment(query):
    assert list(split_statements(StringIO(query), remove_comments=True)) == [(query, False)]


# ---------------------------------------------------------------------------
# Basic splitting and whitespace
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("sql", "expected"),
    [
        ("SELECT 1; SELECT 2; SELECT 3;", ["SELECT 1;", "SELECT 2;", "SELECT 3;"]),
        ("SELECT 1;SELECT 2;", ["SELECT 1;", "SELECT 2;"]),
        ("SELECT 1 ; SELECT 2 ;", ["SELECT 1 ;", "SELECT 2 ;"]),
        ("select\t1;\tselect 2;", ["select\t1;", "select 2;"]),
        ("select 1;\r\nselect 2;\r\n", ["select 1;", "select 2;"]),
        ("select 1;\nselect 2", ["select 1;", "select 2"]),
        ("SELECT 1", ["SELECT 1"]),
        ("select\n  a,\n  b\nfrom t;", ["select\n  a,\n  b\nfrom t;"]),
        ("select 'héllo;wörld' as 名前; select 2;", ["select 'héllo;wörld' as 名前;", "select 2;"]),
    ],
)
def test_basic_splitting(sql, expected):
    assert split(sql) == [(s, False) for s in expected]


def test_empty_input():
    assert split("") == []


def test_whitespace_only_input():
    assert split_sql("   \n\n   ") == []


def test_blank_lines_between_statements():
    assert split("  \n\nSELECT 1;\n\n\nSELECT 2;\n\n") == [
        ("SELECT 1;", False),
        ("SELECT 2;", False),
        ("", None),
    ]


def test_empty_statements_are_yielded_as_bare_semicolons():
    assert split(";;select 1;;") == [(";", None), (";", None), ("select 1;", False), (";", None)]


def test_semicolon_followed_by_gt_is_kept():
    # `;>` is a SnowSQL-ism: the `>` stays attached to the statement.
    assert split("select 1;>\nselect 2;") == [("select 1;>", False), ("select 2;", False)]


# ---------------------------------------------------------------------------
# Edge cases for quoting and dollar-quoting
# ---------------------------------------------------------------------------


def test_semicolon_inside_single_quoted_string():
    assert split("SELECT 'a;b;c' AS x;") == [("SELECT 'a;b;c' AS x;", False)]


def test_semicolon_inside_double_quoted_identifier():
    assert split('SELECT "col;name" FROM t;') == [('SELECT "col;name" FROM t;', False)]


def test_semicolon_inside_dollar_quoted_block():
    # Dollar-quoted strings should not be split on interior semicolons.
    assert split("SELECT $$hello;world$$ AS x;") == [("SELECT $$hello;world$$ AS x;", False)]


def test_multiple_dollar_quoted_blocks_in_one_statement():
    assert split("select $$a$$, $$b;c$$; select 2;") == [
        ("select $$a$$, $$b;c$$;", False),
        ("select 2;", False),
    ]


def test_escaped_single_quote_inside_string():
    assert split("SELECT 'it''s fine' AS x;") == [("SELECT 'it''s fine' AS x;", False)]


def test_escaped_double_quote_inside_identifier():
    assert split('select "a""b;c" from t; select 2;') == [
        ('select "a""b;c" from t;', False),
        ("select 2;", False),
    ]


def test_backslash_escaped_quote_inside_string():
    assert split(r"select 'a\'b;c' as x; select 2;") == [
        (r"select 'a\'b;c' as x;", False),
        ("select 2;", False),
    ]


def test_escaped_backslash_at_end_of_string():
    assert split(r"select 'a\\'; select 2;") == [(r"select 'a\\';", False), ("select 2;", False)]


def test_string_spanning_lines():
    assert split("select 'a;\nb' as x;\nselect 2;") == [
        ("select 'a;\nb' as x;", False),
        ("select 2;", False),
    ]


def test_multiline_dollar_quoted_body():
    sql = dedent("""\
        create function f() returns int language javascript as $$
         var a = 1;
         return a;
        $$;
        select 1;
    """)
    assert split(sql) == [
        ("create function f() returns int language javascript as $$\n var a = 1;\n return a;\n$$;", False),
        ("select 1;", False),
    ]


@pytest.mark.parametrize(
    "sql",
    [
        "select 'abc;\nselect 2;",
        "select $$abc;\nselect 2;",
    ],
    ids=["single_quote", "dollar_quote"],
)
def test_unterminated_quote_swallows_rest_of_input(sql):
    assert split(sql) == [(sql, False)]


def test_url_in_string_not_treated_as_comment():
    assert split("select 'http://x.com/a;b';") == [("select 'http://x.com/a;b';", False)]


# ---------------------------------------------------------------------------
# Comment handling
# ---------------------------------------------------------------------------


def test_block_comment_spanning_lines():
    sql = dedent("""\
        SELECT 1 /* this is
        a multiline comment */ AS x;
    """)
    assert split(sql) == [("SELECT 1 /* this is\na multiline comment */ AS x;", False)]


def test_block_comment_spanning_lines_removed():
    assert split("select 1 /* a\nb */ as x;", remove_comments=True) == [("select 1  as x;", False)]


def test_inline_comment_after_statement():
    # A trailing comment on the same line is attached to the preceding statement.
    assert split("SELECT 1; -- trailing comment\nSELECT 2;") == [
        ("SELECT 1; -- trailing comment", False),
        ("SELECT 2;", False),
    ]


def test_inline_comment_after_statement_removed():
    assert split("SELECT 1; -- trailing comment\nSELECT 2;", remove_comments=True) == [
        ("SELECT 1;", False),
        ("SELECT 2;", False),
    ]


def test_trailing_comment_after_last_of_several_statements_on_a_line():
    assert split("select 1; select 2; -- c\n") == [("select 1;", False), ("select 2; -- c", False)]


@pytest.mark.parametrize(
    ("remove_comments", "expected"),
    [(False, "-- hdr\nselect 1;"), (True, "select 1;")],
)
def test_leading_comment(remove_comments, expected):
    assert split("-- hdr\nselect 1;", remove_comments=remove_comments) == [(expected, False)]


def test_remove_comments_strips_block_comment():
    assert split("SELECT /* drop me */ 1;", remove_comments=True) == [("SELECT  1;", False)]


def test_remove_comments_strips_double_slash_comment():
    assert split("select 1 // c\n;", remove_comments=True) == [("select 1 \n;", False)]


@pytest.mark.parametrize(
    "sql",
    ["select 1 /* ; */ as x;", "select 1 -- ;\n as x;"],
    ids=["block", "line"],
)
def test_semicolon_inside_comment_does_not_split(sql):
    assert split(sql) == [(sql, False)]


@pytest.mark.parametrize(
    ("sql", "remove_comments", "expected"),
    [
        ("-- just a comment\n", False, [("-- just a comment", None)]),
        ("-- just a comment\n", True, [("", None)]),
        ("/* only */", False, [("/* only */", None)]),
    ],
)
def test_comment_only_input_has_none_flag(sql, remove_comments, expected):
    assert split(sql, remove_comments=remove_comments) == expected


def test_keywords_inside_comments_and_strings_are_ignored():
    sql = "-- BEGIN\nselect 1;\n/* END; */ select 2;\nselect 'BEGIN' as x; select 'END;' as y;"
    assert split_sql(sql) == [
        "-- BEGIN\nselect 1;",
        "/* END; */ select 2;",
        "select 'BEGIN' as x;",
        "select 'END;' as y;",
    ]


# ---------------------------------------------------------------------------
# PUT / GET detection
# ---------------------------------------------------------------------------


def test_put_and_get_are_flagged():
    sql = "PUT file:///tmp/a.csv @st;\nget @st file:///tmp/;\nselect 1;"
    assert split(sql) == [
        ("PUT file:///tmp/a.csv @st;", True),
        ("get @st file:///tmp/;", True),
        ("select 1;", False),
    ]


def test_put_with_leading_whitespace_is_flagged():
    assert split("  put file:///tmp/x @s;") == [("put file:///tmp/x @s;", True)]


def test_put_with_glob_is_not_block_comment():
    # `/*` inside a file:// URL must not open a block comment.
    assert split("put file:///tmp/*.csv @s;") == [("put file:///tmp/*.csv @s;", True)]


@pytest.mark.parametrize(
    "sql",
    ["-- c\nput file:///a @s;", "/* c */ get @s file:///a;"],
    ids=["line_comment", "block_comment"],
)
def test_put_get_after_leading_comment_is_flagged(sql):
    assert split(sql) == [(sql, True)]


# ---------------------------------------------------------------------------
# SnowSQL `!` commands
# ---------------------------------------------------------------------------


def test_bang_command_is_its_own_statement_without_semicolon():
    sql = "select 1;\n!set variable_substitution=true;\nselect 2;"
    assert split(sql) == [
        ("select 1;", False),
        ("!set variable_substitution=true", False),
        ("select 2;", False),
    ]


def test_bang_command_flushes_unterminated_statement():
    assert split("select 1\n!quit\n") == [("select 1", False), ("!quit", False)]


# ---------------------------------------------------------------------------
# Custom delimiters
# ---------------------------------------------------------------------------


def test_custom_word_delimiter():
    sql = "select 1 GO\nselect 2 GO\n"
    assert split(sql, delimiter=SQLDelimiter("GO")) == [("select 1 ;", False), ("select 2 ;", False)]


def test_custom_word_delimiter_requires_word_boundary():
    sql = "select GOAL from t GO\n"
    assert split(sql, delimiter=SQLDelimiter("GO")) == [("select GOAL from t ;", False)]


def test_delimiter_can_change_mid_stream():
    delimiter = SQLDelimiter()
    out = []
    for stmt in split_statements(StringIO("select 1;\nselect 2 GO\nselect 3 GO\n"), delimiter=delimiter):
        out.append(stmt)
        delimiter.sql_delimiter = "GO"
    assert out == [("select 1;", False), ("select 2 ;", False), ("select 3 ;", False)]


# ---------------------------------------------------------------------------
# Transactions — BEGIN here is NOT a scripting block and must always split
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("sql", "expected"),
    [
        (
            "BEGIN TRANSACTION; INSERT INTO t VALUES (1); COMMIT;",
            ["BEGIN TRANSACTION;", "INSERT INTO t VALUES (1);", "COMMIT;"],
        ),
        ("BEGIN WORK; SELECT 1; COMMIT;", ["BEGIN WORK;", "SELECT 1;", "COMMIT;"]),
        ("BEGIN; SELECT 1; COMMIT;", ["BEGIN;", "SELECT 1;", "COMMIT;"]),
        ("begin; select 1; commit;", ["begin;", "select 1;", "commit;"]),
        ("BEGIN\nTRANSACTION;\nSELECT 1;\nCOMMIT;", ["BEGIN\nTRANSACTION;", "SELECT 1;", "COMMIT;"]),
        (
            "BEGIN TRANSACTION NAME t1; SELECT 1; COMMIT;",
            ["BEGIN TRANSACTION NAME t1;", "SELECT 1;", "COMMIT;"],
        ),
        ("START TRANSACTION; SELECT 1; ROLLBACK;", ["START TRANSACTION;", "SELECT 1;", "ROLLBACK;"]),
    ],
    ids=["begin_transaction", "begin_work", "bare_begin", "bare_begin_lower", "multiline", "named", "start"],
)
def test_transaction_statements_split_normally(sql, expected):
    assert split(sql) == [(s, False) for s in expected]


# ---------------------------------------------------------------------------
# Identifiers / expressions that look like BEGIN / END but aren't blocks
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("sql", "expected"),
    [
        (
            "SELECT CASE WHEN a THEN 1 ELSE 2 END FROM t; SELECT 2;",
            ["SELECT CASE WHEN a THEN 1 ELSE 2 END FROM t;", "SELECT 2;"],
        ),
        (
            'SELECT begin_date, end_date, "END" FROM t; SELECT 2;',
            ['SELECT begin_date, end_date, "END" FROM t;', "SELECT 2;"],
        ),
        ("select 1 as begin; select 2;", ["select 1 as begin;", "select 2;"]),
        ("select 1 as begin\n; select 2;", ["select 1 as begin\n;", "select 2;"]),
    ],
    ids=["case_expression", "identifiers", "alias_begin", "alias_begin_newline"],
)
def test_begin_end_lookalikes_split_normally(sql, expected):
    assert split(sql) == [(s, False) for s in expected]


# ---------------------------------------------------------------------------
# Dollar-quoted scripting bodies — already work today
# ---------------------------------------------------------------------------


def test_procedure_with_dollar_quoted_body():
    sql = "CREATE PROCEDURE p() RETURNS INT LANGUAGE SQL AS $$\nBEGIN\n  RETURN 1;\nEND;\n$$;\nCALL p();"
    assert split(sql) == [
        ("CREATE PROCEDURE p() RETURNS INT LANGUAGE SQL AS $$\nBEGIN\n  RETURN 1;\nEND;\n$$;", False),
        ("CALL p();", False),
    ]


def test_execute_immediate_dollar_quoted_block():
    sql = "EXECUTE IMMEDIATE $$\nBEGIN\n  RETURN 1;\nEND;\n$$;"
    assert split(sql) == [(sql, False)]


# ---------------------------------------------------------------------------
# Unquoted scripting blocks — these currently FAIL (known gap).
# Marked strict xfail so they document expected-but-broken behavior and will
# fail loudly (prompting removal of the marker) once BEGIN...END support lands.
# ---------------------------------------------------------------------------

begin_end_xfail = pytest.mark.xfail(reason="split_statements does not yet track BEGIN...END depth", strict=True)


@begin_end_xfail
def test_stored_procedure_is_single_statement():
    sql = dedent("""\
        CREATE OR REPLACE PROCEDURE p()
        RETURNS INT
        LANGUAGE SQL
        AS
        BEGIN
          LET x INT := 1;
          RETURN x;
        END;
    """)
    assert split(sql) == [(sql.strip(), False)]


@begin_end_xfail
def test_stored_procedure_surrounded_by_statements():
    sql = dedent("""\
        USE DATABASE mydb;
        CREATE OR REPLACE PROCEDURE p()
        RETURNS INT LANGUAGE SQL
        AS BEGIN RETURN 1; END;
        SELECT 1;
    """)
    assert split_sql(sql) == [
        "USE DATABASE mydb;",
        "CREATE OR REPLACE PROCEDURE p()\nRETURNS INT LANGUAGE SQL\nAS BEGIN RETURN 1; END;",
        "SELECT 1;",
    ]


@begin_end_xfail
def test_task_with_begin_end_body():
    sql = "CREATE TASK t WAREHOUSE = w SCHEDULE = '1 minute' AS BEGIN INSERT INTO a VALUES (1); END; SELECT 1;"
    assert split_sql(sql) == [
        "CREATE TASK t WAREHOUSE = w SCHEDULE = '1 minute' AS BEGIN INSERT INTO a VALUES (1); END;",
        "SELECT 1;",
    ]


@begin_end_xfail
def test_anonymous_begin_end_block():
    sql = dedent("""\
        BEGIN
          LET x INT := 42;
          LET y INT := x + 1;
        END;
    """)
    assert split(sql) == [(sql.strip(), False)]


@begin_end_xfail
def test_anonymous_block_followed_by_statement_on_same_line():
    assert split_sql("BEGIN RETURN 1; END; SELECT 2;") == ["BEGIN RETURN 1; END;", "SELECT 2;"]


@begin_end_xfail
def test_lowercase_begin_end_block():
    assert split_sql("begin\n  let x := 1;\nend;\nselect 1;") == ["begin\n  let x := 1;\nend;", "select 1;"]


@begin_end_xfail
def test_declare_begin_end_block():
    sql = dedent("""\
        DECLARE
          x INT;
          y INT DEFAULT 0;
        BEGIN
          x := 1;
          RETURN x + y;
        END;
    """)
    assert split(sql) == [(sql.strip(), False)]


@begin_end_xfail
def test_procedure_with_declare_section():
    sql = dedent("""\
        CREATE OR REPLACE PROCEDURE p()
        RETURNS INT LANGUAGE SQL
        AS
        DECLARE
          x INT DEFAULT 1;
        BEGIN
          RETURN x;
        END;
        CALL p();
    """)
    assert split_sql(sql) == [sql.split("CALL")[0].strip(), "CALL p();"]


@begin_end_xfail
def test_nested_begin_end():
    sql = dedent("""\
        BEGIN
          IF TRUE THEN
            BEGIN
              LET x := 1;
            END;
          END IF;
        END;
    """)
    assert split(sql) == [(sql.strip(), False)]


@begin_end_xfail
def test_end_if_does_not_decrement_depth():
    sql = dedent("""\
        BEGIN
          IF TRUE THEN
            LET x := 1;
          END IF;
          RETURN x;
        END;
    """)
    assert split(sql) == [(sql.strip(), False)]


@begin_end_xfail
@pytest.mark.parametrize(
    "loop",
    [
        "FOR i IN 1 TO 3 DO\n    x := x + i;\n  END FOR;",
        "WHILE (x < 3) DO\n    x := x + 1;\n  END WHILE;",
        "LOOP\n    x := x + 1;\n    BREAK;\n  END LOOP;",
        "REPEAT\n    x := x + 1;\n  UNTIL (x > 3)\n  END REPEAT;",
        "CASE (x)\n    WHEN 1 THEN RETURN 'one';\n    ELSE RETURN 'other';\n  END CASE;",
    ],
    ids=["for", "while", "loop", "repeat", "case"],
)
def test_compound_end_keywords_do_not_decrement_depth(loop):
    sql = f"BEGIN\n  LET x INT := 0;\n  {loop}\n  RETURN x;\nEND;\nSELECT 1;"
    assert split_sql(sql) == [sql.rsplit("\n", 1)[0], "SELECT 1;"]


@begin_end_xfail
@pytest.mark.parametrize("opener", ["THEN", "ELSE", "DO"])
def test_begin_after_block_opener_keyword_is_nested(opener):
    body = {
        "THEN": "IF (TRUE) THEN BEGIN LET y := 1; END; END IF;",
        "ELSE": "IF (FALSE) THEN RETURN 0; ELSE BEGIN LET y := 1; END; END IF;",
        "DO": "FOR i IN 1 TO 2 DO BEGIN LET y := i; END; END FOR;",
    }[opener]
    sql = f"BEGIN {body} RETURN 1; END; SELECT 1;"
    assert split_sql(sql) == [f"BEGIN {body} RETURN 1; END;", "SELECT 1;"]


@begin_end_xfail
def test_exception_handler_inside_block():
    sql = dedent("""\
        BEGIN
          SELECT 1/0;
        EXCEPTION
          WHEN OTHER THEN
            RETURN 'error';
        END;
        SELECT 1;
    """)
    assert split_sql(sql) == [sql.split("SELECT 1;")[0].strip(), "SELECT 1;"]


@begin_end_xfail
def test_begin_transaction_inside_block_is_not_a_nested_block():
    sql = "BEGIN BEGIN TRANSACTION; INSERT INTO t VALUES (1); COMMIT; END; SELECT 1;"
    assert split_sql(sql) == ["BEGIN BEGIN TRANSACTION; INSERT INTO t VALUES (1); COMMIT; END;", "SELECT 1;"]


@begin_end_xfail
def test_quoted_and_commented_end_inside_block_is_ignored():
    sql = "BEGIN\n  LET s := 'END;';\n  -- END;\n  /* END; */\n  RETURN s;\nEND;\nSELECT 1;"
    assert split_sql(sql) == [sql.rsplit("\n", 1)[0], "SELECT 1;"]


@begin_end_xfail
@pytest.mark.parametrize("remove_comments", [True, False])
def test_block_respects_remove_comments(remove_comments):
    sql = "BEGIN -- start\n  RETURN 1; /* done */\nEND;"
    expected = "BEGIN \n  RETURN 1; \nEND;" if remove_comments else sql
    assert split(sql, remove_comments=remove_comments) == [(expected, False)]


@pytest.mark.xfail(
    reason="CASE expression END inside a block is indistinguishable from block END "
    "without a full parser; schemachange's splitter has the same limitation",
    strict=True,
)
def test_case_expression_inside_block():
    sql = "BEGIN\n  LET y := CASE WHEN TRUE THEN 1 ELSE 2 END;\n  RETURN y;\nEND;\nSELECT 1;"
    assert split_sql(sql) == [sql.rsplit("\n", 1)[0], "SELECT 1;"]
