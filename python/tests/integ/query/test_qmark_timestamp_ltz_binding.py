"""Reference-driver qmark datetime binding into TIMESTAMP_NTZ / TIMESTAMP_LTZ.

Locks the snowflake-connector-python contract from ``test/integ/test_bindings.py``:

* A bare ``datetime`` is always tagged ``TIMESTAMP_NTZ``. Tz-aware values are
  converted to UTC, then stored as a naive wall-clock.
* Absolute-instant round-trip into ``TIMESTAMP_LTZ`` requires the explicit
  ``("TIMESTAMP_LTZ", dt)`` type tuple.
* A bare tz-aware bind into ``TIMESTAMP_LTZ`` (or ``?::TIMESTAMP_LTZ``) is an
  NTZ value reinterpreted in the session timezone — not a preservation of the
  original instant.

Run against both the reference connector and the universal driver.
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest
import pytz


SESSION_TZ = "America/New_York"
# January is EST (UTC-5) / PST (UTC-8); use fixed offsets so the expected
# values do not depend on IANA DST tables at bind time.
PST = timezone(timedelta(hours=-8))
# 2024-01-15 02:30:00-08:00 == 2024-01-15 10:30:00 UTC
AWARE_PST = datetime(2024, 1, 15, 2, 30, 0, tzinfo=PST)
NAIVE_WALL = datetime(2024, 1, 15, 2, 30, 0)
# UTC-strip of AWARE_PST when bound as TIMESTAMP_NTZ
NTZ_UTC_WALL = datetime(2024, 1, 15, 10, 30, 0)
# NTZ 10:30 wall-clock reinterpreted as America/New_York (EST, UTC-5)
SESSION_LOCAL_REINTERPRET = pytz.timezone(SESSION_TZ).localize(NTZ_UTC_WALL)


def _epoch(dt: datetime) -> float:
    """Match reference ``convert_datetime_to_epoch``: naive = UTC wall-clock."""
    if dt.tzinfo is None:
        return (dt - datetime(1970, 1, 1)).total_seconds()
    return dt.timestamp()


@pytest.fixture
def connection(connection_factory):
    with connection_factory(
        paramstyle="qmark",
        session_parameters={"TIMEZONE": SESSION_TZ},
    ) as conn:
        yield conn


class TestQmarkBareDatetimeBindsAsNtz:
    def test_tz_aware_select_ntz_stores_utc_wall_clock(self, cursor):
        cursor.execute("SELECT ?::TIMESTAMP_NTZ", (AWARE_PST,))
        (got,) = cursor.fetchone()
        assert got.tzinfo is None
        assert got == NTZ_UTC_WALL

    def test_naive_select_ntz_preserves_wall_clock(self, cursor):
        cursor.execute("SELECT ?::TIMESTAMP_NTZ", (NAIVE_WALL,))
        (got,) = cursor.fetchone()
        assert got.tzinfo is None
        assert got == NAIVE_WALL

    def test_tz_aware_insert_ntz_stores_utc_wall_clock(self, cursor, tmp_schema):
        table = f"{tmp_schema}.ntz_bare"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (col TIMESTAMP_NTZ)")
        cursor.execute(f"INSERT INTO {table} VALUES (?)", (AWARE_PST,))
        cursor.execute(f"SELECT col FROM {table}")
        (got,) = cursor.fetchone()
        assert got == NTZ_UTC_WALL
        assert _epoch(got) == _epoch(AWARE_PST)


class TestQmarkExplicitLtzTuplePreservesInstant:
    def test_tz_aware_select_preserves_instant(self, cursor):
        cursor.execute("SELECT ?::TIMESTAMP_LTZ", (("TIMESTAMP_LTZ", AWARE_PST),))
        (got,) = cursor.fetchone()
        assert got.tzinfo is not None
        assert _epoch(got) == pytest.approx(_epoch(AWARE_PST))

    def test_tz_aware_insert_preserves_instant(self, cursor, tmp_schema):
        table = f"{tmp_schema}.ltz_tuple"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (col TIMESTAMP_LTZ)")
        cursor.execute(f"INSERT INTO {table} VALUES (?)", (("TIMESTAMP_LTZ", AWARE_PST),))
        cursor.execute(f"SELECT col FROM {table}")
        (got,) = cursor.fetchone()
        assert got.tzinfo is not None
        assert _epoch(got) == pytest.approx(_epoch(AWARE_PST))
        assert got.astimezone(timezone.utc) == AWARE_PST.astimezone(timezone.utc)

    def test_naive_insert_treats_wall_clock_as_utc_instant(self, cursor, tmp_schema):
        """Naive TIMESTAMP_LTZ binds are localized from UTC, not session wall-clock.

        Reference ``_derive_offset_timestamp`` does ``pytz.utc.localize(naive)``
        then converts to the session TZ only to derive the offset; the epoch
        sent on the wire is still the naive wall-clock interpreted as UTC.
        """
        table = f"{tmp_schema}.ltz_naive_tuple"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (col TIMESTAMP_LTZ)")
        cursor.execute(f"INSERT INTO {table} VALUES (?)", (("TIMESTAMP_LTZ", NAIVE_WALL),))
        cursor.execute(f"SELECT col FROM {table}")
        (got,) = cursor.fetchone()
        expected_utc = NAIVE_WALL.replace(tzinfo=timezone.utc)
        assert _epoch(got) == pytest.approx(_epoch(expected_utc))


class TestQmarkBareTzAwareIntoLtzIsSessionLocalNtzCast:
    """BD#87: bare qmark datetime is NTZ; LTZ cast uses session-local wall-clock."""

    def test_select_cast_does_not_preserve_original_instant(self, cursor):
        cursor.execute("SELECT ?::TIMESTAMP_LTZ", (AWARE_PST,))
        (got,) = cursor.fetchone()
        assert got.tzinfo is not None
        assert _epoch(got) == pytest.approx(_epoch(SESSION_LOCAL_REINTERPRET))
        assert _epoch(got) != pytest.approx(_epoch(AWARE_PST))

    def test_insert_does_not_preserve_original_instant(self, cursor, tmp_schema):
        table = f"{tmp_schema}.ltz_bare"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (col TIMESTAMP_LTZ)")
        cursor.execute(f"INSERT INTO {table} VALUES (?)", (AWARE_PST,))
        cursor.execute(f"SELECT col FROM {table}")
        (got,) = cursor.fetchone()
        assert got.tzinfo is not None
        assert _epoch(got) == pytest.approx(_epoch(SESSION_LOCAL_REINTERPRET))
        assert _epoch(got) != pytest.approx(_epoch(AWARE_PST))
