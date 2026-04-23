"""GEOMETRY type tests for Universal Driver.

This module tests GEOMETRY type which represents geospatial data in a planar coordinate system.
Values are returned as JSON strings (GeoJSON format by default).
Input via WKT strings through TO_GEOMETRY().

Snowflake GEOMETRY type: planar coordinate system (arbitrary x,y coordinates).
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


class TestGeometryTypeCasting:
    """Tests for GEOMETRY type casting to appropriate type."""

    def test_should_cast_geometry_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
        sql = "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, str)

        # And Parsed GeoJSON value should be a valid Point
        parsed = parse_geojson(result[0])
        assert parsed["type"] == "Point"
        assert parsed["coordinates"] == [1820.12, 890.56]


class TestGeometryLiteral:
    """Tests for GEOMETRY type using SELECT with literals (no tables)."""

    def test_should_select_geometry_literals_with_different_shapes(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOMETRY('POINT(0 0)'), TO_GEOMETRY('LINESTRING(1 1, 2 2, 3 3)'),
        # TO_GEOMETRY('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))')" is executed
        sql = (
            "SELECT TO_GEOMETRY('POINT(0 0)'), "
            "TO_GEOMETRY('LINESTRING(1 1, 2 2, 3 3)'), "
            "TO_GEOMETRY('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))')"
        )
        result = execute_query(sql, single_row=True)

        # Then Result should contain GeoJSON values for Point, LineString, and Polygon
        assert_type(result, str)

        # Verify Point
        point = parse_geojson(result[0])
        assert point["type"] == "Point"
        assert point["coordinates"] == [0.0, 0.0]

        # Verify LineString
        linestring = parse_geojson(result[1])
        assert linestring["type"] == "LineString"
        assert linestring["coordinates"] == [[1.0, 1.0], [2.0, 2.0], [3.0, 3.0]]

        # Verify Polygon
        polygon = parse_geojson(result[2])
        assert polygon["type"] == "Polygon"
        assert polygon["coordinates"] == [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]

    def test_should_handle_null_geometry_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOMETRY('POINT(0 0)'), TO_GEOMETRY(NULL)" is executed
        sql = "SELECT TO_GEOMETRY('POINT(0 0)'), TO_GEOMETRY(NULL)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [GeoJSON Point, NULL]
        assert isinstance(result[0], str)
        point = parse_geojson(result[0])
        assert point["type"] == "Point"
        assert point["coordinates"] == [0.0, 0.0]
        assert result[1] is None


class TestGeometryTable:
    """Tests for GEOMETRY type using table operations."""

    def test_should_select_geometry_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with GEOMETRY column exists with WKT values
        table_name = f"{tmp_schema}.geometry_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOMETRY)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT 1, TO_GEOMETRY('POINT(1820.12 890.56)') "
            f"UNION ALL SELECT 2, TO_GEOMETRY('LINESTRING(0 0, 1 1, 2 2)') "
            f"UNION ALL SELECT 3, TO_GEOMETRY('POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))')"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain the expected GeoJSON values
        assert len(rows) == 3

        # Verify Point
        point = parse_geojson(rows[0][1])
        assert point["type"] == "Point"
        assert point["coordinates"] == [1820.12, 890.56]

        # Verify LineString
        linestring = parse_geojson(rows[1][1])
        assert linestring["type"] == "LineString"
        assert linestring["coordinates"] == [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]

        # Verify Polygon
        polygon = parse_geojson(rows[2][1])
        assert polygon["type"] == "Polygon"
        assert polygon["coordinates"] == [[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0], [0.0, 0.0]]]

    def test_should_handle_null_geometry_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with GEOMETRY column exists containing NULLs and values
        table_name = f"{tmp_schema}.geometry_null_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOMETRY)")
        execute_query(f"INSERT INTO {table_name} SELECT 1, TO_GEOMETRY('POINT(0 0)') UNION ALL SELECT 2, NULL")

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain [GeoJSON Point, NULL]
        assert len(rows) == 2

        # Verify Point
        point = parse_geojson(rows[0][1])
        assert point["type"] == "Point"
        assert point["coordinates"] == [0.0, 0.0]

        # Verify NULL
        assert rows[1][1] is None


class TestGeometryMultipleChunks:
    """Tests for GEOMETRY type with multiple chunks downloading."""

    def test_should_download_geometry_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOMETRY('POINT(' || seq8() || ' ' || seq8() || ')') AS geo
        # FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
        sql = (
            f"SELECT TO_GEOMETRY('POINT(' || seq8() || ' ' || seq8() || ')') AS geo "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v"
        )
        rows = execute_query(sql)

        # Then All 20000 rows should be fetched and each should be a non-null string value
        assert len(rows) == LARGE_RESULT_SET_SIZE
        for row in rows:
            assert isinstance(row[0], str)


@with_paramstyle("qmark")
class TestGeometryBinding:
    """Tests for GEOMETRY type using parameter binding."""

    def test_should_select_geometry_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOMETRY(?)" is executed with bound WKT string 'POINT(1820.12 890.56)'
        sql = "SELECT TO_GEOMETRY(?)"
        result = execute_query(sql, ("POINT(1820.12 890.56)",), single_row=True)

        # Then Result should contain a GeoJSON Point value
        assert isinstance(result[0], str)
        parsed = parse_geojson(result[0])
        assert parsed["type"] == "Point"
        assert parsed["coordinates"] == [1820.12, 890.56]

    def test_should_select_null_geometry_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOMETRY(?)" is executed with bound NULL value
        sql = "SELECT TO_GEOMETRY(?)"
        result = execute_query(sql, (None,), single_row=True)

        # Then Result should be NULL
        assert result == (None,)

    def test_should_insert_geometry_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with GEOMETRY column exists
        table_name = f"{tmp_schema}.geometry_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOMETRY)")

        # When Geometry WKT values are inserted using parameter binding via TO_GEOMETRY(?)
        test_values = [
            (1, "POINT(1820.12 890.56)"),
            (2, "LINESTRING(0 0, 1 1, 2 2)"),
            (3, "POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))"),
        ]
        for id_val, wkt_val in test_values:
            # Uses a loop instead of executemany because INSERT ... SELECT TO_GEOMETRY(?)
            # is incompatible with Snowflake's server-side array binding (VALUES-only).
            execute_query(f"INSERT INTO {table_name} SELECT ?, TO_GEOMETRY(?)", (id_val, wkt_val))

        # Then SELECT should return the inserted GeoJSON values
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")
        assert len(rows) == 3

        # Verify Point
        point = parse_geojson(rows[0][1])
        assert point["type"] == "Point"
        assert point["coordinates"] == [1820.12, 890.56]

        # Verify LineString
        linestring = parse_geojson(rows[1][1])
        assert linestring["type"] == "LineString"
        assert linestring["coordinates"] == [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]

        # Verify Polygon
        polygon = parse_geojson(rows[2][1])
        assert polygon["type"] == "Polygon"
        assert polygon["coordinates"] == [[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0], [0.0, 0.0]]]


class TestGeometryJsonResultFormat:
    """Tests for GEOMETRY type with JSON result format."""

    def test_should_select_geometry_with_json_result_format(self, connection_factory):
        # Given Snowflake client is logged in
        pass

        # And Session parameter PYTHON_CONNECTOR_QUERY_RESULT_FORMAT is set to JSON
        with connection_factory(session_parameters={"PYTHON_CONNECTOR_QUERY_RESULT_FORMAT": "JSON"}) as conn:
            with conn.cursor() as cursor:
                # When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
                cursor.execute("SELECT TO_GEOMETRY('POINT(1820.12 890.56)')")
                result = cursor.fetchone()

                # Then Result should contain a GeoJSON Point value
                assert isinstance(result[0], str)
                parsed = parse_geojson(result[0])
                assert parsed["type"] == "Point"
                assert parsed["coordinates"] == [1820.12, 890.56]
