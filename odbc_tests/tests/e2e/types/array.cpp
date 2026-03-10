// ARRAY datatype ODBC tests
// Based on: tests/definitions/shared/types/array.feature
//
// Snowflake ARRAY is a semi-structured data type storing ordered lists of values.
// Values can be any Snowflake type including nested ARRAYs and OBJECTs.
// Constructed via ARRAY_CONSTRUCT(val1, val2, ...).
// ODBC returns ARRAY values as SQL_C_CHAR (JSON string representation).
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

// ============================================================================
// TYPE CASTING
// ============================================================================

TEST_CASE("should cast array values to appropriate type", "[datatype][array]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ARRAY_CONSTRUCT(1, 2, 3)::ARRAY" is executed
  const auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1, 2, 3)::ARRAY");

  // Then Value should be returned as appropriate type
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  REQUIRE(!value.empty());

  // And Value should be an array containing elements [1, 2, 3]
  CHECK(value.find("1") != std::string::npos);
  CHECK(value.find("2") != std::string::npos);
  CHECK(value.find("3") != std::string::npos);
}

// ============================================================================
// SIMPLE SELECTS - LITERALS
// ============================================================================

TEST_CASE("should select hardcoded array literals", "[datatype][array]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ARRAY_CONSTRUCT('a', 'b', 'c')" is executed
  const auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT('a', 'b', 'c')");

  // Then Result should contain an array with 3 elements
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  REQUIRE(!value.empty());

  // And Array values should be ['a', 'b', 'c']
  CHECK(value.find("\"a\"") != std::string::npos);
  CHECK(value.find("\"b\"") != std::string::npos);
  CHECK(value.find("\"c\"") != std::string::npos);
}

TEST_CASE("should select array corner case values from literals", "[datatype][array]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Queries selecting corner case array literals are executed
  // Then Results should contain expected corner case array values

  // Empty array
  {
    const auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT()");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(value == "[]");
  }

  // Single element array
  {
    const auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(42)");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(value.find("42") != std::string::npos);
  }

  // Nested array
  {
    const auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(1, 2), ARRAY_CONSTRUCT(3, 4))");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    REQUIRE(!value.empty());
    CHECK(value.find("[") != std::string::npos);
  }

  // Mixed types
  {
    const auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1, 'two', TRUE)");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(value.find("1") != std::string::npos);
    CHECK(value.find("\"two\"") != std::string::npos);
    CHECK(value.find("true") != std::string::npos);
  }

  // NULL::ARRAY
  {
    const auto stmt = conn.execute_fetch("SELECT NULL::ARRAY");
    auto value = get_data_optional<SQL_C_CHAR>(stmt, 1);
    CHECK(value == std::nullopt);
  }
}

// ============================================================================
// SELECT FROM TABLE
// ============================================================================

TEST_CASE("should select array values from table", "[datatype][array]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with VARIANT column is created
  conn.execute("CREATE TABLE array_table (col VARIANT)");

  // And The table is populated with array values
  conn.execute(
      "INSERT INTO array_table SELECT PARSE_JSON(column1) FROM VALUES "
      "('[1, 2, 3]'), "
      "('[4, 5, 6]')");

  // When Query "SELECT * FROM <table>" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM array_table"), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain the inserted array values
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

TEST_CASE("should select array corner case values from table", "[datatype][array]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with VARIANT column is created
  conn.execute("CREATE TABLE array_corner_table (col VARIANT)");

  // And The table is populated with corner case array values
  conn.execute(
      "INSERT INTO array_corner_table SELECT PARSE_JSON(column1) FROM VALUES "
      "('[]'), "
      "('[[1,2],[3,4]]'), "
      "('[1, \"two\", true]')");
  conn.execute("INSERT INTO array_corner_table VALUES (NULL)");

  // When Query "SELECT * FROM <table>" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM array_corner_table"), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain the inserted corner case array values
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

  CHECK(row_count == 4);
  CHECK(null_count == 1);
}

// ============================================================================
// MULTIPLE CHUNKS DOWNLOADING
// ============================================================================

TEST_CASE("should download array data in multiple chunks", "[datatype][array][large_result_set]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting 10000 ARRAY_CONSTRUCT rows from GENERATOR is executed
  const auto stmt = conn.createStatement();
  const char* sql =
      "SELECT ARRAY_CONSTRUCT(seq8(), seq8() * 2, seq8() * 3) "
      "FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then there are 10000 rows returned
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    // And All returned values should be valid array representations
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    REQUIRE(!value.empty());
    CHECK(value.front() == '[');
    CHECK(value.back() == ']');
    row_count++;
  }

  CHECK(row_count == 10000);
}
