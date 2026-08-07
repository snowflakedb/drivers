"""Async write_pandas for snowflake.connector.aio.

The sync write_pandas in snowflake.connector.pandas_tools is unchanged.
This module adds an aio-native write_pandas that runs the full
DDL → PUT → INFER_SCHEMA → COPY INTO pipeline on AsyncConnection /
AsyncSnowflakeCursor.
"""

from __future__ import annotations

import warnings

from collections.abc import Callable
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import TYPE_CHECKING, Any, Literal

from .._internal.decorators import api_telemetry
from .._internal.errorhandler import route_exception
from .._internal.extras import pandas, requires_dependency
from .._internal.logging import get_logger
from .._internal.write_pandas_operation import (
    WritePandasConfig,
    WritePandasMixin,
    WritePandasResult,
    generate_temp_name,
)
from ..errors import Error, ProgrammingError
from .cursor import SnowflakeCursor as AsyncSnowflakeCursor


if TYPE_CHECKING:
    from pandas import DataFrame

    from .connection import Connection

logger = get_logger(__name__)


# ---------------------------------------------------------------------------
# _AsyncWritePandasOperation
# ---------------------------------------------------------------------------


class _AsyncWritePandasOperation(WritePandasMixin):
    """Async-native write_pandas pipeline from a validated WritePandasConfig.

    execute() runs the full pipeline without mutating self — all intermediate
    state is threaded through local variables and return values.
    """

    def __init__(self, cfg: WritePandasConfig) -> None:
        self._cfg = cfg

    # -- Object lifetime -----------------------------------------------------

    async def _drop_object(self, cur: AsyncSnowflakeCursor, name: str, object_type: str) -> None:
        await cur.execute(**self._build_drop_object_sql(name, object_type))

    async def _create_temp_object(
        self,
        cur: AsyncSnowflakeCursor,
        sql_builder: Callable[[str], dict[str, Any]],
        qualified_name: str,
        bare_name: str,
    ) -> str:
        """Try creating in target schema; fall back to current schema on privilege error."""
        try:
            await cur.execute(**sql_builder(qualified_name))
            return qualified_name
        except ProgrammingError:
            await cur.execute(**sql_builder(bare_name))
            return bare_name

    # -- Main pipeline -------------------------------------------------------

    async def execute(self) -> WritePandasResult:
        cfg = self._cfg
        cfg.emit_warnings()

        cur: AsyncSnowflakeCursor = cfg.conn.cursor(AsyncSnowflakeCursor)  # type: ignore[assignment, arg-type]
        target_location: str | None = None
        try:
            stage_location = await self._create_stage(cur)
            nchunks, nrows = await self._upload_to_stage(cur, stage_location)

            column_type_map: dict[str, str] | None = None
            if cfg.needs_inference:
                file_format_location = await self._create_file_format(cur)
                column_type_map = await self._infer_column_types(cur, stage_location, file_format_location)

            target_location = self._resolve_target_table()

            if cfg.needs_table_creation:
                await self._create_table(cur, target_location, column_type_map)

            if cfg.needs_truncate:
                await self._truncate_table(cur, target_location)

            copy_results = await self._copy_into(cur, stage_location, target_location, column_type_map)

            if cfg.needs_swap:
                await self._swap_tables(cur, target_location)

        except Exception:
            if cfg.needs_swap and target_location:
                try:
                    await self._drop_object(cur, target_location, "TABLE")
                except Exception:
                    logger.warning(
                        "aio.write_pandas failed and could not drop temporary "
                        "staging table %s created for overwrite swap — "
                        "it may need to be dropped manually",
                        target_location,
                    )
            raise
        finally:
            await cur.close()

        success = all(row[1] == "LOADED" for row in copy_results)
        return WritePandasResult(success, nchunks, nrows, copy_results)

    # -- Stage & upload ------------------------------------------------------

    async def _create_stage(self, cur: AsyncSnowflakeCursor) -> str:
        cfg = self._cfg
        name = generate_temp_name("STAGE")
        qualified = cfg.qualify(name)
        return await self._create_temp_object(cur, self._build_create_stage_sql, qualified, name)

    async def _upload_to_stage(self, cur: AsyncSnowflakeCursor, stage_location: str) -> tuple[int, int]:
        cfg = self._cfg
        nchunks = 0
        nrows = 0
        with TemporaryDirectory() as tmp_dir:
            for idx, chunk in self._iter_chunks():
                chunk_path = Path(tmp_dir) / f"file{idx}.txt"
                chunk.to_parquet(path=chunk_path, compression=cfg.compression, **cfg.parquet_kwargs)
                nchunks += 1
                nrows += len(chunk)

                if not cfg.bulk_upload:
                    await self._put_file(cur, stage_location, chunk_path)
                    chunk_path.unlink()

            if cfg.bulk_upload:
                await self._put_directory(cur, stage_location, tmp_dir)

        return nchunks, nrows

    async def _put_file(self, cur: AsyncSnowflakeCursor, stage_location: str, path: Path) -> None:
        await cur.execute(**self._build_put_file_sql(stage_location, path))

    async def _put_directory(self, cur: AsyncSnowflakeCursor, stage_location: str, directory: str) -> None:
        await cur.execute(**self._build_put_directory_sql(stage_location, directory))

    # -- Schema inference ----------------------------------------------------

    async def _create_file_format(self, cur: AsyncSnowflakeCursor) -> str:
        cfg = self._cfg
        name = generate_temp_name("FILE_FORMAT")
        qualified = cfg.qualify(name)
        return await self._create_temp_object(cur, self._build_create_file_format_sql, qualified, name)

    async def _infer_column_types(
        self,
        cur: AsyncSnowflakeCursor,
        stage_location: str,
        file_format_location: str,
    ) -> dict[str, str]:
        """Run INFER_SCHEMA and return {UPPER_COL_NAME: SQL_TYPE} mapping."""
        await cur.execute(**self._build_infer_column_types_sql(stage_location, file_format_location))
        rows = await cur.fetchall()
        return {row[0].upper(): row[1] for row in rows}

    # -- Table management ----------------------------------------------------

    async def _create_table(
        self,
        cur: AsyncSnowflakeCursor,
        target_location: str,
        column_type_map: dict[str, str] | None,
    ) -> None:
        await cur.execute(**self._build_create_table_sql(target_location, column_type_map))

    async def _truncate_table(self, cur: AsyncSnowflakeCursor, target_location: str) -> None:
        await cur.execute(**self._build_truncate_table_sql(target_location))

    async def _swap_tables(self, cur: AsyncSnowflakeCursor, target_location: str) -> None:
        """Replace original table with temp target via DROP + RENAME.

        ALTER TABLE SWAP WITH would be atomic but doesn't work when the
        original table doesn't exist yet or tables are different types
        (TEMPORARY vs permanent).
        """
        original = self._cfg.qualify(self._cfg.table_name)
        await self._drop_object(cur, original, "TABLE")
        await cur.execute(**self._build_rename_table_sql(target_location))

    # -- COPY INTO -----------------------------------------------------------

    async def _copy_into(
        self,
        cur: AsyncSnowflakeCursor,
        stage_location: str,
        target_location: str,
        column_type_map: dict[str, str] | None,
    ) -> list:
        await cur.execute(**self._build_copy_into_sql(stage_location, target_location, column_type_map))
        return await cur.fetchall()


# ---------------------------------------------------------------------------
# Public async entry point
# ---------------------------------------------------------------------------


@requires_dependency(pandas)
@api_telemetry
async def write_pandas(
    conn: Connection,
    df: DataFrame,
    table_name: str,
    database: str | None = None,
    schema: str | None = None,
    chunk_size: int | None = None,
    compression: Literal["gzip", "snappy"] = "gzip",
    on_error: str = "abort_statement",
    parallel: int = 4,
    quote_identifiers: bool = True,
    infer_schema: bool = False,
    auto_create_table: bool = False,
    create_temp_table: bool = False,
    overwrite: bool = False,
    table_type: Literal["", "temp", "temporary", "transient"] = "",
    use_logical_type: bool | None = None,
    iceberg_config: dict[str, str] | None = None,
    bulk_upload_chunks: bool = False,
    use_vectorized_scanner: bool = False,
    **kwargs: Any,
) -> WritePandasResult:
    """Write a pandas DataFrame to a Snowflake table via Parquet stage upload (async).

    Async-native counterpart of :func:`snowflake.connector.pandas_tools.write_pandas`.
    Requires an :class:`~snowflake.connector.aio.Connection`.

    Returns a WritePandasResult named tuple (success, nchunks, nrows, copy_results).
    Backward-compatible with plain tuple unpacking and indexing.

    Note:
        Unlike the sync module, this module does not expose ``pd_writer`` or
        ``make_pd_writer``.  ``DataFrame.to_sql(method=...)`` invokes the method
        callback synchronously and cannot await a coroutine, so an async adapter
        for ``to_sql`` is not possible.  Use ``pd_writer`` from
        :mod:`snowflake.connector.pandas_tools` if you need ``to_sql`` integration.
    """
    # ``create_temp_table`` is the legacy boolean spelling of ``table_type="temp"``;
    # Snowpark still passes it. Translate it and do not forward it to the config.
    if create_temp_table and not table_type:
        table_type = "temp"
        warnings.warn(
            "'create_temp_table' is deprecated; use 'table_type=\"temp\"' instead.",
            DeprecationWarning,
            stacklevel=2,
        )
    try:
        cfg = WritePandasConfig(
            conn,
            df,
            table_name,
            database=database,
            schema=schema,
            chunk_size=chunk_size,
            compression=compression,
            on_error=on_error,
            parallel=parallel,
            quote_identifiers=quote_identifiers,
            infer_schema=infer_schema,
            auto_create_table=auto_create_table,
            overwrite=overwrite,
            table_type=table_type,
            use_logical_type=use_logical_type,
            iceberg_config=iceberg_config,
            bulk_upload_chunks=bulk_upload_chunks,
            use_vectorized_scanner=use_vectorized_scanner,
            **kwargs,
        )
        return await _AsyncWritePandasOperation(cfg).execute()
    except Error as exc:
        route_exception(conn, None, exc)
