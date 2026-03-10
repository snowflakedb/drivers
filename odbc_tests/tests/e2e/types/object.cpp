// OBJECT datatype ODBC tests
// Based on: tests/definitions/shared/types/object.feature
//
// Snowflake OBJECT is a semi-structured data type storing key-value pairs.
// Keys are always strings; values can be any Snowflake type.
// Constructed via OBJECT_CONSTRUCT('key1', val1, 'key2', val2, ...).
// ODBC returns OBJECT values as SQL_C_CHAR (JSON string representation).
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

TEST_CASE("should cast object values to appropriate type", "[datatype][object]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT OBJECT_CONSTRUCT('name', 'Alice', 'age', 30)::OBJECT" is executed
  const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('name', 'Alice', 'age', 30)::OBJECT");

  // Then Value should be returned as appropriate type
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  REQUIRE(!value.empty());

  // And Value should contain key 'name' with value 'Alice' and key 'age' with value 30
  CHECK(json_contains_key(value, "name"));
  CHECK(json_contains_key(value, "age"));
  CHECK(value.find("\"Alice\"") != std::string::npos);
  CHECK(value.find("30") != std::string::npos);
}

// ============================================================================
// SIMPLE SELECTS - LITERALS
// ============================================================================

TEST_CASE("should select hardcoded object literals", "[datatype][object]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT OBJECT_CONSTRUCT('key1', 'value1', 'key2', 42)" is executed
  const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key1', 'value1', 'key2', 42)");

  // Then Result should contain an object with keys [key1, key2]
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  REQUIRE(!value.empty());
  CHECK(json_contains_key(value, "key1"));
  CHECK(json_contains_key(value, "key2"));

  // And Object values should be key1='value1' and key2=42
  CHECK(value.find("\"value1\"") != std::string::npos);
  CHECK(value.find("42") != std::string::npos);
}

TEST_CASE("should select object corner case values from literals", "[datatype][object]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Queries selecting corner case object literals are executed
  // Then Results should contain expected corner case object values

  // Empty object
  {
    const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT()");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(value == "{}");
  }

  // Object with NULL value — Snowflake omits NULL-valued keys by default
  {
    const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key', NULL)");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(value == "{}");
  }

  // Nested object
  {
    const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('outer', OBJECT_CONSTRUCT('inner', 'value'))");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(json_contains_key(value, "outer"));
    CHECK(value.find("\"inner\"") != std::string::npos);
    CHECK(value.find("\"value\"") != std::string::npos);
  }

  // Object with boolean
  {
    const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('flag', TRUE)");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(json_contains_key(value, "flag"));
    CHECK(value.find("true") != std::string::npos);
  }

  // Object with numeric types
  {
    const auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('int', 1, 'float', 1.5)");
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(json_contains_key(value, "int"));
    CHECK(json_contains_key(value, "float"));
  }

  // NULL::OBJECT
  {
    const auto stmt = conn.execute_fetch("SELECT NULL::OBJECT");
    auto value = get_data_optional<SQL_C_CHAR>(stmt, 1);
    CHECK(value == std::nullopt);
  }
}

// ============================================================================
// SELECT FROM TABLE
// ============================================================================

TEST_CASE("should select object values from table", "[datatype][object]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with VARIANT column is created
  conn.execute("CREATE TABLE object_table (col VARIANT)");

  // And The table is populated with object values
  conn.execute(
      "INSERT INTO object_table SELECT PARSE_JSON(column1) FROM VALUES "
      "('{\"name\": \"Alice\", \"age\": 30}'), "
      "('{\"name\": \"Bob\", \"age\": 25}')");

  // When Query "SELECT * FROM <table>" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM object_table"), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain the inserted object values
  int row_count = 0;
  bool found_alice = false;
  bool found_bob = false;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    if (value.find("\"Alice\"") != std::string::npos) found_alice = true;
    if (value.find("\"Bob\"") != std::string::npos) found_bob = true;
    row_count++;
  }

  CHECK(row_count == 2);
  CHECK(found_alice);
  CHECK(found_bob);
}

TEST_CASE("should select object corner case values from table", "[datatype][object]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with VARIANT column is created
  conn.execute("CREATE TABLE object_corner_table (col VARIANT)");

  // And The table is populated with corner case object values
  conn.execute(
      "INSERT INTO object_corner_table SELECT PARSE_JSON(column1) FROM VALUES "
      "('{}'), "
      "('{\"nested\": {\"key\": \"value\"}}'), "
      "('{\"str\": \"hello\", \"num\": 42, \"bool\": true}')");
  conn.execute("INSERT INTO object_corner_table VALUES (NULL)");

  // When Query "SELECT * FROM <table>" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM object_corner_table"), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain the inserted corner case object values
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

TEST_CASE("should download object data in multiple chunks", "[datatype][object][large_result_set]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting 10000 OBJECT_CONSTRUCT rows from GENERATOR is executed
  const auto stmt = conn.createStatement();
  const char* sql =
      "SELECT OBJECT_CONSTRUCT('id', seq8(), 'value', TO_VARCHAR(seq8())) "
      "FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then there are 10000 rows returned
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    // And All returned values should be valid object representations
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    REQUIRE(!value.empty());
    CHECK(json_contains_key(value, "id"));
    CHECK(json_contains_key(value, "value"));
    row_count++;
  }

  CHECK(row_count == 10000);
}
