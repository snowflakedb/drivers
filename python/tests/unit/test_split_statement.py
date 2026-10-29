from io import StringIO
from textwrap import dedent

import pytest

from snowflake.connector._internal.text_utils import SQLDelimiter, split_statements


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


def test_quotes_dollar_quotes_and_commands():
    script = "select 'a;b'; select $$x;y$$;\n!quit\n"
    assert list(split_statements(StringIO(script))) == [
        ("select 'a;b';", False),
        ("select $$x;y$$;", False),
        ("!quit", False),
    ]


def test_put_and_custom_delimiter():
    assert list(split_statements(StringIO("PUT file://a @stage;"))) == [("PUT file://a @stage;", True)]
    statements = list(split_statements(StringIO("select 1 GO\nselect 2 GO"), delimiter=SQLDelimiter("GO")))
    assert statements == [("select 1 ;", False), ("select 2 ;", False)]


def test_unterminated_quote_keeps_the_rest_of_the_script():
    assert list(split_statements(StringIO("select 'a;b\nselect 2;"))) == [("select 'a;b\nselect 2;", False)]


def test_repeated_word_character_delimiter():
    assert list(split_statements(StringIO("AA"), delimiter=SQLDelimiter("A"))) == [("A;", None)]
    assert list(split_statements(StringIO("0__"), delimiter=SQLDelimiter("_"))) == [("0_;", False)]
    assert list(split_statements(StringIO("AAB"), delimiter=SQLDelimiter("A"))) == [("AAB", False)]


def test_empty_delimiter():
    with pytest.raises(ValueError, match="SQL delimiter must not be empty"):
        list(split_statements(StringIO("select 1;"), delimiter=SQLDelimiter("")))
