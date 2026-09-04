"""Result metadata and query statistics types."""

from __future__ import annotations

from typing import Any, NamedTuple

from ..protobuf_gen.database_driver_v1_pb2 import (
    PrepareResult,
    QueryStats,
    ResultSetDescriptor,
)
from ..type_codes import get_type_code


_TEXT_TYPES = ("TEXT", "VARCHAR", "CHAR", "STRING")


def _column_display_size(col: Any) -> int | None:
    # Char count, text-only — a new-driver-only enhancement (BD#90); the old driver leaves it unset.
    return col.length if col.HasField("length") and col.type.upper() in _TEXT_TYPES else None


def _column_internal_size(col: Any) -> int | None:
    # Char count (old-driver semantics), ungated — BINARY/VARBINARY need a real value too.
    return col.length if col.HasField("length") else None


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

        display_size = _column_display_size(col)
        internal_size = _column_internal_size(col)
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


class ResultMetadataV2:
    """New-format column description carrying ``vector_dimension`` and ``fields``.

    Matches the legacy ``snowflake-connector-python`` ``ResultMetadataV2``
    interface so Snowpark can read ``.name``, ``.type_code``, ``.is_nullable``,
    ``.vector_dimension``, and ``.fields`` without connector-version guards.

    ``vector_dimension`` is populated from the proto ``dimension`` field.
    ``fields`` carries nested metadata for structured types: the element type
    for VECTOR and ARRAY, the key and value types for MAP, and one entry per
    attribute for structured OBJECT. It is ``None`` for every other type, and
    also for semi-structured ARRAY/OBJECT/MAP, which the server describes
    without a nested field list.

    Note: ``_is_nullable`` is exposed both as the ``is_nullable`` public property
    and as the ``_is_nullable`` private attribute, because Snowpark accesses the
    private attribute directly on nested ARRAY/MAP element metadata objects
    (``obj._is_nullable``). Both must exist by their exact names.
    """

    __slots__ = (
        "_name",
        "_type_code",
        "_is_nullable",
        "_display_size",
        "_internal_size",
        "_precision",
        "_scale",
        "_vector_dimension",
        "_fields",
    )

    def __init__(
        self,
        name: str | None,
        type_code: int,
        is_nullable: bool,
        display_size: int | None = None,
        internal_size: int | None = None,
        precision: int | None = None,
        scale: int | None = None,
        vector_dimension: int | None = None,
        fields: list[ResultMetadataV2] | None = None,
    ) -> None:
        self._name = name
        self._type_code = type_code
        self._is_nullable = is_nullable
        self._display_size = display_size
        self._internal_size = internal_size
        self._precision = precision
        self._scale = scale
        self._vector_dimension = vector_dimension
        self._fields = fields

    @property
    def name(self) -> str | None:
        return self._name

    @property
    def type_code(self) -> int:
        return self._type_code

    @property
    def is_nullable(self) -> bool:
        return self._is_nullable

    @property
    def display_size(self) -> int | None:
        return self._display_size

    @property
    def internal_size(self) -> int | None:
        return self._internal_size

    @property
    def precision(self) -> int | None:
        return self._precision

    @property
    def scale(self) -> int | None:
        return self._scale

    @property
    def vector_dimension(self) -> int | None:
        return self._vector_dimension

    @property
    def fields(self) -> list[ResultMetadataV2] | None:
        return self._fields

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, ResultMetadataV2):
            return NotImplemented
        return (
            self._name == other._name
            and self._type_code == other._type_code
            and self._is_nullable == other._is_nullable
            and self._display_size == other._display_size
            and self._internal_size == other._internal_size
            and self._precision == other._precision
            and self._scale == other._scale
            and self._vector_dimension == other._vector_dimension
            and self._fields == other._fields
        )

    def _to_result_metadata_v1(self) -> ResultMetadata:
        """Downcast to a PEP 249-compatible ``ResultMetadata`` NamedTuple."""
        return ResultMetadata(
            name=self._name,  # type: ignore[arg-type]  # sub-fields carry name=None (legacy parity)
            type_code=self._type_code,
            display_size=self._display_size,
            internal_size=self._internal_size,
            precision=self._precision,
            scale=self._scale,
            is_nullable=self._is_nullable,
        )

    def __repr__(self) -> str:
        return (
            f"ResultMetadataV2(name={self._name!r}, type_code={self._type_code!r}, "
            f"is_nullable={self._is_nullable!r}, vector_dimension={self._vector_dimension!r}, "
            f"fields={self._fields!r})"
        )

    @classmethod
    def from_column(cls, col: Any, *, nested: bool = False) -> ResultMetadataV2:
        """Build from a proto ``ColumnMetadata`` message.

        ``nested`` marks a sub-field of a structured type. Those carry a name
        only for the attributes of a structured OBJECT; the proto renders an
        absent name as the empty string, which maps back to ``None`` here for
        parity with the old driver.
        """
        type_code = get_type_code(col.type)
        display_size = _column_display_size(col)
        internal_size = _column_internal_size(col)
        precision = col.precision if col.HasField("precision") else None
        scale = col.scale if col.HasField("scale") else None
        vector_dimension = col.dimension if col.HasField("dimension") else None
        return cls(
            name=(col.name or None) if nested else col.name,
            type_code=type_code,
            is_nullable=col.nullable,
            display_size=display_size,
            internal_size=internal_size,
            precision=precision,
            scale=scale,
            vector_dimension=vector_dimension,
            # For MAP, `col.fields` carries exactly two entries, key type then value
            # type, in that order. Snowpark's structured-type inference reads them
            # positionally as fields[0]/fields[1] today, so this recursion must
            # preserve wire order rather than resort or drop an entry.
            fields=[cls.from_column(f, nested=True) for f in col.fields] if col.fields else None,
        )

    @classmethod
    def create_description(cls, result: PrepareResult | ResultSetDescriptor | None) -> list[ResultMetadataV2] | None:
        """Build a V2 description list from a prepare/describe result."""
        if result and result.columns:
            return [cls.from_column(col) for col in result.columns]
        return None


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
