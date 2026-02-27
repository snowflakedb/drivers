"""BACKWARD COMPATIBILITY MODULE ONLY"""

from __future__ import annotations

from typing import Any


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
    **kwargs: Any,
) -> tuple[bool, int, int, Any]:
    raise NotImplementedError("write_pandas is not yet implemented in the universal driver")
