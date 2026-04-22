"""Public API for writing pandas DataFrames to Snowflake tables.

Usage:
    from snowflake.connector.pandas_tools import write_pandas
    success, nchunks, nrows, _ = write_pandas(conn, df, 'my_table')

Implementation details are in snowflake.connector._internal.write_pandas_operation.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal

from ._internal.extras import pandas, requires_dependency
from ._internal.write_pandas_operation import (
    WritePandasConfig,
    WritePandasOperation,
    WritePandasResult,
)


if TYPE_CHECKING:
    from pandas import DataFrame

    from .connection import Connection


@requires_dependency(pandas)
def write_pandas(
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
    overwrite: bool = False,
    table_type: Literal["", "temp", "temporary", "transient"] = "",
    use_logical_type: bool | None = None,
    iceberg_config: dict[str, str] | None = None,
    bulk_upload_chunks: bool = False,
    use_vectorized_scanner: bool = False,
    **kwargs: Any,
) -> WritePandasResult:
    """Write a pandas DataFrame to a Snowflake table via Parquet stage upload.

    Returns a WritePandasResult named tuple (success, nchunks, nrows, copy_results).
    Backward-compatible with plain tuple unpacking and indexing.
    """
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
    return WritePandasOperation(cfg).execute()


def build_location_helper(
    database: str | None,
    schema: str | None,
    name: str,
    quote_identifiers: bool,
) -> str:
    """Format a fully-qualified stage/table/file-format location string."""

    def _escape(part: str) -> str:
        if "'" in part or quote_identifiers:
            if not part.startswith('"'):
                part = '"' + part
            if not part.endswith('"'):
                part = part + '"'
        return part

    parts = []
    if database:
        parts.append(_escape(database))
    if schema:
        parts.append(_escape(schema))
    parts.append(_escape(name))
    return ".".join(parts)


def _create_temp_stage(
    cursor: Any,
    database: str | None,
    schema: str | None,
    quote_identifiers: bool,
    compression: str,
    auto_create_table: bool,
    overwrite: bool,
    use_scoped_temp_object: bool = False,
) -> str:
    """Create a temporary stage and return its fully-qualified location.

    Stub — raises NotImplementedError.
    """
    raise NotImplementedError("_create_temp_stage is not yet implemented in the Universal Driver.")


def _create_temp_file_format(
    cursor: Any,
    database: str | None,
    schema: str | None,
    quote_identifiers: bool,
    compression: str,
    sql_use_logical_type: str,
    use_scoped_temp_object: bool = False,
) -> str:
    """Create a temporary file format and return its fully-qualified location.

    Stub — raises NotImplementedError.
    """
    raise NotImplementedError("_create_temp_file_format is not yet implemented in the Universal Driver.")
