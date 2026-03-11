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
