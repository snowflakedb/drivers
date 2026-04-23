"""GEOGRAPHY type tests for Universal Driver.

This module tests the GEOGRAPHY type which represents geospatial data on a sphere (WGS84).
Values are returned as JSON strings (GeoJSON format by default).
Input via WKT strings or GeoJSON through TO_GEOGRAPHY().

Snowflake GEOGRAPHY type represents geospatial data on a sphere.
Internal representation: Python connector returns these as JSON strings (str type).
Reference: https://docs.snowflake.com/en/sql-reference/data-types-geospatial
"""

from __future__ import annotations

from ...conftest import with_paramstyle
from .utils import assert_type, parse_geojson


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
        sql = "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, str)

        # And Parsed GeoJSON value should contain expected structure
        geo = parse_geojson(result[0])
        assert geo["type"] == "Point"
        assert geo["coordinates"] == [-122.35, 37.55]


class TestGeographyLiteral:
    """Tests for GEOGRAPHY type using SELECT with literals (no tables)."""

    def test_should_select_geography_literals_with_different_shapes(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)'), TO_GEOGRAPHY('LINESTRING(0 0, 1 1, 2 2)'),
        # TO_GEOGRAPHY('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')" is executed
        sql = (
            "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)'), "
            "TO_GEOGRAPHY('LINESTRING(0 0, 1 1, 2 2)'), "
            "TO_GEOGRAPHY('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')"
        )
        result = execute_query(sql, single_row=True)

        # Then Result should contain GeoJSON values for Point, LineString, and Polygon
        assert_type(result, str)

        # Parse and verify each geometry type
        point = parse_geojson(result[0])
        assert point["type"] == "Point"
        assert point["coordinates"] == [-122.35, 37.55]

        linestring = parse_geojson(result[1])
        assert linestring["type"] == "LineString"
        assert linestring["coordinates"] == [[0, 0], [1, 1], [2, 2]]

        polygon = parse_geojson(result[2])
        assert polygon["type"] == "Polygon"
        assert polygon["coordinates"] == [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]]

    def test_should_select_geography_from_geojson_input(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOGRAPHY('{\"type\":\"Point\",\"coordinates\":[-122.35,37.55]}')" is executed
        sql = 'SELECT TO_GEOGRAPHY(\'{"type":"Point","coordinates":[-122.35,37.55]}\')'
        result = execute_query(sql, single_row=True)

        # Then Result should contain a GeoJSON Point value
        assert isinstance(result[0], str)
        geo = parse_geojson(result[0])
        assert geo["type"] == "Point"
        assert geo["coordinates"] == [-122.35, 37.55]

    def test_should_handle_null_geography_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)'), TO_GEOGRAPHY(NULL)" is executed
        sql = "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)'), TO_GEOGRAPHY(NULL)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [GeoJSON Point, NULL]
        assert isinstance(result[0], str)
        geo = parse_geojson(result[0])
        assert geo["type"] == "Point"
        assert geo["coordinates"] == [-122.35, 37.55]
        assert result[1] is None


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
            f"SELECT 1, TO_GEOGRAPHY('POINT(-122.35 37.55)') "
            f"UNION ALL SELECT 2, TO_GEOGRAPHY('LINESTRING(0 0, 1 1, 2 2)')"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain the expected GeoJSON values
        assert len(rows) == 2

        # Verify first row
        assert rows[0][0] == 1
        geo1 = parse_geojson(rows[0][1])
        assert geo1["type"] == "Point"

        # Verify second row
        assert rows[1][0] == 2
        geo2 = parse_geojson(rows[1][1])
        assert geo2["type"] == "LineString"

    def test_should_handle_null_geography_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with GEOGRAPHY column exists containing NULLs and values
        table_name = f"{tmp_schema}.geography_null_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOGRAPHY)")
        execute_query(
            f"INSERT INTO {table_name} SELECT 1, TO_GEOGRAPHY('POINT(-122.35 37.55)') UNION ALL SELECT 2, NULL"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain [GeoJSON Point, NULL]
        assert len(rows) == 2

        # Verify first row
        assert rows[0][0] == 1
        geo = parse_geojson(rows[0][1])
        assert geo["type"] == "Point"

        # Verify second row
        assert rows[1][0] == 2
        assert rows[1][1] is None


class TestGeographyMultipleChunks:
    """Tests for GEOGRAPHY type with multiple chunks downloading."""

    def test_should_download_geography_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOGRAPHY('POINT(' || (MOD(seq8(), 360) - 180) || ' '
        # || (MOD(seq8(), 180) - 90) || ')') AS geo FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
        sql = (
            "SELECT TO_GEOGRAPHY('POINT(' || (MOD(seq8(), 360) - 180)"
            " || ' ' || (MOD(seq8(), 180) - 90) || ')') AS geo "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v"
        )
        rows = execute_query(sql)

        # Then All 20000 rows should be fetched and each should be a non-null string value
        assert len(rows) == LARGE_RESULT_SET_SIZE
        for row in rows:
            assert isinstance(row[0], str)


@with_paramstyle("qmark")
class TestGeographyBinding:
    """Tests for GEOGRAPHY type using parameter binding."""

    def test_should_select_geography_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound WKT string 'POINT(-122.35 37.55)'
        sql = "SELECT TO_GEOGRAPHY(?)"
        result = execute_query(sql, ("POINT(-122.35 37.55)",), single_row=True)

        # Then Result should contain a GeoJSON Point value
        assert isinstance(result[0], str)
        geo = parse_geojson(result[0])
        assert geo["type"] == "Point"
        assert geo["coordinates"] == [-122.35, 37.55]

    def test_should_select_null_geography_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound NULL value
        sql = "SELECT TO_GEOGRAPHY(?)"
        result = execute_query(sql, (None,), single_row=True)

        # Then Result should be NULL
        assert result == (None,)

    def test_should_insert_geography_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with GEOGRAPHY column exists
        table_name = f"{tmp_schema}.geography_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOGRAPHY)")

        # When Geography WKT values are inserted using parameter binding via TO_GEOGRAPHY(?)
        test_values = [
            "POINT(-122.35 37.55)",
            "LINESTRING(0 0, 1 1, 2 2)",
            "POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))",
        ]
        for i, wkt in enumerate(test_values, 1):
            # Uses a loop instead of executemany because INSERT ... SELECT TO_GEOGRAPHY(?)
            # is incompatible with Snowflake's server-side array binding (VALUES-only).
            execute_query(f"INSERT INTO {table_name} SELECT {i}, TO_GEOGRAPHY(?)", (wkt,))

        # Then SELECT should return the inserted GeoJSON values
        rows = execute_query(f"SELECT geo FROM {table_name} ORDER BY id")
        assert len(rows) == 3

        # Verify Point
        geo1 = parse_geojson(rows[0][0])
        assert geo1["type"] == "Point"
        assert geo1["coordinates"] == [-122.35, 37.55]

        # Verify LineString
        geo2 = parse_geojson(rows[1][0])
        assert geo2["type"] == "LineString"
        assert geo2["coordinates"] == [[0, 0], [1, 1], [2, 2]]

        # Verify Polygon
        geo3 = parse_geojson(rows[2][0])
        assert geo3["type"] == "Polygon"
        assert geo3["coordinates"] == [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]]


class TestGeographyJsonResultFormat:
    """Tests for GEOGRAPHY type with JSON result format."""

    def test_should_select_geography_with_json_result_format(self, connection_factory):
        # Given Snowflake client is logged in
        pass

        # And Session parameter PYTHON_CONNECTOR_QUERY_RESULT_FORMAT is set to JSON
        with connection_factory(session_parameters={"PYTHON_CONNECTOR_QUERY_RESULT_FORMAT": "JSON"}) as conn:
            with conn.cursor() as cursor:
                # When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
                sql = "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')"
                cursor.execute(sql)
                result = cursor.fetchone()

                # Then Result should contain a GeoJSON Point value
                assert isinstance(result[0], str)
                geo = parse_geojson(result[0])
                assert geo["type"] == "Point"
                assert geo["coordinates"] == [-122.35, 37.55]
