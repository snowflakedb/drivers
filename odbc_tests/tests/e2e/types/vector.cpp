// VECTOR type ODBC E2E tests
// Based on: tests/definitions/shared/types/vector.feature
//
// Snowflake VECTOR type stores fixed-size arrays of numeric values.
// Subtypes: INT (integer) and FLOAT (32-bit floating-point).
// ODBC returns vector values as JSON-serialized strings via SQL_C_CHAR.
// Maximum dimension: 4096.
#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cmath>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// =============================================================================
// Helpers
// =============================================================================

/// Retrieve a potentially large SQL_C_CHAR column via chunked SQLGetData.
/// The default get_data<SQL_C_CHAR> uses an 8 KiB buffer, which is too small
/// for high-dimension vectors (e.g. VECTOR(FLOAT, 4096) can exceed 60 KiB).
static std::string get_large_char_data(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  std::string result;
  char buffer[8192];
  SQLLEN indicator;
  while (true) {
    SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
    if (ret == SQL_NO_DATA) break;
    REQUIRE(indicator != SQL_NULL_DATA);
    REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
    result.append(buffer);
    if (ret == SQL_SUCCESS) break;
  }
  return result;
}

/// Parse a JSON string and assert it is an array.
static picojson::array parse_json_array(const std::string& json_text) {
  picojson::value json;
  const auto error = picojson::parse(json, json_text);
  REQUIRE(error.empty());
  REQUIRE(json.is<picojson::array>());
  return json.get<picojson::array>();
}

/// Assert that a JSON array represents the expected integer vector.
static void check_int_vector(const std::string& json_text, const std::vector<int64_t>& expected) {
  auto arr = parse_json_array(json_text);
  REQUIRE(arr.size() == expected.size());
  for (size_t i = 0; i < expected.size(); ++i) {
    REQUIRE(arr[i].is<double>());
    CHECK(static_cast<int64_t>(arr[i].get<double>()) == expected[i]);
  }
}

/// Assert that a JSON array represents the expected float vector within tolerance.
/// Uses relative tolerance suitable for 32-bit float precision.
static void check_float_vector(const std::string& json_text, const std::vector<double>& expected,
                               double rel_tol = 1e-6) {
  auto arr = parse_json_array(json_text);
  REQUIRE(arr.size() == expected.size());
  for (size_t i = 0; i < expected.size(); ++i) {
    REQUIRE(arr[i].is<double>());
    double actual = arr[i].get<double>();
    if (expected[i] == 0.0) {
      CHECK(std::abs(actual) < 1e-6);
    } else {
      CHECK(std::abs(actual - expected[i]) <= std::abs(expected[i]) * rel_tol);
    }
  }
}

// =============================================================================
// TYPE CASTING
// =============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should cast vector values to appropriate type", "[vector]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
  auto stmt = conn.execute_fetch("SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)");

  // Then All values should be returned as appropriate type
  check_int_vector(get_data<SQL_C_CHAR>(stmt, 1), {1, 2, 3});
  check_float_vector(get_data<SQL_C_CHAR>(stmt, 2), {1.5, 2.5, 3.5});
}

// =============================================================================
// SELECT WITH LITERALS
// =============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should select subtype vector literal", "[vector]") {
  // Given Snowflake client is logged in
  Connection conn;

  SECTION("INT-3d") {
    // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
    auto stmt = conn.execute_fetch("SELECT [1, 3, -5]::VECTOR(INT, 3)");

    // Then Result should contain <subtype> vector <expected_value>
    check_int_vector(get_data<SQL_C_CHAR>(stmt, 1), {1, 3, -5});
  }

  SECTION("INT-2d") {
    // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
    auto stmt = conn.execute_fetch("SELECT [40, 1234567]::VECTOR(INT, 2)");

    // Then Result should contain <subtype> vector <expected_value>
    check_int_vector(get_data<SQL_C_CHAR>(stmt, 1), {40, 1234567});
  }

  SECTION("FLOAT-5d") {
    // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
    auto stmt = conn.execute_fetch("SELECT [1.8, -3.4, 6.7, 0.0, 2.3]::VECTOR(FLOAT, 5)");

    // Then Result should contain <subtype> vector <expected_value>
    check_float_vector(get_data<SQL_C_CHAR>(stmt, 1), {1.8, -3.4, 6.7, 0.0, 2.3});
  }
}

// =============================================================================
// SPECIAL VALUES
// =============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should select vector special values", "[vector]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting special vector values is executed
  auto null_stmt = conn.execute_fetch(
      "SELECT [1, 2, 3]::VECTOR(INT, 3), "
      "NULL::VECTOR(INT, 3), "
      "NULL::VECTOR(FLOAT, 3)");

  constexpr int max_dim = 4096;
  std::string values;
  for (int i = 0; i < max_dim; ++i) {
    if (i > 0) values += ", ";
    values += std::to_string(i);
  }
  std::string sql = "SELECT [" + values + "]::VECTOR(FLOAT, " + std::to_string(max_dim) + ")";
  auto max_dim_stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(max_dim_stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, max_dim_stmt);
  ret = SQLFetch(max_dim_stmt.getHandle());
  REQUIRE_ODBC(ret, max_dim_stmt);

  // Then NULL vectors should return None and max-dimension vector should be valid
  check_int_vector(get_data<SQL_C_CHAR>(null_stmt, 1), {1, 2, 3});
  CHECK(!get_data_optional<SQL_C_CHAR>(null_stmt, 2).has_value());
  CHECK(!get_data_optional<SQL_C_CHAR>(null_stmt, 3).has_value());

  auto json_str = get_large_char_data(max_dim_stmt, 1);
  auto arr = parse_json_array(json_str);
  REQUIRE(arr.size() == max_dim);
  std::vector<double> expected(max_dim);
  for (int i = 0; i < max_dim; ++i)
    expected[i] = static_cast<double>(i);
  check_float_vector(json_str, expected);
}

// =============================================================================
// TABLE OPERATIONS
// =============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should select vector values from table", "[vector]") {
  // Given Snowflake client is logged in

  // And Table with VECTOR(INT, 3) and VECTOR(FLOAT, 5) columns exists with values
  conn.execute(
      "CREATE OR REPLACE TEMPORARY TABLE vector_table "
      "(id INT, int_vec VECTOR(INT, 3), float_vec VECTOR(FLOAT, 5))");
  conn.execute(
      "INSERT INTO vector_table "
      "SELECT 1, [1, 2, 3]::VECTOR(INT, 3), [1.1, 2.2, 3.3, 4.4, 5.5]::VECTOR(FLOAT, 5) "
      "UNION ALL "
      "SELECT 2, [10, 20, 30]::VECTOR(INT, 3), [10.5, 20.5, 30.5, 40.5, 50.5]::VECTOR(FLOAT, 5)");

  // When Query "SELECT * FROM <table> ORDER BY id" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM vector_table ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain the expected integer and float vector values
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  check_int_vector(get_data<SQL_C_CHAR>(stmt, 2), {1, 2, 3});
  check_float_vector(get_data<SQL_C_CHAR>(stmt, 3), {1.1, 2.2, 3.3, 4.4, 5.5});

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  check_int_vector(get_data<SQL_C_CHAR>(stmt, 2), {10, 20, 30});
  check_float_vector(get_data<SQL_C_CHAR>(stmt, 3), {10.5, 20.5, 30.5, 40.5, 50.5});
}

TEST_CASE_METHOD(ConnSchemaFixture, "should handle NULL vector values from table", "[vector]") {
  // Given Snowflake client is logged in

  // And Table with VECTOR columns exists containing NULLs and values
  conn.execute(
      "CREATE OR REPLACE TEMPORARY TABLE vector_null_table "
      "(id INT, int_vec VECTOR(INT, 3), float_vec VECTOR(FLOAT, 3))");
  conn.execute(
      "INSERT INTO vector_null_table "
      "SELECT 1, [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3) "
      "UNION ALL "
      "SELECT 2, NULL::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3) "
      "UNION ALL "
      "SELECT 3, NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)");

  // When Query "SELECT * FROM <table> ORDER BY id" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM vector_null_table ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain both vector values and NULLs
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  check_int_vector(get_data<SQL_C_CHAR>(stmt, 2), {1, 2, 3});
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 3).has_value());

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 2).has_value());
  check_float_vector(get_data<SQL_C_CHAR>(stmt, 3), {1.5, 2.5, 3.5});

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 2).has_value());
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 3).has_value());
}

// =============================================================================
// MULTIPLE CHUNKS DOWNLOADING
// =============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should download vector data in multiple chunks", "[vector][large_result_set]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query generating 20000 integer vectors is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                sqlchar("SELECT id, [id, id * 2, id * 3]::VECTOR(INT, 3) AS vec "
                                        "FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id "
                                        "FROM TABLE(GENERATOR(ROWCOUNT => 20000))) "
                                        "ORDER BY id"),
                                SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then All 20000 rows should be fetched with valid 3-element integer vectors
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);

    auto arr = parse_json_array(get_data<SQL_C_CHAR>(stmt, 2));
    REQUIRE(arr.size() == 3);

    REQUIRE(arr[0].is<double>());
    int64_t id = static_cast<int64_t>(arr[0].get<double>());
    CHECK(static_cast<int64_t>(arr[1].get<double>()) == id * 2);
    CHECK(static_cast<int64_t>(arr[2].get<double>()) == id * 3);

    row_count++;
  }
  REQUIRE(row_count == 20000);
}
