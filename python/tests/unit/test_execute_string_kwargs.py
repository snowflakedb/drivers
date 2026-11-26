import asyncio

from io import StringIO
from unittest.mock import AsyncMock, MagicMock, call, patch

import pytest


EXECUTE_KWARGS = {
    "_no_results": True,
    "statement_parameters": {"QUERY_TAG": "execute-kwargs-test"},
}

LEGACY_STATEMENT_PARAMS_KWARGS = {
    "_no_results": True,
    "_statement_params": {"QUERY_TAG": "execute-kwargs-test"},
}


_SPLIT_STATEMENTS = [("SELECT 1;", False), ("SELECT 2", False)]


@pytest.fixture(autouse=True)
def _stub_split_statements():
    with (
        patch(
            "snowflake.connector.connection._connection.split_statements",
            return_value=_SPLIT_STATEMENTS,
        ),
        patch(
            "snowflake.connector.aio.connection._connection.split_statements",
            return_value=_SPLIT_STATEMENTS,
        ),
    ):
        yield


def _assert_kwargs_forwarded(execute_mock):
    assert execute_mock.call_args_list == [
        call("SELECT 1;", _is_put_get=False, **EXECUTE_KWARGS),
        call("SELECT 2", _is_put_get=False, **EXECUTE_KWARGS),
    ]


def test_execute_string_forwards_kwargs_to_each_cursor(mock_db_api):
    from snowflake.connector.connection import Connection

    connection = Connection(user="test_user", account="test_account")
    cursor = MagicMock()
    connection.cursor = MagicMock(return_value=cursor)

    connection.execute_string(
        "SELECT 1; SELECT 2",
        return_cursors=False,
        **EXECUTE_KWARGS,
    )

    _assert_kwargs_forwarded(cursor.execute)


def test_execute_stream_forwards_kwargs_to_each_cursor(mock_db_api):
    from snowflake.connector.connection import Connection

    connection = Connection(user="test_user", account="test_account")
    cursor = MagicMock()
    connection.cursor = MagicMock(return_value=cursor)

    list(
        connection.execute_stream(
            StringIO("SELECT 1; SELECT 2"),
            **EXECUTE_KWARGS,
        )
    )

    _assert_kwargs_forwarded(cursor.execute)


def test_async_execute_string_forwards_kwargs_to_each_cursor(mock_async_db_api):
    from snowflake.connector.aio.connection import Connection as AsyncConnection

    async def run():
        async with AsyncConnection(user="test_user", account="test_account") as connection:
            cursor = MagicMock()
            cursor.execute = AsyncMock()
            connection.cursor = MagicMock(return_value=cursor)

            await connection.execute_string(
                "SELECT 1; SELECT 2",
                return_cursors=False,
                **EXECUTE_KWARGS,
            )

            _assert_kwargs_forwarded(cursor.execute)

    asyncio.run(run())


def test_async_execute_stream_forwards_kwargs_to_each_cursor(mock_async_db_api):
    from snowflake.connector.aio.connection import Connection as AsyncConnection

    async def run():
        async with AsyncConnection(user="test_user", account="test_account") as connection:
            cursor = MagicMock()
            cursor.execute = AsyncMock()
            connection.cursor = MagicMock(return_value=cursor)

            async for _ in connection.execute_stream(
                StringIO("SELECT 1; SELECT 2"),
                **EXECUTE_KWARGS,
            ):
                pass

            _assert_kwargs_forwarded(cursor.execute)

    asyncio.run(run())


def test_execute_string_legacy_statement_params_warns_once_and_forwards_public_name(mock_db_api):
    from snowflake.connector.connection import Connection

    connection = Connection(user="test_user", account="test_account")
    cursor = MagicMock()
    connection.cursor = MagicMock(return_value=cursor)

    with pytest.warns(DeprecationWarning, match="statement_parameters") as caught:
        connection.execute_string(
            "SELECT 1; SELECT 2",
            return_cursors=False,
            **LEGACY_STATEMENT_PARAMS_KWARGS,
        )

    _assert_kwargs_forwarded(cursor.execute)
    matching = [w for w in caught if "_statement_params" in str(w.message)]
    assert len(matching) == 1
    assert matching[0].filename == __file__


def test_execute_stream_legacy_statement_params_warns_once_and_forwards_public_name(mock_db_api):
    from snowflake.connector.connection import Connection

    connection = Connection(user="test_user", account="test_account")
    cursor = MagicMock()
    connection.cursor = MagicMock(return_value=cursor)

    with pytest.warns(DeprecationWarning, match="statement_parameters") as caught:
        list(
            connection.execute_stream(
                StringIO("SELECT 1; SELECT 2"),
                **LEGACY_STATEMENT_PARAMS_KWARGS,
            )
        )

    _assert_kwargs_forwarded(cursor.execute)
    matching = [w for w in caught if "_statement_params" in str(w.message)]
    assert len(matching) == 1
    assert matching[0].filename == __file__


def test_async_execute_string_legacy_statement_params_warns_once_and_forwards_public_name(mock_async_db_api):
    from snowflake.connector.aio.connection import Connection as AsyncConnection

    async def run():
        async with AsyncConnection(user="test_user", account="test_account") as connection:
            cursor = MagicMock()
            cursor.execute = AsyncMock()
            connection.cursor = MagicMock(return_value=cursor)

            with pytest.warns(DeprecationWarning, match="statement_parameters") as caught:
                await connection.execute_string(
                    "SELECT 1; SELECT 2",
                    return_cursors=False,
                    **LEGACY_STATEMENT_PARAMS_KWARGS,
                )

            _assert_kwargs_forwarded(cursor.execute)
            matching = [w for w in caught if "_statement_params" in str(w.message)]
            assert len(matching) == 1
            assert matching[0].filename == __file__

    asyncio.run(run())


def test_async_execute_stream_legacy_statement_params_warns_once_and_forwards_public_name(mock_async_db_api):
    from snowflake.connector.aio.connection import Connection as AsyncConnection

    async def run():
        async with AsyncConnection(user="test_user", account="test_account") as connection:
            cursor = MagicMock()
            cursor.execute = AsyncMock()
            connection.cursor = MagicMock(return_value=cursor)

            with pytest.warns(DeprecationWarning, match="statement_parameters") as caught:
                async for _ in connection.execute_stream(
                    StringIO("SELECT 1; SELECT 2"),
                    **LEGACY_STATEMENT_PARAMS_KWARGS,
                ):
                    pass

            _assert_kwargs_forwarded(cursor.execute)
            matching = [w for w in caught if "_statement_params" in str(w.message)]
            assert len(matching) == 1
            assert matching[0].filename == __file__

    asyncio.run(run())
