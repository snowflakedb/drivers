from io import StringIO
from textwrap import dedent

import pytest

from snowflake.connector._internal.text_utils import split_statements


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
