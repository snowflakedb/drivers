from __future__ import annotations

from collections.abc import Sequence
from typing import TYPE_CHECKING, cast

from snowflake.connector._core import sf_core_python
from snowflake.connector.errors import InternalError, NotSupportedError

from .._common.extras import pyarrow
from .arrow import ArrowRowIterator
from .arrow_context import ArrowConverterContext
from .arrow_stream_iterator import ArrowStreamIterator as CythonArrowStreamIterator
from .arrow_stream_iterator import ArrowStreamTableIterator
from .type_codes import FIXED


if TYPE_CHECKING:
    from pyarrow import Schema, Table

    from .cursor.result_metadata import ResultMetadata


def release_arrow_stream(stream_ptr: int | None) -> None:
    """Release an ArrowArrayStream pointer to prevent memory leaks.

    Works by creating a throwaway ArrowStreamIterator whose C++ destructor
    calls ``stream->release(stream)`` when it goes out of scope.  This is
    indirect — the C++ from_stream() factory also reads the schema — but it
    is the only release path currently exposed to Python.

    If the stream is in a bad state (already released, corrupt), the
    ArrowStreamIterator constructor will fail.  We catch that to avoid
    propagating errors into callers that are doing best-effort cleanup
    (e.g. from_prepare_result cleanup).
    """
    if not stream_ptr:
        return
    try:
        _ = CythonArrowStreamIterator(stream_ptr, ArrowConverterContext())
    except Exception:
        pass


def create_row_iterator(
    stream_ptr: int,
    *,
    context: ArrowConverterContext,
    use_dict_result: bool = False,
    use_numpy: bool = False,
) -> ArrowRowIterator:
    """Build a sync row iterator that yields one row at a time.

    When ``sf_core_python`` is built with the ``native-arrow`` feature, returns
    the PyO3 ``ArrowStreamIterator`` directly. Otherwise uses the Cython
    nanoarrow iterator.
    """
    if sf_core_python.native_arrow_enabled():
        if use_numpy or use_dict_result:
            release_arrow_stream(stream_ptr)
            raise NotSupportedError(
                msg=(
                    "Native Arrow row path does not support use_numpy / use_dict_result. "
                    "Disable SF_NATIVE_ARROW or drop use_numpy/use_dict_result."
                )
            )
        # Class is only exported when built with ``native-arrow``; stub_gen
        # emits ``#[pyfunction]``s only, so do not attribute-access it on the stub.
        iterator_cls = getattr(sf_core_python, "ArrowStreamIterator", None)
        if iterator_cls is None:
            release_arrow_stream(stream_ptr)
            raise InternalError(
                msg=(
                    "sf_core_python.ArrowStreamIterator is unavailable; "
                    "rebuild with --features native-arrow / SF_NATIVE_ARROW=1"
                )
            )
        return cast(
            ArrowRowIterator,
            iterator_cls(stream_ptr, session_timezone=context.timezone),
        )
    return CythonArrowStreamIterator(
        stream_ptr,
        context,
        use_dict_result=use_dict_result,
        use_numpy=use_numpy,
    )


def create_table_iterator(
    stream_ptr: int,
    *,
    context: ArrowConverterContext,
    number_to_decimal: bool = False,
    force_microsecond_precision: bool = False,
) -> ArrowStreamTableIterator:
    """Build an :class:`ArrowStreamTableIterator` that yields one RecordBatch at a time."""
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
    for field, metadata in zip(schema, description, strict=False):
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
