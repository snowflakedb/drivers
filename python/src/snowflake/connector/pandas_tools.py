"""BACKWARD COMPATIBILITY MODULE ONLY — pandas integration stubs."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any, Literal


def write_pandas(
    conn: Any,
    df: Any,
    table_name: str,
    database: str | None = None,
    schema: str | None = None,
    chunk_size: int | None = None,
    compression: str = "gzip",
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
) -> tuple[bool, int, int, Sequence[Any]]:
    """Write a pandas DataFrame to a Snowflake table.

    Stub — raises NotImplementedError. A full implementation requires staging
    support that is not yet present in the Universal Driver.
    """
    raise NotImplementedError(
        "write_pandas is not yet implemented in the Universal Driver. "
        "Use snowflake-connector-python for this operation."
    )
