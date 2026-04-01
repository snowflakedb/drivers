"""Result metadata and query statistics types."""

from __future__ import annotations

from typing import Any, NamedTuple

from .._internal.protobuf_gen.database_driver_v1_pb2 import (
    ExecuteResult,
    PrepareResult,
    QueryStats,
)
from .._internal.type_codes import get_type_code


class ResultMetadata(NamedTuple):
    """PEP 249 column description entry.

    Each item in ``Cursor.description`` is a ``ResultMetadata`` instance.
    Being a :class:`~typing.NamedTuple` it is fully tuple-compatible as
    required by the spec, while also providing named attribute access.
    """

    name: str
    type_code: int
    display_size: int | None
    internal_size: int | None
    precision: int | None
    scale: int | None
    is_nullable: bool | None

    @classmethod
    def from_column(cls, col: Any) -> ResultMetadata:
        """Create a ``ResultMetadata`` from a protobuf ``ColumnMetadata``."""
        type_code = get_type_code(col.type)

        display_size = (
            col.length if col.HasField("length") and col.type.upper() in ("TEXT", "VARCHAR", "CHAR", "STRING") else None
        )
        internal_size = col.byte_length if col.HasField("byte_length") else None
        precision = col.precision if col.HasField("precision") else None
        scale = col.scale if col.HasField("scale") else None

        return cls(
            name=col.name,
            type_code=type_code,
            display_size=display_size,
            internal_size=internal_size,
            precision=precision,
            scale=scale,
            is_nullable=col.nullable,
        )

    @classmethod
    def create_description(cls, result: ExecuteResult | PrepareResult | None) -> list[ResultMetadata] | None:
        """Extract description from execute result column metadata."""
        if result and result.columns:
            return [cls.from_column(col) for col in result.columns]
        return None


# Backward compatibility alias
ResultMetadataV2 = ResultMetadata


class QueryResultStats(NamedTuple):
    """DML operation statistics returned by Snowflake.

    Exposes per-operation row counts for INSERT, UPDATE, DELETE,
    and the number of duplicate rows skipped during DML.
    """

    num_rows_inserted: int | None = None
    num_rows_deleted: int | None = None
    num_rows_updated: int | None = None
    num_dml_duplicates: int | None = None

    @classmethod
    def from_query_stats(cls, s: QueryStats) -> QueryResultStats:
        """Create a ``QueryResultStats`` from a protobuf ``QueryStats``."""
        return cls(
            num_rows_inserted=s.num_rows_inserted if s.HasField("num_rows_inserted") else None,
            num_rows_deleted=s.num_rows_deleted if s.HasField("num_rows_deleted") else None,
            num_rows_updated=s.num_rows_updated if s.HasField("num_rows_updated") else None,
            num_dml_duplicates=s.num_dml_duplicates if s.HasField("num_dml_duplicates") else None,
        )
