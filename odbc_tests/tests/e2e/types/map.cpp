// MAP datatype ODBC tests
// Based on: tests/definitions/shared/types/map.feature
//
// Snowflake MAP is a semi-structured data type storing key-value pairs
// with typed keys and typed values. Created by casting OBJECT to MAP.
// ODBC returns MAP values as SQL_C_CHAR (JSON string representation).
// Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured

#include <optional>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "macros.hpp"
#include "odbc_cast.hpp"

static bool json_contains_key(const std::string& json, const std::string& key) {
  return json.find("\"" + key + "\"") != std::string::npos;
}

// ============================================================================
// TYPE CASTING
// ============================================================================

TEST_CASE("should cast map values to appropriate type", "[datatype][map]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting a MAP(VARCHAR, VARCHAR) value is executed
  const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('x', '1', 'y', '2')::MAP(VARCHAR, VARCHAR)");

  // Then Value should be returned as appropriate type
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  REQUIRE(!value.empty());

  // And Value should be a map containing key 'x' with value '1' and key 'y' with value '2'
  CHECK(json_contains_key(value, "x"));
  CHECK(json_contains_key(value, "y"));
  CHECK(value.find("\"1\"") != std::string::npos);
  CHECK(value.find("\"2\"") != std::string::npos);
}

// ============================================================================
// SIMPLE SELECTS - LITERALS
// ============================================================================

TEST_CASE("should select hardcoded map literals", "[datatype][map]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting a MAP(VARCHAR, INTEGER) value with keys [a, b] is executed
  const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('a', 1, 'b', 2)::MAP(VARCHAR, INTEGER)");

  // Then Result should contain a map with 2 entries
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  REQUIRE(!value.empty());
  CHECK(json_contains_key(value, "a"));
  CHECK(json_contains_key(value, "b"));

  // And Map values should be a=1 and b=2
  CHECK(value.find("1") != std::string::npos);
  CHECK(value.find("2") != std::string::npos);
}

TEST_CASE("should select map corner case values from literals", "[datatype][map]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Queries selecting corner case map literals are executed
  // Then Results should contain expected corner case map values

  // Empty map
  {
    const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT()::MAP(VARCHAR, VARCHAR)");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(value == "{}");
  }

  // Single entry map
  {
    const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('only', 'one')::MAP(VARCHAR, VARCHAR)");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(json_contains_key(value, "only"));
    CHECK(value.find("\"one\"") != std::string::npos);
  }

  // NULL::MAP
  {
    const auto stmt = conn.execute_fetch("SELECT NULL::MAP(VARCHAR, VARCHAR)");
    auto value = get_data_optional<SQL_C_CHAR>(stmt, 1);
    CHECK(value == std::nullopt);
  }
}

// ============================================================================
// SELECT FROM TABLE
// ============================================================================

TEST_CASE("should select map values from table", "[datatype][map]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with MAP column is created
  conn.execute("CREATE TABLE map_table (col MAP(VARCHAR, VARCHAR))");

  // And The table is populated with map values
  conn.execute(
      "INSERT INTO map_table "
      "SELECT OBJECT_CONSTRUCT('k1', 'v1')::MAP(VARCHAR, VARCHAR)");
  conn.execute(
      "INSERT INTO map_table "
      "SELECT OBJECT_CONSTRUCT('k2', 'v2')::MAP(VARCHAR, VARCHAR)");

  // When Query "SELECT * FROM <table>" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM map_table"), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain the inserted map values
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    REQUIRE(!value.empty());
    row_count++;
  }

  CHECK(row_count == 2);
}

TEST_CASE("should select map corner case values from table", "[datatype][map]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with MAP column is created
  conn.execute("CREATE TABLE map_corner_table (col MAP(VARCHAR, VARCHAR))");

  // And The table is populated with corner case map values
  conn.execute(
      "INSERT INTO map_corner_table "
      "SELECT OBJECT_CONSTRUCT()::MAP(VARCHAR, VARCHAR)");
  conn.execute(
      "INSERT INTO map_corner_table "
      "SELECT OBJECT_CONSTRUCT('a', 'b')::MAP(VARCHAR, VARCHAR)");
  conn.execute(
      "INSERT INTO map_corner_table "
      "SELECT NULL::MAP(VARCHAR, VARCHAR)");

  // When Query "SELECT * FROM <table>" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM map_corner_table"), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain the inserted corner case map values
  int row_count = 0;
  int null_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    auto value = get_data_optional<SQL_C_CHAR>(stmt, 1);
    if (!value.has_value()) {
      null_count++;
    }
    row_count++;
  }

  CHECK(row_count == 3);
  CHECK(null_count == 1);
}

// ============================================================================
// MULTIPLE CHUNKS DOWNLOADING
// ============================================================================

TEST_CASE("should download map data in multiple chunks", "[datatype][map][large_result_set]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting 10000 MAP rows from GENERATOR is executed
  const auto stmt = conn.createStatement();
  const char* sql =
      "SELECT OBJECT_CONSTRUCT("
      "'id', TO_VARCHAR(seq8()), "
      "'val', TO_VARCHAR(seq8() * 10)"
      ")::MAP(VARCHAR, VARCHAR) "
      "FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then there are 10000 rows returned
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    // And All returned values should be valid map representations
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    REQUIRE(!value.empty());
    CHECK(json_contains_key(value, "id"));
    CHECK(json_contains_key(value, "val"));
    row_count++;
  }

  CHECK(row_count == 10000);
}
