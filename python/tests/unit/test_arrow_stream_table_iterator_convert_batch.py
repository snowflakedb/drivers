"""
Exercise ``CArrowStreamTableIterator::convertBatch``'s GIL-released
per-column conversion loop through the real Python API.

These cases cover the happy path plus error paths that call
``py::setPyError`` / ``SF_CHECK_ARROW_RC`` while the GIL is released, so a
missing GIL reacquire would typically crash (segfault) or hang rather than
raise a clean ``InterfaceError``.
"""

from __future__ import annotations

import ctypes
import threading

import pytest

from snowflake.connector._internal.arrow_context import ArrowConverterContext
from snowflake.connector._internal.arrow_stream_utils import create_table_iterator
from snowflake.connector.errors import InterfaceError


pa = pytest.importorskip("pyarrow")


class _ArrowArrayStream(ctypes.Structure):
    """Mirrors the Arrow C Data Interface ``ArrowArrayStream`` struct layout."""

    _fields_ = [
        ("get_schema", ctypes.c_void_p),
        ("get_next", ctypes.c_void_p),
        ("get_last_error", ctypes.c_void_p),
        ("release", ctypes.c_void_p),
        ("private_data", ctypes.c_void_p),
    ]


def _iterate_batches(columns: list[tuple[pa.Field, pa.Array]]) -> list[pa.RecordBatch]:
    """Export a one-batch stream and drain ``ArrowStreamTableIterator``."""
    fields = [field for field, _ in columns]
    arrays = [array for _, array in columns]
    schema = pa.schema(fields)
    batch = pa.record_batch(arrays, schema=schema)
    reader = pa.RecordBatchReader.from_batches(schema, [batch])

    stream = _ArrowArrayStream()
    stream_ptr = ctypes.addressof(stream)
    reader._export_to_c(stream_ptr)

    iterator = create_table_iterator(stream_ptr, context=ArrowConverterContext())
    return list(iterator)


def _scaled_fixed_column(name: str, values: list[int], scale: int) -> tuple[pa.Field, pa.Array]:
    field = pa.field(name, pa.int64(), metadata={"logicalType": "FIXED", "scale": str(scale)})
    return field, pa.array(values, type=pa.int64())


def _bool_column(name: str, values: list[bool]) -> tuple[pa.Field, pa.Array]:
    field = pa.field(name, pa.bool_(), metadata={"logicalType": "BOOLEAN"})
    return field, pa.array(values, type=pa.bool_())


def _text_column(name: str, values: list[str]) -> tuple[pa.Field, pa.Array]:
    field = pa.field(name, pa.string(), metadata={"logicalType": "TEXT"})
    return field, pa.array(values, type=pa.string())


def _time_column(name: str, values: list[int], scale: int) -> tuple[pa.Field, pa.Array]:
    field = pa.field(name, pa.int64(), metadata={"logicalType": "TIME", "scale": str(scale)})
    return field, pa.array(values, type=pa.int64())


def test_should_convert_multi_column_batch_under_gil_released_loop():
    # Given a multi-column batch that exercises scaled FIXED conversion plus
    # pass-through BOOLEAN/TEXT columns
    columns = [
        _scaled_fixed_column("num", [1234, 5678], scale=2),
        _bool_column("flag", [True, False]),
        _text_column("label", ["a", "b"]),
    ]

    # When convertBatch runs with the GIL released around the column loop
    batches = _iterate_batches(columns)

    # Then conversion completes without error and preserves row count / names
    assert len(batches) == 1
    assert batches[0].num_rows == 2
    assert batches[0].schema.names == ["num", "flag", "label"]


def test_should_raise_on_invalid_scale_after_successful_columns():
    # Given earlier columns convert successfully and a later TIME column has
    # an out-of-range scale (setPyError under GIL release; checked after loop)
    columns = [
        _bool_column("ok_before", [True]),
        _text_column("also_ok", ["x"]),
        _time_column("bad_time", [0], scale=10),
    ]

    # When the batch is converted
    # Then InterfaceError is raised (no segfault / hang) with the scale message
    with pytest.raises(InterfaceError, match="invalid scale value"):
        _iterate_batches(columns)


def test_should_raise_on_invalid_scale_before_remaining_columns():
    # Given the first column fails scale validation and later columns would
    # otherwise convert successfully — error is still surfaced after the loop
    columns = [
        _time_column("bad_time", [0], scale=-1),
        _bool_column("ok_after", [False]),
        _text_column("also_after", ["y"]),
    ]

    # When the batch is converted
    # Then InterfaceError is raised once the GIL is held again
    with pytest.raises(InterfaceError, match="invalid scale value"):
        _iterate_batches(columns)


@pytest.mark.parametrize(
    "logical_type",
    ["TIME", "TIMESTAMP_NTZ", "TIMESTAMP_LTZ"],
)
def test_should_raise_on_invalid_scale_for_timestamp_family(logical_type: str):
    # Given a single column whose Snowflake logical type uses scale metadata
    field = pa.field(
        "col",
        pa.int64(),
        metadata={"logicalType": logical_type, "scale": "100"},
    )
    columns = [(field, pa.array([0], type=pa.int64()))]

    # When convertBatch hits the invalid-scale setPyError path
    # Then a clean InterfaceError is raised
    with pytest.raises(InterfaceError, match="invalid scale value"):
        _iterate_batches(columns)


def test_should_raise_on_mismatched_array_physical_type():
    # Given ARRAY logicalType paired with a non-list/non-string physical type
    # (the malformed-schema path exercised in the PR smoke test)
    field = pa.field("arr", pa.int64(), metadata={"logicalType": "ARRAY"})
    columns = [(field, pa.array([1], type=pa.int64()))]

    # When convertBatch validates the schema under the GIL-released loop
    # Then InterfaceError is raised for the unknown ARRAY physical type
    with pytest.raises(InterfaceError, match="unknown arrow type.*ARRAY"):
        _iterate_batches(columns)


def test_should_survive_convert_errors_under_concurrent_python_threads():
    """
    Hammer Python/C API from other threads while convertBatch error paths
    reacquire the GIL via setPyError. A missing GIL guard typically segfaults
    under this kind of contention rather than raising InterfaceError.
    """
    stop = threading.Event()
    worker_errors: list[BaseException] = []

    def _python_work() -> None:
        try:
            while not stop.is_set():
                # Cheap allocations / dict ops that require the GIL
                payload = {str(i): i for i in range(32)}
                _ = sum(payload.values())
        except BaseException as exc:
            worker_errors.append(exc)

    workers = [threading.Thread(target=_python_work, daemon=True) for _ in range(4)]
    for worker in workers:
        worker.start()

    try:
        for _ in range(40):
            with pytest.raises(InterfaceError, match="invalid scale value"):
                _iterate_batches(
                    [
                        _bool_column("ok", [True]),
                        _time_column("bad_time", [0], scale=10),
                        _text_column("unused", ["z"]),
                    ]
                )
            with pytest.raises(InterfaceError, match="unknown arrow type.*ARRAY"):
                _iterate_batches(
                    [
                        (
                            pa.field("arr", pa.int64(), metadata={"logicalType": "ARRAY"}),
                            pa.array([1], type=pa.int64()),
                        )
                    ]
                )
    finally:
        stop.set()
        for worker in workers:
            worker.join(timeout=5)

    assert worker_errors == []
