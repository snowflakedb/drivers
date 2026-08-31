"""Large (stage-based) parameter binding tests for the Python wrapper."""

from __future__ import annotations

from datetime import date

import pytest

from tests.compatibility import IS_UNIVERSAL_DRIVER

from ...conftest import with_paramstyles


CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD"
DEFAULT_STAGE_ARRAY_BINDING_THRESHOLD = 65280


# Universal driver uploads via sf_core to @SYSTEM$BIND; the reference connector
# uses BindUploadAgent with a temporary SYSTEMBIND stage (see snowflake-connector-python).
_BIND_STAGE_LIST_SQL = "LIST @SYSTEM$BIND" if IS_UNIVERSAL_DRIVER else "LIST @SYSTEMBIND"


def set_stage_binding_threshold(connection, value: int) -> None:
    """Set CLIENT_STAGE_ARRAY_BINDING_THRESHOLD for stage-binding decisions."""
    param = CLIENT_STAGE_ARRAY_BINDING_THRESHOLD
    with connection.cursor() as session_cursor:
        session_cursor.execute(f"ALTER SESSION SET {param} = {value}")
    if not IS_UNIVERSAL_DRIVER:
        # Reference connector reads the threshold from _session_parameters at executemany time.
        connection._session_parameters[param] = value


def list_bind_stage_file_count(connection) -> int | None:
    """Return file count in the bind upload stage, or None if the stage does not exist."""
    with connection.cursor() as list_cursor:
        try:
            list_cursor.execute(_BIND_STAGE_LIST_SQL)
        except Exception:
            return None
        return len(list_cursor.fetchall())


def assert_bind_stage_file_count_increased(
    connection,
    before: int | None,
    after: int | None,
) -> None:
    """Assert the bind stage received a new upload."""
    assert after is not None
    if IS_UNIVERSAL_DRIVER:
        assert before is None or after > before
    else:
        # Reference connector recreates SYSTEMBIND via CREATE OR REPLACE; file count
        # may not grow even when a new bind file was uploaded.
        assert after > 0


def assert_bind_stage_reused_across_uploads(
    connection,
    after_first: int | None,
    after_second: int | None,
) -> None:
    """Assert the second bulk insert also used stage binding."""
    assert after_second is not None
    if IS_UNIVERSAL_DRIVER:
        assert after_first is not None
        assert after_second > after_first
    else:
        assert after_second > 0


def assert_bind_stage_file_count_unchanged(
    connection,
    before: int | None,
    after: int | None,
) -> None:
    """Assert no new bind file was uploaded."""
    assert after == before


def _insert_placeholders(cursor, column_count: int) -> str:
    connection = cursor.connection
    if IS_UNIVERSAL_DRIVER:
        return connection.paramstyle.placeholders(column_count)
    # Reference connector stores paramstyle as a string on _paramstyle (set via connect()).
    if connection._paramstyle == "numeric":
        return ", ".join(f":{i}" for i in range(1, column_count + 1))
    return ", ".join("?" for _ in range(column_count))


def _update_placeholders(cursor) -> tuple[str, str]:
    """Return (set_placeholder, where_placeholder) for a two-parameter UPDATE."""
    connection = cursor.connection
    if IS_UNIVERSAL_DRIVER:
        if str(connection.paramstyle) == "numeric":
            return ":1", ":2"
        return "?", "?"
    if connection._paramstyle == "numeric":
        return ":1", ":2"
    return "?", "?"


def bulk_insert_id_name(cursor, table: str, count: int, id_offset: int, name_prefix: str) -> None:
    ids = [id_offset + i for i in range(count)]
    names = [f"{name_prefix}{i}" for i in range(count)]
    placeholders = _insert_placeholders(cursor, 2)
    cursor.executemany(f"INSERT INTO {table} VALUES ({placeholders})", list(zip(ids, names, strict=True)))


def bulk_insert_types(cursor, table: str, count: int) -> None:
    rows = []
    for i in range(count):
        n = None if i % 7 == 0 else i * 10
        rows.append((i, n, i * 0.5, i % 2 == 0, f"txt-{i}"))
    placeholders = _insert_placeholders(cursor, 5)
    cursor.executemany(f"INSERT INTO {table} VALUES ({placeholders})", rows)


def hazard_string(i: int) -> str | None:
    match i % 7:
        case 0:
            return f"val,{i}"
        case 1:
            return f'say"{i}"'
        case 2:
            return "a\nb"
        case 3:
            return f"C:\\dir\\{i}"
        case 4:
            return ""
        case 5:
            return None
        case 6:
            return "日本語"
    return ""


def bulk_insert_hazard_strings(cursor, table: str, count: int) -> None:
    rows = [(i, hazard_string(i)) for i in range(count)]
    placeholders = _insert_placeholders(cursor, 2)
    cursor.executemany(f"INSERT INTO {table} VALUES ({placeholders})", rows)


def bulk_insert_all_null_row(cursor, table: str) -> None:
    placeholders = _insert_placeholders(cursor, 6)
    cursor.executemany(f"INSERT INTO {table} VALUES ({placeholders})", [(None,) * 6])


@with_paramstyles("qmark", "numeric")
class TestLargeBindings:
    @pytest.fixture(autouse=True)
    def _restore_stage_binding_threshold(self, cursor):
        yield
        set_stage_binding_threshold(cursor.connection, DEFAULT_STAGE_ARRAY_BINDING_THRESHOLD)

    def test_should_stage_bind_at_the_default_threshold_and_reuse_system_bind_across_consecutive_bulk_inserts(
        self, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table = f"{tmp_schema}.lb_threshold_reuse"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (id NUMBER, name VARCHAR)")

        # When 33000 rows generated as [[i, "first-" + i] for i in 0..33000] are inserted using multirow binding
        before1 = list_bind_stage_file_count(cursor.connection)
        bulk_insert_id_name(cursor, table, 33000, 0, "first-")

        # Then the bind file on SYSTEM$BIND from the last bulk insert should contain
        # the same values as the bound parameters
        after1 = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_increased(cursor.connection, before1, after1)

        # When 33000 rows generated as [[33000 + i, "second-" + i] for i in 0..33000]
        # are inserted using multirow binding
        bulk_insert_id_name(cursor, table, 33000, 33000, "second-")

        # Then the bind file on SYSTEM$BIND from the last bulk insert should contain
        # the same values as the bound parameters
        after2 = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_reused_across_uploads(cursor.connection, after1, after2)

        # And Query "SELECT id, name FROM {table} ORDER BY id" is executed
        cursor.execute(f"SELECT id, name FROM {table} WHERE id IN (0, 1, 32999, 33000, 65999) ORDER BY id")

        # Then Result should contain the same values as the bound parameters from both bulk inserts
        assert cursor.fetchall() == [
            (0, "first-0"),
            (1, "first-1"),
            (32999, "first-32999"),
            (33000, "second-0"),
            (65999, "second-32999"),
        ]

    def test_should_round_trip_all_bindable_types_via_stage_binding(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And A temporary table with columns (id NUMBER, n NUMBER, f FLOAT, flag BOOLEAN, txt VARCHAR) exists
        table = f"{tmp_schema}.lb_types"
        cursor.execute(
            f"CREATE OR REPLACE TEMPORARY TABLE {table} (id NUMBER, n NUMBER, f FLOAT, flag BOOLEAN, txt VARCHAR)"
        )

        # When 13200 rows are inserted using multirow binding
        before = list_bind_stage_file_count(cursor.connection)
        bulk_insert_types(cursor, table, 13200)

        # Then the bind file on SYSTEM$BIND from the last bulk insert should contain
        # the same values as the bound parameters
        after = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_increased(cursor.connection, before, after)

        # And Query "SELECT id, n, f, flag, txt FROM {table} ORDER BY id" is executed
        cursor.execute(f"SELECT id, n, f, flag, txt FROM {table} WHERE id IN (0, 1, 7, 100, 13199) ORDER BY id")
        rows = cursor.fetchall()

        # Then Result should contain the same values as the bound parameters
        assert rows[0] == (0, None, 0.0, True, "txt-0")
        assert rows[1] == (1, 10, 0.5, False, "txt-1")
        assert rows[2] == (7, None, 3.5, False, "txt-7")
        assert rows[3] == (100, 1000, 50.0, True, "txt-100")
        assert rows[4] == (13199, 131990, 6599.5, False, "txt-13199")

    def test_should_preserve_csv_escaping_hazards_via_stage_binding(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And A temporary table with columns (id NUMBER, txt VARCHAR) exists
        table = f"{tmp_schema}.lb_hazards"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (id NUMBER, txt VARCHAR)")

        # When 33000 rows are inserted using multirow binding with values cycling every 7 rows
        # through [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""],
        # [5, NULL], [6, "日本語"]]
        before = list_bind_stage_file_count(cursor.connection)
        bulk_insert_hazard_strings(cursor, table, 33000)

        # Then the bind file on SYSTEM$BIND from the last bulk insert should contain
        # the same values as the bound parameters
        after = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_increased(cursor.connection, before, after)

        # And Query "SELECT id, txt FROM {table} WHERE id BETWEEN 0 AND 6 ORDER BY id" is executed
        cursor.execute(f"SELECT id, txt FROM {table} WHERE id BETWEEN 0 AND 6 ORDER BY id")

        # Then Result should contain rows [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"],
        # [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"]]
        assert cursor.fetchall() == [
            (0, "val,0"),
            (1, 'say"1"'),
            (2, "a\nb"),
            (3, "C:\\dir\\3"),
            (4, ""),
            (5, None),
            (6, "日本語"),
        ]

    def test_should_not_stage_bind_scalar_or_non_insert_queries_even_when_threshold_is_crossed(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
        set_stage_binding_threshold(cursor.connection, 1)
        before = list_bind_stage_file_count(cursor.connection)

        # When "SELECT ? AS val" is executed with bound integer value 42
        placeholder = _insert_placeholders(cursor, 1)
        cursor.execute(f"SELECT {placeholder} AS val", (42,))

        # Then the bind file on SYSTEM$BIND from the last execute should not contain the bound parameter values
        after = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_unchanged(cursor.connection, before, after)

        # And the result should equal 42
        assert cursor.fetchone() == (42,)

    def test_should_use_inline_json_when_row_count_is_below_client_stage_array_binding_threshold(
        self, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table = f"{tmp_schema}.lb_below_threshold"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (id NUMBER, name VARCHAR)")

        # And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 100
        set_stage_binding_threshold(cursor.connection, 100)

        # When 10 rows generated as [[i, "json-" + i] for i in 0..10] are inserted using multirow binding
        before = list_bind_stage_file_count(cursor.connection)
        bulk_insert_id_name(cursor, table, 10, 0, "json-")

        # Then no new bind file should have been uploaded to SYSTEM$BIND
        after = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_unchanged(cursor.connection, before, after)

        # And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
        cursor.execute(f"SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id")

        # Then Result should contain rows [[0, "json-0"], [9, "json-9"]]
        assert cursor.fetchall() == [(0, "json-0"), (9, "json-9")]

    def test_should_use_stage_binding_at_exact_threshold_boundary(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table = f"{tmp_schema}.lb_boundary"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (id NUMBER, name VARCHAR)")

        # And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 20
        set_stage_binding_threshold(cursor.connection, 20)

        # When 10 rows generated as [[i, "stage-" + i] for i in 0..10] are inserted using multirow binding
        before = list_bind_stage_file_count(cursor.connection)
        bulk_insert_id_name(cursor, table, 10, 0, "stage-")

        # Then the bind file on SYSTEM$BIND from the last bulk insert should contain
        # the same values as the bound parameters
        after = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_increased(cursor.connection, before, after)

        # And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
        cursor.execute(f"SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id")

        # Then Result should contain rows [[0, "stage-0"], [9, "stage-9"]]
        assert cursor.fetchall() == [(0, "stage-0"), (9, "stage-9")]

    def test_should_keep_an_all_null_row_on_the_inline_json_path_when_stage_binding_is_disabled(
        self, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And A temporary table with columns (id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE
        # INTEGER) exists
        table = f"{tmp_schema}.lb_all_null_inline"
        cursor.execute(
            f"CREATE OR REPLACE TEMPORARY TABLE {table} "
            "(id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER)"
        )

        # And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 0
        set_stage_binding_threshold(cursor.connection, 0)
        before = list_bind_stage_file_count(cursor.connection)

        # When a batch of one row with every column set to SQL NULL is inserted using multirow binding
        bulk_insert_all_null_row(cursor, table)

        # Then no new bind file should have been uploaded to SYSTEM$BIND
        after = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_unchanged(cursor.connection, before, after)

        # And every column of the round-tripped row reads back as SQL NULL
        cursor.execute(f"SELECT id, colA, colB, colC, colD, colE FROM {table}")
        assert cursor.fetchall() == [(None,) * 6]

    def test_should_stage_bind_an_all_null_row_when_the_bound_cell_count_meets_the_threshold(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And A temporary table with columns (id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE
        # INTEGER) exists
        table = f"{tmp_schema}.lb_all_null_stage"
        cursor.execute(
            f"CREATE OR REPLACE TEMPORARY TABLE {table} "
            "(id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER)"
        )

        # And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 6
        set_stage_binding_threshold(cursor.connection, 6)
        before = list_bind_stage_file_count(cursor.connection)

        # When a batch of one row with every column set to SQL NULL is inserted using multirow binding
        bulk_insert_all_null_row(cursor, table)

        # Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound
        # parameters
        after = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_increased(cursor.connection, before, after)

        # And every column of the round-tripped row reads back as SQL NULL
        cursor.execute(f"SELECT id, colA, colB, colC, colD, colE FROM {table}")
        assert cursor.fetchall() == [(None,) * 6]

    def test_should_fall_back_to_per_row_execution_for_non_insert_statements(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table = f"{tmp_schema}.lb_non_insert"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (id NUMBER, name VARCHAR)")
        placeholder = _insert_placeholders(cursor, 2)
        cursor.executemany(f"INSERT INTO {table} VALUES ({placeholder})", [(i, f"v{i}") for i in range(10)])

        # And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
        set_stage_binding_threshold(cursor.connection, 1)
        before = list_bind_stage_file_count(cursor.connection)

        # When an UPDATE with array bindings above the threshold is executed via executemany
        p1, p2 = _update_placeholders(cursor)
        update_sql = f"UPDATE {table} SET name = {p1} WHERE id = {p2}"
        cursor.executemany(update_sql, [(f"updated-{i}", i) for i in range(5)])

        # Then all updated rows reflect the new values
        cursor.execute(f"SELECT id, name FROM {table} WHERE id < 5 ORDER BY id")
        assert cursor.fetchall() == [(i, f"updated-{i}") for i in range(5)]

        # And no new bind file should have been uploaded
        after = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_unchanged(cursor.connection, before, after)

    def test_should_round_trip_far_future_dates_via_stage_binding(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And A temporary table with columns (id NUMBER, d DATE) exists
        table = f"{tmp_schema}.lb_far_future_dates"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table} (id NUMBER, d DATE)")

        # And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
        set_stage_binding_threshold(cursor.connection, 1)

        # A threshold of 1 routes the multirow INSERT through CSV stage binding. A naive
        # millisecond encoding overflows server-side for dates after ~year 2969, so the
        # stage path must switch to nanoseconds to round-trip the far-future rows.
        # When dates spanning the epoch-millisecond overflow bound are inserted using multirow binding
        rows = [
            (0, date(2024, 1, 15)),
            (1, date(2969, 1, 1)),
            (2, date(2970, 1, 1)),
            (3, date(3000, 6, 23)),
            (4, date(9999, 12, 31)),
        ]
        placeholders = _insert_placeholders(cursor, 2)
        before = list_bind_stage_file_count(cursor.connection)
        cursor.executemany(f"INSERT INTO {table} VALUES ({placeholders})", rows)
        # Confirm the bulk insert actually used stage binding.
        after = list_bind_stage_file_count(cursor.connection)
        assert_bind_stage_file_count_increased(cursor.connection, before, after)

        # And Query "SELECT id, d FROM {table} ORDER BY id" is executed
        cursor.execute(f"SELECT id, d FROM {table} ORDER BY id")

        # Then Result should contain the same dates as the bound parameters
        assert cursor.fetchall() == rows
