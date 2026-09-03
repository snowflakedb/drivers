"""async_write_pandas e2e tests (universal driver only).

The async pipeline is exercised via the _LoopRunner blocking bridge used
throughout this suite — no pytest-asyncio dependency required.
"""

from __future__ import annotations

import math

from uuid import uuid4

import pandas as pd
import pytest

from tests.connector_factory import create_connection_with_adapter


@pytest.fixture
def aio_dependencies():
    from snowflake.connector import aio
    from snowflake.connector.aio.pandas_tools import write_pandas
    from tests._async_bridge import _LoopRunner

    return aio, write_pandas, _LoopRunner


SAMPLE_DATA = [
    ("Alice", 100),
    ("Bob", 200),
    ("Charlie", 300),
    ("Diana", 400),
    ("Eve", 500),
]
SAMPLE_DF = pd.DataFrame(SAMPLE_DATA, columns=["NAME", "SCORE"])


def _table(prefix: str) -> str:
    return f"{prefix}_{uuid4().hex[:8]}".upper()


@pytest.mark.skip_reference(reason="aio module is universal driver only")
class TestAsyncWritePandas:
    """Tests for async_write_pandas."""

    def test_should_write_a_dataframe_to_a_pre_created_table_and_read_it_back(
        self, connector_adapter, tmp_schema, aio_dependencies
    ):
        aio, write_pandas, loop_runner = aio_dependencies
        loop = loop_runner.instance()
        table_name = _table("AIO_WP_BASIC")
        fq_table = f"{tmp_schema}.{table_name}"

        with create_connection_with_adapter(connector_adapter) as sync_conn:
            config = sync_conn.config

        async def _run():
            async with aio.connect(config=config) as conn:
                cur = conn.cursor()
                try:
                    await cur.execute(
                        "CREATE OR REPLACE TEMPORARY TABLE IDENTIFIER(?) (NAME STRING, SCORE INT)",
                        parameters=(fq_table,),
                        _force_qmark_paramstyle=True,
                    )

                    # When write_pandas is called with the sample DataFrame
                    success, nchunks, nrows, _ = await write_pandas(
                        conn,
                        SAMPLE_DF,
                        table_name,
                        schema=tmp_schema,
                        quote_identifiers=False,
                    )
                    await cur.execute(
                        "SELECT * FROM IDENTIFIER(?)",
                        parameters=(fq_table,),
                        _force_qmark_paramstyle=True,
                    )
                    rows = await cur.fetchall()
                finally:
                    cur.close()
            return success, nchunks, nrows, rows

        success, nchunks, nrows, rows = loop.run(_run())

        # Then async_write_pandas should return success with correct chunk and row counts
        assert success is True
        assert nchunks == 1
        assert nrows == len(SAMPLE_DATA)
        # And SELECT from the table should return all original rows
        assert set(rows) == set(SAMPLE_DATA)

    def test_should_auto_create_table_from_dataframe_schema(self, connector_adapter, tmp_schema, aio_dependencies):
        aio, write_pandas, loop_runner = aio_dependencies
        loop = loop_runner.instance()
        table_name = _table("AIO_WP_AUTOCREATE")
        fq_table = f"{tmp_schema}.{table_name}"

        with create_connection_with_adapter(connector_adapter) as sync_conn:
            config = sync_conn.config

        async def _run():
            async with aio.connect(config=config) as conn:
                # When async_write_pandas is called with auto_create_table=True
                success, nchunks, nrows, _ = await write_pandas(
                    conn,
                    SAMPLE_DF,
                    table_name,
                    schema=tmp_schema,
                    quote_identifiers=False,
                    auto_create_table=True,
                    table_type="temp",
                )
                cur = conn.cursor()
                try:
                    await cur.execute(
                        "SELECT * FROM IDENTIFIER(?)",
                        parameters=(fq_table,),
                        _force_qmark_paramstyle=True,
                    )
                    rows = await cur.fetchall()
                finally:
                    cur.close()
            return success, nchunks, nrows, rows

        success, nchunks, nrows, rows = loop.run(_run())

        # Then the table should be created and contain all rows
        assert success is True
        assert nchunks == 1
        assert nrows == len(SAMPLE_DATA)
        assert set(rows) == set(SAMPLE_DATA)

    def test_should_write_dataframe_in_multiple_chunks(self, connector_adapter, tmp_schema, aio_dependencies):
        aio, write_pandas, loop_runner = aio_dependencies
        loop = loop_runner.instance()
        chunk_size = 2
        expected_chunks = math.ceil(len(SAMPLE_DATA) / chunk_size)
        table_name = _table("AIO_WP_CHUNKED")
        fq_table = f"{tmp_schema}.{table_name}"

        with create_connection_with_adapter(connector_adapter) as sync_conn:
            config = sync_conn.config

        async def _run():
            async with aio.connect(config=config) as conn:
                cur = conn.cursor()
                try:
                    await cur.execute(
                        "CREATE OR REPLACE TEMPORARY TABLE IDENTIFIER(?) (NAME STRING, SCORE INT)",
                        parameters=(fq_table,),
                        _force_qmark_paramstyle=True,
                    )

                    # When write_pandas is called with chunk_size=2
                    success, nchunks, nrows, _ = await write_pandas(
                        conn,
                        SAMPLE_DF,
                        table_name,
                        schema=tmp_schema,
                        quote_identifiers=False,
                        chunk_size=chunk_size,
                    )
                finally:
                    cur.close()
            return success, nchunks, nrows

        success, nchunks, nrows = loop.run(_run())

        # Then async_write_pandas should report the correct number of chunks
        assert success is True
        assert nchunks == expected_chunks
        assert nrows == len(SAMPLE_DATA)
