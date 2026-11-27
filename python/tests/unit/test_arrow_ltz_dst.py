"""Offline LTZ regressions for whole-epoch Arrow microseconds and DST offsets."""

from __future__ import annotations

import ctypes
import sys

from datetime import UTC, datetime, timedelta
from zoneinfo import ZoneInfo

import pytest
import pytz

from snowflake.connector._internal.arrow_context import ArrowConverterContext
from snowflake.connector.errors import InterfaceError


_EPOCH = datetime(1970, 1, 1, tzinfo=UTC)
_ZONES = ("UTC", "America/New_York", "Europe/London", "Australia/Sydney")
_PRE_EPOCH = "1969-12-31T23:59:59.999999+00:00"
_INSTANTS = {
    "UTC": ("2024-05-01T12:00:00.123456+00:00", _PRE_EPOCH),
    "America/New_York": (
        "2024-05-01T12:00:00.123456+00:00",
        "2024-03-10T06:59:59.999999+00:00",
        "2024-03-10T07:00:00.000001+00:00",
        "2024-11-03T05:30:00.123456+00:00",
        "2024-11-03T06:30:00.123456+00:00",
        _PRE_EPOCH,
    ),
    "Europe/London": (
        "2024-01-15T12:00:00.123456+00:00",
        "2024-03-31T00:59:59.999999+00:00",
        "2024-03-31T01:00:00.000001+00:00",
        "2024-10-27T00:30:00.123456+00:00",
        "2024-10-27T01:30:00.123456+00:00",
        _PRE_EPOCH,
    ),
    "Australia/Sydney": (
        "2024-01-15T12:00:00.123456+00:00",
        "2024-10-05T15:59:59.999999+00:00",
        "2024-10-05T16:00:00.000001+00:00",
        "2024-04-06T15:30:00.123456+00:00",
        "2024-04-06T16:30:00.123456+00:00",
        _PRE_EPOCH,
    ),
}
_TRANSITIONS = (
    ("UTC", "2024-03-10T07:00:00+00:00"),
    ("America/New_York", "2024-03-10T07:00:00+00:00"),
    ("Europe/London", "2024-03-31T01:00:00+00:00"),
    ("Australia/Sydney", "2024-10-05T16:00:00+00:00"),
    ("America/New_York", "2024-11-03T06:00:00+00:00"),
    ("Europe/London", "2024-10-27T01:00:00+00:00"),
    ("Australia/Sydney", "2024-04-06T16:00:00+00:00"),
)


def _epoch_microseconds(instant: datetime) -> int:
    delta = instant - _EPOCH
    return (delta.days * 86400 + delta.seconds) * 1_000_000 + delta.microseconds


def _expected_local(instant: datetime, zone: str) -> datetime:
    # Convert an independently constructed UTC instant, never an epoch-local wall clock.
    return instant.astimezone(pytz.timezone(zone))


def _assert_local(actual: datetime, expected: datetime) -> None:
    assert isinstance(actual, datetime)
    assert actual.replace(tzinfo=None) == expected.replace(tzinfo=None)
    assert actual.utcoffset() == expected.utcoffset()
    assert actual.isoformat() == expected.isoformat()


@pytest.mark.parametrize(
    "zone,utc_text",
    [pytest.param(zone, text, id=f"{zone}-{text}") for zone, instants in _INSTANTS.items() for text in instants],
)
def test_whole_epoch_microseconds_use_offset_at_the_result_instant(zone: str, utc_text: str):
    instant = datetime.fromisoformat(utc_text)
    if sys.platform == "win32" and instant < _EPOCH:
        pytest.skip("The non-Windows helper may require a negative timestamp on this case")
    context = ArrowConverterContext(timezone=zone)

    actual = context.TIMESTAMP_LTZ_to_python(0, _epoch_microseconds(instant))

    _assert_local(actual, _expected_local(instant, zone))


@pytest.mark.parametrize(
    "zone,utc_text,expected_iso",
    [
        ("America/New_York", "2024-05-01T12:00:00.123456+00:00", "2024-05-01T08:00:00.123456-04:00"),
        ("Australia/Sydney", "2024-01-15T12:00:00.123456+00:00", "2024-01-15T23:00:00.123456+11:00"),
    ],
)
def test_whole_epoch_microseconds_have_explicit_summer_wall_time_anchors(zone, utc_text, expected_iso):
    context = ArrowConverterContext(timezone=zone)
    actual = context.TIMESTAMP_LTZ_to_python(0, _epoch_microseconds(datetime.fromisoformat(utc_text)))
    expected = datetime.fromisoformat(expected_iso)

    _assert_local(actual, expected)


@pytest.mark.parametrize("zone,transition", _TRANSITIONS)
def test_fraction_carry_and_borrow_resolve_dst_before_localizing(zone: str, transition: str):
    context = ArrowConverterContext(timezone=zone)
    transition_seconds = _epoch_microseconds(datetime.fromisoformat(transition)) // 1_000_000
    for second_offset, microseconds in (
        (-1, 1_000_000),
        (-1, 1_000_001),
        (-1, 1_999_999),
        (0, -1),
        (1, -1_000_000),
        (1, -1_000_001),
    ):
        epoch = transition_seconds + second_offset
        expected = _expected_local(_EPOCH + timedelta(seconds=epoch, microseconds=microseconds), zone)

        actual = context.TIMESTAMP_LTZ_to_python(epoch, microseconds)

        _assert_local(actual, expected)


@pytest.mark.parametrize("encoding", ["whole-epoch", "negative-carry"])
@pytest.mark.parametrize(
    "utc_text,expected_iso",
    [
        ("2024-11-03T06:30:00+00:00", "2024-11-03T01:30:00-05:00"),
        ("2024-11-03T06:30:00.123456+00:00", "2024-11-03T01:30:00.123456-05:00"),
    ],
)
def test_unnormalized_zoneinfo_microseconds_preserve_fall_back_fold(monkeypatch, encoding, utc_text, expected_iso):
    zone = ZoneInfo("America/New_York")
    instant = datetime.fromisoformat(utc_text)
    context = ArrowConverterContext(timezone="America/New_York")
    monkeypatch.setattr(context, "_get_session_tz", lambda: zone)
    if encoding == "whole-epoch":
        epoch, microseconds = 0, _epoch_microseconds(instant)
    else:
        next_second = instant.replace(microsecond=0) + timedelta(seconds=1)
        epoch = _epoch_microseconds(next_second) // 1_000_000
        microseconds = instant.microsecond - 1_000_000

    actual = context.TIMESTAMP_LTZ_to_python(epoch, microseconds)

    _assert_local(actual, instant.astimezone(zone))
    assert actual.isoformat() == expected_iso
    assert actual.fold == 1
    assert actual.astimezone(UTC) == instant


@pytest.mark.parametrize("zone,transition", _TRANSITIONS)
def test_normalized_fraction_control_on_each_side_of_dst(zone: str, transition: str):
    context = ArrowConverterContext(timezone=zone)
    transition_seconds = _epoch_microseconds(datetime.fromisoformat(transition)) // 1_000_000
    for second_offset in (-1, 0):
        for microseconds in (0, 1, 999_999):
            epoch = transition_seconds + second_offset
            expected = _expected_local(_EPOCH + timedelta(seconds=epoch, microseconds=microseconds), zone)

            actual = context.TIMESTAMP_LTZ_to_python(epoch, microseconds)

            _assert_local(actual, expected)


@pytest.mark.parametrize("zone", _ZONES)
def test_windows_helper_keeps_whole_and_split_epoch_conversion(zone: str):
    context = ArrowConverterContext(timezone=zone)
    for utc_text in _INSTANTS[zone]:
        instant = datetime.fromisoformat(utc_text)
        total = _epoch_microseconds(instant)
        expected = _expected_local(instant, zone)

        _assert_local(context.TIMESTAMP_LTZ_to_python_windows(0, total), expected)
        _assert_local(context.TIMESTAMP_LTZ_to_python_windows(total // 1_000_000, instant.microsecond), expected)


@pytest.mark.parametrize("timezone", [pytz.UTC, UTC], ids=["pytz-utc", "datetime-utc"])
@pytest.mark.parametrize("instant", [datetime.min.replace(tzinfo=UTC), datetime.max.replace(tzinfo=UTC)])
@pytest.mark.parametrize("method", ["TIMESTAMP_LTZ_to_python", "TIMESTAMP_LTZ_to_python_windows"])
def test_utc_whole_epoch_microseconds_preserve_datetime_extrema(monkeypatch, timezone, instant, method):
    context = ArrowConverterContext(timezone="UTC")
    monkeypatch.setattr(context, "_get_session_tz", lambda: timezone)

    actual = getattr(context, method)(0, _epoch_microseconds(instant))

    _assert_local(actual, instant)


class _ArrowArrayStream(ctypes.Structure):
    """Arrow C Data Interface stream layout, as used by the existing scale tests."""

    _fields_ = [
        ("get_schema", ctypes.c_void_p),
        ("get_next", ctypes.c_void_p),
        ("get_last_error", ctypes.c_void_p),
        ("release", ctypes.c_void_p),
        ("private_data", ctypes.c_void_p),
    ]


def _read_ltz_rows(pa, zone, arrow_type, values, scale):
    from snowflake.connector._internal.arrow_stream_utils import create_row_iterator

    field = pa.field("col", arrow_type, metadata={"logicalType": "TIMESTAMP_LTZ", "scale": str(scale)})
    schema = pa.schema([field])
    batch = pa.record_batch([pa.array(values, type=arrow_type)], schema=schema)
    reader = pa.RecordBatchReader.from_batches(schema, [batch])
    stream = _ArrowArrayStream()
    reader._export_to_c(ctypes.addressof(stream))
    iterator = create_row_iterator(
        ctypes.addressof(stream),
        context=ArrowConverterContext(timezone=zone),
        use_core_arrow=False,
    )
    try:
        return list(iterator)
    finally:
        del iterator


def _assert_ltz_rows(rows, instants, zone):
    assert len(rows) == len(instants)
    for row, instant in zip(rows, instants, strict=True):
        assert isinstance(row, tuple)
        assert len(row) == 1
        if instant is None:
            assert row[0] is None
        else:
            _assert_local(row[0], _expected_local(instant, zone))


@pytest.mark.parametrize("zone", _ZONES)
@pytest.mark.parametrize("scale", [6, 7])
def test_native_integer_ltz_scales6_and7_preserve_local_wall_time_and_nulls(zone: str, scale: int):
    pa = pytest.importorskip("pyarrow")
    instants = [datetime.fromisoformat(text) for text in _INSTANTS[zone]]
    multiplier = 10 ** (scale - 6)
    values = [_epoch_microseconds(instant) * multiplier + multiplier - 1 for instant in instants]
    values.insert(1, None)
    instants.insert(1, None)

    rows = _read_ltz_rows(pa, zone, pa.int64(), values, scale)

    _assert_ltz_rows(rows, instants, zone)


@pytest.mark.parametrize("zone", ["UTC", "America/New_York"])
@pytest.mark.parametrize("value", [-(1 << 63), (1 << 63) - 1], ids=["int64-min", "int64-max"])
def test_native_out_of_range_ltz_preserves_conversion_error(zone: str, value: int):
    pa = pytest.importorskip("pyarrow")

    with pytest.raises(InterfaceError) as error:
        _read_ltz_rows(pa, zone, pa.int64(), [value], 6)

    assert error.value.errno == 252005


@pytest.mark.parametrize("zone", _ZONES)
@pytest.mark.parametrize("scale", [8, 9])
def test_native_struct_ltz_normalized_fraction_controls(zone: str, scale: int):
    pa = pytest.importorskip("pyarrow")
    instants = [datetime.fromisoformat(text) for text in _INSTANTS[zone]]
    arrow_type = pa.struct([pa.field("epoch", pa.int64()), pa.field("fraction", pa.int32())])
    values = [
        {
            "epoch": _epoch_microseconds(instant) // 1_000_000,
            "fraction": instant.microsecond * 1000 + (990 if scale == 8 else 999),
        }
        for instant in instants
    ]
    values.insert(1, None)
    instants.insert(1, None)

    rows = _read_ltz_rows(pa, zone, arrow_type, values, scale)

    _assert_ltz_rows(rows, instants, zone)


@pytest.mark.parametrize("zone", _ZONES)
def test_synthetic_integer_scale9_floors_nanoseconds_before_localizing(zone: str):
    """Control supported integer decoding, not a claim about server scale-9 encoding."""
    pa = pytest.importorskip("pyarrow")
    summer = _epoch_microseconds(datetime.fromisoformat("2024-05-01T12:00:00.123456+00:00"))
    values = [0, 1, -1, 999, -999, 1001, -1001, summer * 1000 + 999, None]
    instants = [None if value is None else _EPOCH + timedelta(microseconds=value // 1000) for value in values]

    rows = _read_ltz_rows(pa, zone, pa.int64(), values, 9)

    _assert_ltz_rows(rows, instants, zone)
