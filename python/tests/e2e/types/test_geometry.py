"""GEOMETRY type tests for Universal Driver.

This module tests the GEOMETRY type which represents geospatial data in a planar coordinate system.
Values are returned as strings by default (GeoJSON format).
The output format is controlled by the GEOMETRY_OUTPUT_FORMAT session parameter:
  GeoJSON (default), WKT, EWKT -> VARCHAR (str in Python)
  WKB, EWKB -> BINARY (bytes in Python)
Input via WKT strings through TO_GEOMETRY().

Reference: https://docs.snowflake.com/en/sql-reference/data-types-geospatial
"""

import pytest

from ...conftest import with_paramstyle
from .utils import assert_type, parse_geojson


# =============================================================================
# WKT TEST VALUES
# =============================================================================
# Point with non-geographic coordinates (planar coordinate system)
POINT_WKT = "POINT(1820.12 890.56)"
POINT_GEOJSON_COORDS = [1820.12, 890.56]

# Simple 3-vertex line
LINESTRING_WKT = "LINESTRING(0 0, 1 1, 2 2)"
LINESTRING_GEOJSON_COORDS = [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]

# Simple rectangle polygon (ring must be closed)
POLYGON_WKT = "POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))"
POLYGON_GEOJSON_COORDS = [[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0], [0.0, 0.0]]]

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
        sql = f"SELECT TO_GEOMETRY('{POINT_WKT}')"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, str)
        parsed = parse_geojson(result[0])
        assert parsed["type"] == "Point"
        assert parsed["coordinates"] == POINT_GEOJSON_COORDS


class TestGeometryLiteral:
    """Tests for GEOMETRY type using SELECT with literals (no tables)."""

    LITERAL_TEST_CASES = [
        ("Point", f"TO_GEOMETRY('{POINT_WKT}')", "Point", POINT_GEOJSON_COORDS),
        ("LineString", f"TO_GEOMETRY('{LINESTRING_WKT}')", "LineString", LINESTRING_GEOJSON_COORDS),
        ("Polygon", f"TO_GEOMETRY('{POLYGON_WKT}')", "Polygon", POLYGON_GEOJSON_COORDS),
    ]

    @pytest.mark.parametrize(
        "shape, query_value, expected_type, expected_coords",
        LITERAL_TEST_CASES,
        ids=[c[0] for c in LITERAL_TEST_CASES],
    )
    def test_should_select_shape_geometry_literal(
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

    def test_should_handle_null_geometry_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)'), TO_GEOMETRY(NULL)" is executed
        sql = f"SELECT TO_GEOMETRY('{POINT_WKT}'), TO_GEOMETRY(NULL)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [GeoJSON Point, NULL]
        assert isinstance(result[0], str)
        point = parse_geojson(result[0])
        assert point["type"] == "Point"
        assert point["coordinates"] == POINT_GEOJSON_COORDS
        assert result[1] is None


class TestGeometryOutputFormat:
    """Tests for GEOMETRY type with different output formats.

    The driver must correctly handle all 5 output formats controlled by
    the GEOMETRY_OUTPUT_FORMAT session parameter. Text formats (GeoJSON,
    WKT, EWKT) are returned as str; binary formats (WKB, EWKB) as bytes.
    """

    @pytest.mark.parametrize(
        "output_format, expected_type",
        [
            ("GeoJSON", str),
            ("WKT", str),
            ("WKB", bytes),
            ("EWKT", str),
            ("EWKB", bytes),
        ],
        ids=["GeoJSON", "WKT", "WKB", "EWKT", "EWKB"],
    )
    def test_should_select_geometry_in_format_output_format(self, connection_factory, output_format, expected_type):
        # Given Snowflake client is logged in
        pass
        # And Session parameter GEOMETRY_OUTPUT_FORMAT is set to <format>
        with connection_factory(session_parameters={"GEOMETRY_OUTPUT_FORMAT": output_format}) as conn:
            with conn.cursor() as cursor:
                # When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
                cursor.execute(f"SELECT TO_GEOMETRY('{POINT_WKT}')")
                result = cursor.fetchone()

                # Then Result should be returned as <expected_type> type
                assert isinstance(result[0], expected_type)
                assert len(result[0]) > 0


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
            f"SELECT 1, TO_GEOMETRY('{POINT_WKT}') "
            f"UNION ALL SELECT 2, TO_GEOMETRY('{LINESTRING_WKT}') "
            f"UNION ALL SELECT 3, TO_GEOMETRY('{POLYGON_WKT}')"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain the expected GeoJSON values
        assert len(rows) == 3
        point = parse_geojson(rows[0][1])
        assert point["type"] == "Point"

        linestring = parse_geojson(rows[1][1])
        assert linestring["type"] == "LineString"

        polygon = parse_geojson(rows[2][1])
        assert polygon["type"] == "Polygon"

    def test_should_handle_null_geometry_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with GEOMETRY column exists containing NULLs and values
        table_name = f"{tmp_schema}.geometry_null_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOMETRY)")
        execute_query(f"INSERT INTO {table_name} SELECT 1, TO_GEOMETRY('{POINT_WKT}') UNION ALL SELECT 2, NULL")

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain [GeoJSON Point, NULL]
        assert len(rows) == 2
        assert rows[0][0] == 1
        point = parse_geojson(rows[0][1])
        assert point["type"] == "Point"

        assert rows[1][0] == 2
        assert rows[1][1] is None


class TestGeometryMultipleChunks:
    """Tests for GEOMETRY type with multiple chunks downloading."""

    @pytest.mark.skip_for_json_result_set(
        reason="Multichunk geometry generates dynamic WKT that may not round-trip identically in JSON format"
    )
    def test_should_download_geometry_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query generating 20000 geometry points is executed
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

    @pytest.mark.parametrize(
        "bind_value, is_null",
        [
            (POINT_WKT, False),
            (None, True),
        ],
        ids=["WKT string", "NULL"],
    )
    def test_should_select_geometry_using_parameter_binding_with_input_type_value(
        self, execute_query, bind_value, is_null
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_GEOMETRY(?)" is executed with bound <input_type> value
        sql = "SELECT TO_GEOMETRY(?)"
        result = execute_query(sql, (bind_value,), single_row=True)

        # Then Result should <expected_result>
        if is_null:
            assert result == (None,)
        else:
            assert isinstance(result[0], str)
            geo = parse_geojson(result[0])
            assert geo["type"] == "Point"
            assert geo["coordinates"] == POINT_GEOJSON_COORDS

    def test_should_insert_geometry_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with GEOMETRY column exists
        table_name = f"{tmp_schema}.geometry_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, geo GEOMETRY)")

        # When Geometry WKT values are inserted using parameter binding via TO_GEOMETRY(?)
        test_values = [POINT_WKT, LINESTRING_WKT, POLYGON_WKT]
        for i, wkt in enumerate(test_values, 1):
            execute_query(f"INSERT INTO {table_name} SELECT {i}, TO_GEOMETRY(?)", (wkt,))

        # Then SELECT should return the inserted GeoJSON values
        rows = execute_query(f"SELECT geo FROM {table_name} ORDER BY id")
        assert len(rows) == 3

        geo1 = parse_geojson(rows[0][0])
        assert geo1["type"] == "Point"
        assert geo1["coordinates"] == POINT_GEOJSON_COORDS

        geo2 = parse_geojson(rows[1][0])
        assert geo2["type"] == "LineString"
        assert geo2["coordinates"] == LINESTRING_GEOJSON_COORDS

        geo3 = parse_geojson(rows[2][0])
        assert geo3["type"] == "Polygon"
        assert geo3["coordinates"] == POLYGON_GEOJSON_COORDS
