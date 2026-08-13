#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cmath>
#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

static picojson::value parse_json_text(const std::string& json_text);
static void check_json_equals(const std::string& actual, const std::string& expected);
static std::string get_data_char_full(const StatementHandleWrapper& stmt, SQLUSMALLINT col);

// ============================================================================
// TYPE CASTING (shared @odbc_e2e)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should cast vector values to appropriate type",
                 "[datatype][vector][conversion][c_char]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
  auto stmt = conn.execute_fetch("SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)");

  // Then All values should be returned as appropriate type
  auto col1 = get_data<SQL_C_CHAR>(stmt, 1);
  auto col2 = get_data<SQL_C_CHAR>(stmt, 2);

  auto json1 = parse_json_text(col1);
  auto json2 = parse_json_text(col2);

  CHECK(json1.is<picojson::array>());
  CHECK(json1.get<picojson::array>().size() == 3);
  CHECK(json2.is<picojson::array>());
  CHECK(json2.get<picojson::array>().size() == 3);
}

// ============================================================================
// SELECT WITH LITERALS (shared @odbc_e2e)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should select <subtype> vector literal", "[datatype][vector]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
  auto stmt1 = conn.execute_fetch("SELECT [1, 3, -5]::VECTOR(INT, 3)");
  auto stmt2 = conn.execute_fetch("SELECT [40, 1234567]::VECTOR(INT, 2)");
  auto stmt3 = conn.execute_fetch("SELECT [1.8, -3.4, 6.7, 0.0, 2.3]::VECTOR(FLOAT, 5)");

  // Then Result should contain <subtype> vector <expected_value>
  check_json_equals(get_data<SQL_C_CHAR>(stmt1, 1), "[1,3,-5]");
  check_json_equals(get_data<SQL_C_CHAR>(stmt2, 1), "[40,1234567]");
  auto json3 = parse_json_text(get_data<SQL_C_CHAR>(stmt3, 1));
  REQUIRE(json3.is<picojson::array>());
  const auto& arr = json3.get<picojson::array>();
  REQUIRE(arr.size() == 5);
  CHECK(std::abs(arr[0].get<double>() - 1.8) < 1e-5);
  CHECK(std::abs(arr[1].get<double>() - (-3.4)) < 1e-5);
  CHECK(std::abs(arr[2].get<double>() - 6.7) < 1e-5);
  CHECK(std::abs(arr[3].get<double>()) < 1e-10);
  CHECK(std::abs(arr[4].get<double>() - 2.3) < 1e-5);
}

// ============================================================================
// NULL HANDLING (shared @odbc_e2e)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should handle NULL vector values", "[datatype][vector]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)" is executed
  auto stmt = conn.execute_fetch("SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)");

  // Then Result should contain [[1, 2, 3], NULL, NULL]
  auto col1 = get_data_optional<SQL_C_CHAR>(stmt, 1);
  REQUIRE(col1.has_value());
  check_json_equals(*col1, "[1,2,3]");
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 2).has_value());
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 3).has_value());
}

// ============================================================================
// BOUNDARY VALUES (shared @odbc_e2e)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should select <subtype> vector boundary values", "[datatype][vector]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
  auto stmt1 = conn.execute_fetch("SELECT [-2147483648, 2147483647, 0]::VECTOR(INT, 3)");
  auto stmt2 = conn.execute_fetch("SELECT [3.4028235e38, -3.4028235e38, 0.0]::VECTOR(FLOAT, 3)");

  // Then Result should preserve <subtype> boundary values
  auto json1 = parse_json_text(get_data<SQL_C_CHAR>(stmt1, 1));
  REQUIRE(json1.is<picojson::array>());
  const auto& iarr = json1.get<picojson::array>();
  REQUIRE(iarr.size() == 3);
  CHECK(iarr[0].get<double>() == -2147483648.0);
  CHECK(iarr[1].get<double>() == 2147483647.0);
  CHECK(iarr[2].get<double>() == 0.0);

  auto json2 = parse_json_text(get_data<SQL_C_CHAR>(stmt2, 1));
  REQUIRE(json2.is<picojson::array>());
  const auto& farr = json2.get<picojson::array>();
  REQUIRE(farr.size() == 3);
  CHECK(farr[0].get<double>() > 3.4e38);
  CHECK(farr[1].get<double>() < -3.4e38);
  CHECK(std::abs(farr[2].get<double>()) < 1e-10);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should preserve FLOAT smallest-normal", "[datatype][vector]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When Query selects a VECTOR(FLOAT, ...) containing FLOAT32_SMALLEST_NORMAL
  auto stmt = conn.execute_fetch("SELECT [1.1754944e-38]::VECTOR(FLOAT, 1)");

  // Then the smallest-normal value must not underflow to zero
  auto json = parse_json_text(get_data<SQL_C_CHAR>(stmt, 1));
  REQUIRE(json.is<picojson::array>());
  const auto& arr = json.get<picojson::array>();
  REQUIRE(arr.size() == 1);
  CHECK(arr[0].get<double>() > 0.0);
}

// ============================================================================
// MAX DIMENSION (shared @odbc_e2e)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should select max-dimension vector", "[datatype][vector]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting 4096-element float vector is executed
  static constexpr int kMaxDimension = 4096;
  std::string values;
  values.reserve(static_cast<size_t>(kMaxDimension) * 6);
  for (int i = 0; i < kMaxDimension; ++i) {
    if (i > 0) {
      values += ',';
    }
    values += std::to_string(i);
  }
  auto stmt = conn.execute_fetch("SELECT [" + values + "]::VECTOR(FLOAT, " + std::to_string(kMaxDimension) + ")");

  // Then Result should be a valid 4096-element float vector
  auto json = parse_json_text(get_data_char_full(stmt, 1));
  REQUIRE(json.is<picojson::array>());
  CHECK(json.get<picojson::array>().size() == kMaxDimension);
}

// ============================================================================
// TABLE OPERATIONS (shared @odbc_e2e)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should select vector values from table", "[datatype][vector]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in

  // And Table with VECTOR(INT, 3) and VECTOR(FLOAT, 5) columns exists with values
  conn.execute(
      "CREATE OR REPLACE TEMPORARY TABLE vector_table "
      "(id INT, iv VECTOR(INT, 3), fv VECTOR(FLOAT, 5))");
  conn.execute(
      "INSERT INTO vector_table "
      "SELECT 1, [1, 2, 3]::VECTOR(INT, 3), [1.0, 2.0, 3.0, 4.0, 5.0]::VECTOR(FLOAT, 5) "
      "UNION ALL SELECT 2, [-1, 0, 1]::VECTOR(INT, 3), [0.1, 0.2, 0.3, 0.4, 0.5]::VECTOR(FLOAT, 5)");

  // When Query "SELECT * FROM <table> ORDER BY id" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT iv, fv FROM vector_table ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain the expected integer and float vector values
  for (SQLUSMALLINT col = 1; col <= 2; ++col) {
    INFO("column " << col);
    SQLSMALLINT data_type = 0;
    SQLULEN column_size = 0;
    SQLSMALLINT decimal_digits = 0;
    ret =
        SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, &column_size, &decimal_digits, nullptr);
    REQUIRE_ODBC(ret, stmt);
    CHECK(data_type == SQL_VARCHAR);
    // Table VECTOR columns report Snowflake's charLength (commonly 16 MiB), which
    // may differ from the session max VARCHAR used for unbound literal selects.
    CHECK(column_size > 0);
    CHECK(decimal_digits == 0);
  }

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), "[1,2,3]");
  auto fv1 = parse_json_text(get_data<SQL_C_CHAR>(stmt, 2));
  REQUIRE(fv1.is<picojson::array>());
  CHECK(fv1.get<picojson::array>().size() == 5);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should handle NULL vector values from table", "[datatype][vector]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in

  // And Table with VECTOR columns exist containing NULLs and values
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE vector_null_table (id INT, v VECTOR(INT, 3))");
  conn.execute(
      "INSERT INTO vector_null_table "
      "SELECT 1, NULL::VECTOR(INT, 3) "
      "UNION ALL SELECT 2, [4, 5, 6]::VECTOR(INT, 3) "
      "UNION ALL SELECT 3, NULL::VECTOR(INT, 3)");

  // When Query "SELECT * FROM <table> ORDER BY id" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT v FROM vector_null_table ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain both vector values and NULLs
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  auto val = get_data_optional<SQL_C_CHAR>(stmt, 1);
  REQUIRE(val.has_value());
  check_json_equals(*val, "[4,5,6]");

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());
}

// ============================================================================
// MULTIPLE CHUNKS DOWNLOADING (shared @odbc_e2e)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should download vector data in multiple chunks",
                 "[datatype][vector][large_result_set]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When Query generating 20000 integer vectors is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                sqlchar("SELECT [seq4() % 100, seq4() % 200, seq4() % 300]::VECTOR(INT, 3) "
                                        "FROM TABLE(GENERATOR(ROWCOUNT => 20000))"),
                                SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then All 20000 rows should be fetched with valid 3-element integer vectors
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    auto val = get_data<SQL_C_CHAR>(stmt, 1);
    auto json = parse_json_text(val);
    REQUIRE(json.is<picojson::array>());
    CHECK(json.get<picojson::array>().size() == 3);
    ++row_count;
  }
  CHECK(row_count == 20000);
}

// ============================================================================
// Static helpers
// ============================================================================

static picojson::value parse_json_text(const std::string& json_text) {
  picojson::value json;
  const auto error = picojson::parse(json, json_text);
  REQUIRE(error.empty());
  return json;
}

static void check_json_equals(const std::string& actual, const std::string& expected) {
  const auto actual_json = parse_json_text(actual);
  const auto expected_json = parse_json_text(expected);
  REQUIRE(actual_json.serialize() == expected_json.serialize());
}

// Fetch a full SQL_C_CHAR value via successive SQLGetData calls. Needed when the
// JSON payload exceeds the fixed 8 KiB buffer used by get_data<SQL_C_CHAR>.
static std::string get_data_char_full(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  std::string result;
  char buffer[4096];
  for (;;) {
    std::memset(buffer, 0xFF, sizeof(buffer));
    SQLLEN indicator = -1;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
    if (indicator == SQL_NULL_DATA) {
      REQUIRE(ret == SQL_SUCCESS);
      return {};
    }
    REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
    const size_t written = (ret == SQL_SUCCESS_WITH_INFO) ? (sizeof(buffer) - 1) : static_cast<size_t>(indicator);
    result.append(buffer, written);
    if (ret == SQL_SUCCESS) {
      break;
    }
  }
  return result;
}
