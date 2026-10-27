"""Static checks for dictionary rows from ``cursor(DictCursor)``.

Run with mypy. ``tests/unit/test_dict_cursor_typing.py`` invokes that check.
"""

from __future__ import annotations

from typing import Any, assert_type

from snowflake.connector import Connection, DictCursor, SnowflakeCursor
from snowflake.connector.aio import Connection as AsyncConnection
from snowflake.connector.aio import DictCursor as AsyncDictCursor
from snowflake.connector.aio import SnowflakeCursor as AsyncSnowflakeCursor


def dict_rows(connection: Connection) -> None:
    with connection.cursor(DictCursor) as cur:
        assert_type(cur, DictCursor)
        executed = cur.execute("SELECT c1, c2 FROM example")
        assert_type(executed, DictCursor)
        fetched = cur.fetchall()
        assert_type(fetched, list[dict[str, Any]])
        for row in fetched:
            assert_type(row, dict[str, Any])
            value: Any = row["C1"]
            assert_type(value, Any)
        one = cur.fetchone()
        if one is not None:
            assert_type(one, dict[str, Any])
            value = one["C1"]
            assert_type(value, Any)
        batch = cur.fetchmany()
        assert_type(batch, list[dict[str, Any]])
        for row in batch:
            assert_type(row, dict[str, Any])
            value = row["C1"]
            assert_type(value, Any)
        for row in cur:
            assert_type(row, dict[str, Any])
            value = row["C1"]
            assert_type(value, Any)
        many = cur.executemany("INSERT INTO example (c1) VALUES (%s)", [["a"]])
        assert_type(many, DictCursor)
        inserted = many.fetchall()
        assert_type(inserted, list[dict[str, Any]])
        nxt = cur.next()
        assert_type(nxt, dict[str, Any])
        value = nxt["C1"]
        assert_type(value, Any)

    chained = connection.cursor(DictCursor).execute("SELECT c1 FROM example").fetchall()
    assert_type(chained, list[dict[str, Any]])
    for row in chained:
        assert_type(row, dict[str, Any])
        value = row["C1"]
        assert_type(value, Any)


def tuple_rows(connection: Connection) -> None:
    with connection.cursor() as cur:
        assert_type(cur, SnowflakeCursor)
        executed = cur.execute("SELECT 1")
        assert_type(executed, SnowflakeCursor)
        one = executed.fetchone()
        if one is not None:
            assert_type(one, tuple[Any, ...])
            value: Any = one[0]
            assert_type(value, Any)
        rows = cur.fetchall()
        assert_type(rows, list[tuple[Any, ...]])
        for row in rows:
            assert_type(row, tuple[Any, ...])
            value = row[0]
            assert_type(value, Any)
        batch = cur.fetchmany()
        assert_type(batch, list[tuple[Any, ...]])
        for row in batch:
            assert_type(row, tuple[Any, ...])
            value = row[0]
            assert_type(value, Any)
        for row in cur:
            assert_type(row, tuple[Any, ...])
            value = row[0]
            assert_type(value, Any)
        many = cur.executemany("INSERT INTO example (c1) VALUES (%s)", [["a"]])
        assert_type(many, SnowflakeCursor)
        nxt = cur.next()
        assert_type(nxt, tuple[Any, ...])
        indexed: Any = nxt[0]
        assert_type(indexed, Any)


async def async_dict_rows(connection: AsyncConnection) -> None:
    async with connection.cursor(AsyncDictCursor) as cur:
        assert_type(cur, AsyncDictCursor)
        executed = await cur.execute("SELECT c1, c2 FROM example")
        assert_type(executed, AsyncDictCursor)
        fetched = await cur.fetchall()
        assert_type(fetched, list[dict[str, Any]])
        for row in fetched:
            assert_type(row, dict[str, Any])
            value: Any = row["C1"]
            assert_type(value, Any)
        one = await cur.fetchone()
        if one is not None:
            assert_type(one, dict[str, Any])
            value = one["C1"]
            assert_type(value, Any)
        batch = await cur.fetchmany()
        assert_type(batch, list[dict[str, Any]])
        many = await cur.executemany("INSERT INTO example (c1) VALUES (%s)", [["a"]])
        assert_type(many, AsyncDictCursor)
        nxt = await cur.next()
        assert_type(nxt, dict[str, Any])
        value = nxt["C1"]
        assert_type(value, Any)
        async for row in cur:
            assert_type(row, dict[str, Any])
            value = row["C1"]
            assert_type(value, Any)

    chained = await (await connection.cursor(AsyncDictCursor).execute("SELECT c1 FROM example")).fetchall()
    assert_type(chained, list[dict[str, Any]])


async def async_tuple_rows(connection: AsyncConnection) -> None:
    async with connection.cursor() as cur:
        assert_type(cur, AsyncSnowflakeCursor)
        executed = await cur.execute("SELECT 1")
        assert_type(executed, AsyncSnowflakeCursor)
        one = await executed.fetchone()
        if one is not None:
            assert_type(one, tuple[Any, ...])
            value: Any = one[0]
            assert_type(value, Any)
        rows = await cur.fetchall()
        assert_type(rows, list[tuple[Any, ...]])
        batch = await cur.fetchmany()
        assert_type(batch, list[tuple[Any, ...]])
        many = await cur.executemany("INSERT INTO example (c1) VALUES (%s)", [["a"]])
        assert_type(many, AsyncSnowflakeCursor)
        nxt = await cur.next()
        assert_type(nxt, tuple[Any, ...])
        indexed: Any = nxt[0]
        assert_type(indexed, Any)
        async for row in cur:
            assert_type(row, tuple[Any, ...])
            indexed = row[0]
            assert_type(indexed, Any)


class _CustomCursor(SnowflakeCursor):
    pass


class _AsyncCustomCursor(AsyncSnowflakeCursor):
    pass


def custom_cursor_subclass(connection: Connection, async_connection: AsyncConnection) -> None:
    cur = connection.cursor(_CustomCursor)
    assert_type(cur, _CustomCursor)
    async_cur = async_connection.cursor(_AsyncCustomCursor)
    assert_type(async_cur, _AsyncCustomCursor)


def rejected_cursor_classes(connection: Connection, async_connection: AsyncConnection) -> None:
    connection.cursor(int)  # type: ignore[type-var]
    connection.cursor(AsyncDictCursor)  # type: ignore[type-var]
    async_connection.cursor(DictCursor)  # type: ignore[type-var]
