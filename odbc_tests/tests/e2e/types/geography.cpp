#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <optional>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "geo_test_helpers.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should select shape geography literal", "[geography]") {
  // Given Snowflake client is logged in
  Connection conn;

  struct Case {
    const char* name;
    const char* query;
    const char* shape;
  };
  const Case cases[] = {
      {"Point", "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')", "Point"},
      {"LineString", "SELECT TO_GEOGRAPHY('LINESTRING(0 0, 1 1, 2 2)')", "LineString"},
      {"Polygon", "SELECT TO_GEOGRAPHY('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')", "Polygon"},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);

    // When Query "SELECT <query_value>" is executed
    const auto stmt = conn.execute_fetch(test_case.query);

    // Then Result should contain a GeoJSON <shape> value
    require_geojson_shape(fetch_char(stmt, 1), test_case.shape);
  }
}

TEST_CASE("should select geography from GeoJSON input", "[geography]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TO_GEOGRAPHY('{"type":"Point","coordinates":[-122.35,37.55]}')" is executed
  const auto stmt = conn.execute_fetch("SELECT TO_GEOGRAPHY('{\"type\":\"Point\",\"coordinates\":[-122.35,37.55]}')");

  // Then Result should contain a GeoJSON Point value
  require_geojson_shape(fetch_char(stmt, 1), "Point");
}

TEST_CASE("should cast geography to expected_type for format output format", "[geography]") {
  // Given Snowflake client is logged in
  Connection conn;

  struct Case {
    const char* format;
    bool is_text;
  };
  const Case cases[] = {
      {"GeoJSON", true}, {"WKT", true}, {"WKB", false}, {"EWKT", true}, {"EWKB", false},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.format);

    // And Session parameter GEOGRAPHY_OUTPUT_FORMAT is set to <format>
    conn.execute(std::string("ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = '") + test_case.format + "'");

    // When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
    const auto stmt = conn.execute_fetch("SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')");

    // Then Result should be returned as <expected_type> type
    if (test_case.is_text) {
      require_sql_type(stmt, 1, SQL_VARCHAR);
      REQUIRE_FALSE(fetch_char(stmt, 1).empty());
    } else {
      REQUIRE_FALSE(fetch_binary(stmt, 1).empty());
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select geography values from table", "[geography]") {
  // Given Snowflake client is logged in

  // And Table with GEOGRAPHY column exists with WKT values
  conn.execute("CREATE TEMPORARY TABLE geo_table (id INT, geo GEOGRAPHY)");
  conn.execute(
      "INSERT INTO geo_table SELECT 1, TO_GEOGRAPHY('POINT(-122.35 37.55)') "
      "UNION ALL SELECT 2, TO_GEOGRAPHY('LINESTRING(0 0, 1 1, 2 2)')");

  // When Query "SELECT * FROM <table> ORDER BY id" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM geo_table ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain the expected GeoJSON values
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 1);
  require_geojson_shape(fetch_char(stmt, 2), "Point");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 2);
  require_geojson_shape(fetch_char(stmt, 2), "LineString");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should handle NULL geography values from table", "[geography]") {
  // Given Snowflake client is logged in

  // And Table with GEOGRAPHY column exists containing NULLs and values
  conn.execute("CREATE TEMPORARY TABLE geo_null (id INT, geo GEOGRAPHY)");
  conn.execute("INSERT INTO geo_null SELECT 1, TO_GEOGRAPHY('POINT(-122.35 37.55)') UNION ALL SELECT 2, NULL");

  // When Query "SELECT * FROM <table> ORDER BY id" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM geo_null ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [GeoJSON Point, NULL]
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_geojson_shape(fetch_char(stmt, 2), "Point");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 2) == std::nullopt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("should download geography data in multiple chunks", "[geography]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query generating 20000 geography points is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                sqlchar("SELECT id, TO_GEOGRAPHY('POINT(' || (MOD(id, 360) - 180) || ' ' || "
                                        "(MOD(id, 180) - 90) || ')') AS geo "
                                        "FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id "
                                        "FROM TABLE(GENERATOR(ROWCOUNT => 20000))) ORDER BY id"),
                                SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then All 20000 rows should be fetched with valid GeoJSON Point values
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);
    INFO("row=" << row_count);
    REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == row_count);
    require_geojson_shape(fetch_char(stmt, 2), "Point");
    row_count++;
  }
  REQUIRE(row_count == 20000);
}

TEST_CASE("should select geography using parameter binding with input_type value", "[geography]") {
  // Given Snowflake client is logged in
  Connection conn;

  struct Case {
    const char* name;
    const char* wkt;
    const char* shape;
  };
  const Case cases[] = {
      {"WKT string", "POINT(-122.35 37.55)", "Point"},
      {"NULL", nullptr, nullptr},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);
    std::string wkt = test_case.wkt == nullptr ? std::string() : test_case.wkt;
    SQLLEN ind = test_case.wkt == nullptr ? SQL_NULL_DATA : SQL_NTS;
    const auto stmt = conn.createStatement();
    SQLRETURN ret = SQL_ERROR;
    if (test_case.wkt == nullptr) {
      ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 0, 0, nullptr, 0, &ind);
      REQUIRE_ODBC(ret, stmt);
    } else {
      bind_varchar(stmt, 1, wkt.data(), ind);
    }

    // When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound <input_type> value
    ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT TO_GEOGRAPHY(?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);

    // Then Result should <expected_result>
    if (test_case.shape == nullptr) {
      REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
    } else {
      require_geojson_shape(fetch_char(stmt, 1), test_case.shape);
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should insert geography using parameter binding", "[geography]") {
  // Given Snowflake client is logged in

  // And Table with GEOGRAPHY column exists
  conn.execute("CREATE TEMPORARY TABLE geo_bind (id INT, geo GEOGRAPHY)");

  // When Geography WKT values are inserted using parameter binding via TO_GEOGRAPHY(?)
  char point[] = "POINT(-122.35 37.55)";
  char line[] = "LINESTRING(0 0, 1 1, 2 2)";
  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  SQLLEN wkt_ind = SQL_NTS;
  const auto insert_stmt = conn.createStatement();
  SQLRETURN ret =
      SQLPrepare(insert_stmt.getHandle(), sqlchar("INSERT INTO geo_bind SELECT ?, TO_GEOGRAPHY(?)"), SQL_NTS);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLBindParameter(insert_stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC(ret, insert_stmt);
  bind_varchar(insert_stmt, 2, point, wkt_ind);
  ret = SQLExecute(insert_stmt.getHandle());
  REQUIRE_ODBC(ret, insert_stmt);
  id = 2;
  bind_varchar(insert_stmt, 2, line, wkt_ind);
  ret = SQLExecute(insert_stmt.getHandle());
  REQUIRE_ODBC(ret, insert_stmt);

  // Then SELECT should return the inserted GeoJSON values
  const auto stmt = conn.createStatement();
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT geo FROM geo_bind ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_geojson_shape(fetch_char(stmt, 1), "Point");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_geojson_shape(fetch_char(stmt, 1), "LineString");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}
