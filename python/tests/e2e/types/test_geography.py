"""GEOGRAPHY type tests for Universal Driver.

This module tests the GEOGRAPHY type which represents geospatial data on a sphere (WGS84).
Values are returned as strings by default (GeoJSON format).
The output format is controlled by the GEOGRAPHY_OUTPUT_FORMAT session parameter:
  GeoJSON (default), WKT, EWKT -> VARCHAR (str in Python)
  WKB, EWKB -> BINARY (bytearray in Python)
Input via WKT strings or GeoJSON through TO_GEOGRAPHY().

Reference: https://docs.snowflake.com/en/sql-reference/data-types-geospatial
"""

import pytest

from ...conftest import with_paramstyle
from .utils import assert_sequential_values, assert_type, parse_geojson


# =============================================================================
# WKT TEST VALUES
# =============================================================================
# Point on the San Francisco coast (longitude, latitude)
POINT_WKT = "POINT(-122.35 37.55)"
POINT_GEOJSON_COORDS = [-122.35, 37.55]

# Simple 3-vertex line
LINESTRING_WKT = "LINESTRING(0 0, 1 1, 2 2)"
LINESTRING_GEOJSON_COORDS = [[0, 0], [1, 1], [2, 2]]

# Simple square polygon (ring must be closed)
POLYGON_WKT = "POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))"
POLYGON_GEOJSON_COORDS = [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]]

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 20_000


class TestGeographyTypeCasting:
    """Tests for GEOGRAPHY type casting to appropriate type."""

    def test_should_cast_geography_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
        sql = f"SELECT TO_GEOGRAPHY('{POINT_WKT}')"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, str)
        geo = parse_geojson(result[0])
        assert geo["type"] == "Point"
        assert geo["coordinates"] == POINT_GEOJSON_COORDS


class TestGeographyLiteral:
    """Tests for GEOGRAPHY type using SELECT with literals (no tables)."""

    LITERAL_TEST_CASES = [
        ("Point", f"TO_GEOGRAPHY('{POINT_WKT}')", "Point", POINT_GEOJSON_COORDS),
        ("LineString", f"TO_GEOGRAPHY('{LINESTRING_WKT}')", "LineString", LINESTRING_GEOJSON_COORDS),
        ("Polygon", f"TO_GEOGRAPHY('{POLYGON_WKT}')", "Polygon", POLYGON_GEOJSON_COORDS),
    ]

    @pytest.mark.parametrize(
        "shape, query_value, expected_type, expected_coords",
        LITERAL_TEST_CASES,
        ids=[c[0] for c in LITERAL_TEST_CASES],
    )
    def test_should_select_shape_geography_literal(
        self, execute_query, shape, query_value, expected_type, expected_coords
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_value>" is executed
        sql = f"SELECT {query_value}"
        result = execute_query(sql, single_row=True)

        # Then Result should contain a GeoJSON <shape> value
        assert isinstance(result[0], str)
        geo = parse_geojson(result[0])
        assert geo["type"] == expected_type
        assert geo["coordinates"] == expected_coords

    def test_should_select_geography_from_geojson_input(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOGRAPHY('{"type":"Point","coordinates":[-122.35,37.55]}')" is executed
        geojson = f'{{"type":"Point","coordinates":[{POINT_GEOJSON_COORDS[0]},{POINT_GEOJSON_COORDS[1]}]}}'
        sql = f"SELECT TO_GEOGRAPHY('{geojson}')"
        result = execute_query(sql, single_row=True)

        # Then Result should contain a GeoJSON Point value
        assert isinstance(result[0], str)
        geo = parse_geojson(result[0])
        assert geo["type"] == "Point"
        assert geo["coordinates"] == POINT_GEOJSON_COORDS


class TestGeographyOutputFormat:
    """Tests for GEOGRAPHY type with different output formats.

    The driver must correctly handle all 5 output formats controlled by
    the GEOGRAPHY_OUTPUT_FORMAT session parameter. Text formats (GeoJSON,
    WKT, EWKT) are returned as str; binary formats (WKB, EWKB) as bytearray.
    """

    @pytest.mark.parametrize(
        "output_format, expected_type",
        [
            ("GeoJSON", str),
            ("WKT", str),
            ("WKB", bytearray),
            ("EWKT", str),
            ("EWKB", bytearray),
        ],
        ids=["GeoJSON", "WKT", "WKB", "EWKT", "EWKB"],
    )
    def test_should_select_geography_in_format_output_format(self, connection_factory, output_format, expected_type):
        # Given Snowflake client is logged in
        pass
        # And Session parameter GEOGRAPHY_OUTPUT_FORMAT is set to <format>
        with connection_factory(session_parameters={"GEOGRAPHY_OUTPUT_FORMAT": output_format}) as conn:
            with conn.cursor() as cursor:
                # When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
                cursor.execute(f"SELECT TO_GEOGRAPHY('{POINT_WKT}')")
                result = cursor.fetchone()

                # Then Result should be returned as <expected_type> type
                assert isinstance(result[0], expected_type)
                assert len(result[0]) > 0


class TestGeographyTable:
    """Tests for GEOGRAPHY type using table operations."""

    def test_should_select_geography_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with GEOGRAPHY column exists with WKT values
        table_name = f"{tmp_schema}.geography_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOGRAPHY)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT 1, TO_GEOGRAPHY('{POINT_WKT}') "
            f"UNION ALL SELECT 2, TO_GEOGRAPHY('{LINESTRING_WKT}')"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain the expected GeoJSON values
        assert len(rows) == 2
        assert rows[0][0] == 1
        geo1 = parse_geojson(rows[0][1])
        assert geo1["type"] == "Point"
        assert geo1["coordinates"] == POINT_GEOJSON_COORDS

        assert rows[1][0] == 2
        geo2 = parse_geojson(rows[1][1])
        assert geo2["type"] == "LineString"
        assert geo2["coordinates"] == LINESTRING_GEOJSON_COORDS

    def test_should_handle_null_geography_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with GEOGRAPHY column exists containing NULLs and values
        table_name = f"{tmp_schema}.geography_null_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOGRAPHY)")
        execute_query(f"INSERT INTO {table_name} SELECT 1, TO_GEOGRAPHY('{POINT_WKT}') UNION ALL SELECT 2, NULL")

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain [GeoJSON Point, NULL]
        assert len(rows) == 2
        assert rows[0][0] == 1
        geo = parse_geojson(rows[0][1])
        assert geo["type"] == "Point"
        assert geo["coordinates"] == POINT_GEOJSON_COORDS

        assert rows[1][0] == 2
        assert rows[1][1] is None


class TestGeographyMultipleChunks:
    """Tests for GEOGRAPHY type with multiple chunks downloading."""

    @pytest.mark.skip_for_json_result_set(
        reason="Multichunk geography generates dynamic WKT that may not round-trip identically in JSON format"
    )
    def test_should_download_geography_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query generating 20000 geography points is executed
        sql = (
            "SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id, "
            "TO_GEOGRAPHY('POINT(' || (MOD(ROW_NUMBER() OVER (ORDER BY seq8()) - 1, 360) - 180) "
            "|| ' ' || (MOD(ROW_NUMBER() OVER (ORDER BY seq8()) - 1, 180) - 90) || ')') AS geo "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) "
            f"ORDER BY id"
        )
        rows = execute_query(sql)

        # Then All 20000 rows should be fetched with valid GeoJSON Point values
        assert len(rows) == LARGE_RESULT_SET_SIZE
        assert_type([row[1] for row in rows], str)

        def expected_row(i):
            lon = (i % 360) - 180
            lat = (i % 180) - 90
            return (i, [float(lon), float(lat)])

        def compare_row(actual, expected):
            geo = parse_geojson(actual[1])
            return actual[0] == expected[0] and geo["type"] == "Point" and geo["coordinates"] == expected[1]

        assert_sequential_values(rows, LARGE_RESULT_SET_SIZE, transform=expected_row, compare=compare_row)


@with_paramstyle("qmark")
class TestGeographyBinding:
    """Tests for GEOGRAPHY type using parameter binding."""

    @pytest.mark.parametrize(
        "bind_value, is_null",
        [
            (POINT_WKT, False),
            (None, True),
        ],
        ids=["WKT string", "NULL"],
    )
    def test_should_select_geography_using_parameter_binding_with_input_type_value(
        self, execute_query, bind_value, is_null
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound <input_type> value
        sql = "SELECT TO_GEOGRAPHY(?)"
        result = execute_query(sql, (bind_value,), single_row=True)

        # Then Result should <expected_result>
        if is_null:
            assert result == (None,)
        else:
            assert isinstance(result[0], str)
            geo = parse_geojson(result[0])
            assert geo["type"] == "Point"
            assert geo["coordinates"] == POINT_GEOJSON_COORDS

    def test_should_insert_geography_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with GEOGRAPHY column exists
        table_name = f"{tmp_schema}.geography_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOGRAPHY)")

        # When Geography WKT values are inserted using parameter binding via TO_GEOGRAPHY(?)
        test_data = [(1, POINT_WKT), (2, LINESTRING_WKT), (3, POLYGON_WKT)]
        for row_id, wkt in test_data:
            execute_query(f"INSERT INTO {table_name} SELECT ?, TO_GEOGRAPHY(?)", (row_id, wkt))

        # Then SELECT should return the inserted GeoJSON values
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")
        assert len(rows) == 3

        geo1 = parse_geojson(rows[0][1])
        assert geo1["type"] == "Point"
        assert geo1["coordinates"] == POINT_GEOJSON_COORDS

        geo2 = parse_geojson(rows[1][1])
        assert geo2["type"] == "LineString"
        assert geo2["coordinates"] == LINESTRING_GEOJSON_COORDS

        geo3 = parse_geojson(rows[2][1])
        assert geo3["type"] == "Polygon"
        assert geo3["coordinates"] == POLYGON_GEOJSON_COORDS
