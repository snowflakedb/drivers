"""
An out-of-range `scale` value parsed from Arrow IPC column metadata was used
directly to index the fixed-size `powTenSB4` array in
`ArrowTableConverter.cpp` for TIME/TIMESTAMP_NTZ/TIMESTAMP_LTZ/TIMESTAMP_TZ
columns - an out-of-bounds array read. These tests assert that an
out-of-range scale is rejected before it reaches any converter, and that all
valid scales (0-9) still work.
"""

from __future__ import annotations

import ctypes

import pytest

from snowflake.connector._internal.arrow_context import ArrowConverterContext
from snowflake.connector._internal.arrow_stream_utils import create_table_iterator
from snowflake.connector.errors import InterfaceError


pa = pytest.importorskip("pyarrow")

_INVALID_SCALES = [-1, 10, 100, -100]
_VALID_SCALES = list(range(10))

_TZ_STRUCT_TYPE = pa.struct([pa.field("epoch", pa.int64()), pa.field("timezone", pa.int32())])


class _ArrowArrayStream(ctypes.Structure):
    """Mirrors the Arrow C Data Interface `ArrowArrayStream` struct layout."""

    _fields_ = [
        ("get_schema", ctypes.c_void_p),
        ("get_next", ctypes.c_void_p),
        ("get_last_error", ctypes.c_void_p),
        ("release", ctypes.c_void_p),
        ("private_data", ctypes.c_void_p),
    ]


def _iterate(arrow_type, metadata, value):
    field = pa.field("col", arrow_type, metadata=metadata)
    schema = pa.schema([field])
    batch = pa.record_batch([pa.array([value], type=arrow_type)], schema=schema)
    reader = pa.RecordBatchReader.from_batches(schema, [batch])

    stream = _ArrowArrayStream()
    stream_ptr = ctypes.addressof(stream)
    reader._export_to_c(stream_ptr)

    iterator = create_table_iterator(stream_ptr, context=ArrowConverterContext())
    return list(iterator)


@pytest.mark.parametrize("scale", _INVALID_SCALES)
def test_time_invalid_scale_raises(scale):
    with pytest.raises(InterfaceError, match="invalid scale value"):
        _iterate(pa.int64(), {"logicalType": "TIME", "scale": str(scale)}, 0)


@pytest.mark.parametrize("scale", _INVALID_SCALES)
def test_timestamp_ntz_invalid_scale_raises(scale):
    with pytest.raises(InterfaceError, match="invalid scale value"):
        _iterate(pa.int64(), {"logicalType": "TIMESTAMP_NTZ", "scale": str(scale)}, 0)


@pytest.mark.parametrize("scale", _INVALID_SCALES)
def test_timestamp_ltz_invalid_scale_raises(scale):
    with pytest.raises(InterfaceError, match="invalid scale value"):
        _iterate(pa.int64(), {"logicalType": "TIMESTAMP_LTZ", "scale": str(scale)}, 0)


@pytest.mark.parametrize("scale", _INVALID_SCALES)
def test_timestamp_tz_invalid_scale_raises(scale):
    with pytest.raises(InterfaceError, match="invalid scale value"):
        _iterate(
            _TZ_STRUCT_TYPE,
            {"logicalType": "TIMESTAMP_TZ", "scale": str(scale), "byteLength": "8"},
            {"epoch": 0, "timezone": 1440},
        )


@pytest.mark.parametrize("scale", _VALID_SCALES)
def test_time_valid_scale_accepted(scale):
    _iterate(pa.int64(), {"logicalType": "TIME", "scale": str(scale)}, 0)


@pytest.mark.parametrize("scale", _VALID_SCALES)
def test_timestamp_ntz_valid_scale_accepted(scale):
    _iterate(pa.int64(), {"logicalType": "TIMESTAMP_NTZ", "scale": str(scale)}, 0)


@pytest.mark.parametrize("scale", _VALID_SCALES)
def test_timestamp_ltz_valid_scale_accepted(scale):
    _iterate(pa.int64(), {"logicalType": "TIMESTAMP_LTZ", "scale": str(scale)}, 0)


@pytest.mark.parametrize("scale", _VALID_SCALES)
def test_timestamp_tz_valid_scale_accepted(scale):
    _iterate(
        _TZ_STRUCT_TYPE,
        {"logicalType": "TIMESTAMP_TZ", "scale": str(scale), "byteLength": "8"},
        {"epoch": 0, "timezone": 1440},
    )
