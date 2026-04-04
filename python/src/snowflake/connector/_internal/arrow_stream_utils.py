from __future__ import annotations

from collections.abc import Sequence
from typing import TYPE_CHECKING

from .arrow_context import ArrowConverterContext
from .arrow_stream_iterator import ArrowStreamIterator, ArrowStreamTableIterator
from .extras import pyarrow
from .type_codes import FIXED


if TYPE_CHECKING:
    from pyarrow import Schema, Table

    from ..cursor import ResultMetadata


def release_arrow_stream(stream_ptr: int | None) -> None:
    # Release the Arrow stream pointer to prevent memory leak
    if stream_ptr:
        # Create ArrowStreamIterator to take ownership of the stream pointer.
        # The C++ destructor will handle cleanup when it goes out of scope.
        _ = ArrowStreamIterator(stream_ptr, ArrowConverterContext())
        # Iterator goes out of scope here, triggering C++ destructor


def create_row_iterator(
    stream_ptr: int,
    use_dict_result: bool = False,
    use_numpy: bool = False,
) -> ArrowStreamIterator:
    """Build an :class:`ArrowStreamIterator` that yields one row at a time."""
    context = ArrowConverterContext()
    return ArrowStreamIterator(
        stream_ptr,
        context,
        use_dict_result=use_dict_result,
        use_numpy=use_numpy,
    )


def create_table_iterator(
    stream_ptr: int,
    number_to_decimal: bool = False,
    force_microsecond_precision: bool = False,
) -> ArrowStreamTableIterator:
    """Build an :class:`ArrowStreamTableIterator` that yields one RecordBatch at a time."""
    context = ArrowConverterContext()
    return ArrowStreamTableIterator(
        stream_ptr,
        context,
        number_to_decimal=number_to_decimal,
        force_microsecond_precision=force_microsecond_precision,
    )


def normalize_fixed_column_types(
    schema: Schema,
    description: Sequence[ResultMetadata],
) -> Schema:
    """Rewrite FIXED columns in an Arrow schema to int64 for backward compatibility.

    When the result set has zero rows, sf-core may choose a narrower integer
    type (e.g. int8) for NUMBER columns.  The old driver always exposed int64,
    so we normalize here to keep behavior consistent.
    """
    new_fields = []
    changed = False
    for field, metadata in zip(schema, description):
        if metadata.type_code == FIXED and field.type != pyarrow.int64():
            new_fields.append(field.with_type(pyarrow.int64()))
            changed = True
        else:
            new_fields.append(field)
    return pyarrow.schema(new_fields) if changed else schema


def collect_arrow_table(
    table_iterator: ArrowStreamTableIterator,
    columns_metadata: Sequence[ResultMetadata] | None = None,
    force_return_table: bool = False,
) -> Table | None:
    """Collect all RecordBatches from *table_iterator* into a single Arrow Table.

    When *force_return_table* is ``True`` an empty result set produces a
    properly-typed empty table.  When ``False`` (the default), an empty result
    set returns ``None``.

    When *columns_metadata* is provided the empty-table schema is normalized via
    :func:`normalize_fixed_column_types` so that FIXED columns are int64.
    """
    batches = list(table_iterator)
    if batches:
        return pyarrow.Table.from_batches(batches)

    if not force_return_table:
        return None

    schema = table_iterator.get_converted_schema()
    if columns_metadata:
        schema = normalize_fixed_column_types(schema, columns_metadata)
    return schema.empty_table()
