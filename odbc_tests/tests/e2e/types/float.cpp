// FLOAT datatype ODBC E2E tests
// Based on: tests/definitions/shared/types/float.feature

#include <algorithm>
#include <cmath>
#include <string>

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"

// Old driver returns "INFINITY"/"-INFINITY", new driver returns "inf"/"-inf"
static bool is_positive_infinity_str(const std::string& s) {
  std::string lower = s;
  std::transform(lower.begin(), lower.end(), lower.begin(), ::tolower);
  return lower == "inf" || lower == "infinity";
}

static bool is_negative_infinity_str(const std::string& s) {
  std::string lower = s;
  std::transform(lower.begin(), lower.end(), lower.begin(), ::tolower);
  return lower == "-inf" || lower == "-infinity";
}

// ============================================================================
// TYPE CASTING
// ============================================================================

TEST_CASE("should cast float values to appropriate type for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0.0::<type>, 123.456::<type>, 1.23e10::<type>, 'NaN'::<type>, 'inf'::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT 0.0::FLOAT, 123.456::FLOAT, 1.23e10::FLOAT, 'NaN'::FLOAT, 'inf'::FLOAT");

  // Then All values should be returned as appropriate type
  double val1 = get_data<SQL_C_DOUBLE>(stmt, 1);
  double val2 = get_data<SQL_C_DOUBLE>(stmt, 2);
  double val3 = get_data<SQL_C_DOUBLE>(stmt, 3);
  double val4 = get_data<SQL_C_DOUBLE>(stmt, 4);
  double val5 = get_data<SQL_C_DOUBLE>(stmt, 5);

  // And Regular values should have approximately 15 decimal digits precision
  CHECK(val1 == 0.0);
  CHECK(val2 == Catch::Approx(123.456));
  CHECK(val3 == Catch::Approx(1.23e10));

  // And NaN and inf values should be identified correctly
  CHECK(std::isnan(val4));
  CHECK(std::isinf(val5));
  CHECK(val5 > 0);
}

// ============================================================================
// SELECT WITH LITERALS (no tables)
// ============================================================================

TEST_CASE("should select float literals for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0.0::<type>, 1.0::<type>, -1.0::<type>, 123.456::<type>, -123.456::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT 0.0::FLOAT, 1.0::FLOAT, -1.0::FLOAT, 123.456::FLOAT, -123.456::FLOAT");

  // Then Result should contain floats [0.0, 1.0, -1.0, 123.456, -123.456]
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 0.0);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 1.0);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == -1.0);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == Catch::Approx(123.456));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 5) == Catch::Approx(-123.456));
}

TEST_CASE("should handle special float values from literals for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT 'NaN'::FLOAT, 'inf'::FLOAT, '-inf'::FLOAT");

  // Then Result should contain [NaN, positive_infinity, negative_infinity]
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "NaN");
  CHECK(is_positive_infinity_str(get_data<SQL_C_CHAR>(stmt, 2)));
  CHECK(is_negative_infinity_str(get_data<SQL_C_CHAR>(stmt, 3)));
}

TEST_CASE("should handle float boundary values from literals for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 1.7976931348623157e308::<type>, -1.7976931348623157e308::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT 1.7976931348623157e308::FLOAT, -1.7976931348623157e308::FLOAT");

  // Then Result should contain floats [1.7976931348623157e308, -1.7976931348623157e308]
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(1.7976931348623157e308));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == Catch::Approx(-1.7976931348623157e308));

  // When Query "SELECT 2.2250738585072014e-308::<type>, 5e-324::<type>" is executed
  auto stmt2 = conn.execute_fetch("SELECT 2.2250738585072014e-308::FLOAT, 5e-324::FLOAT");

  // Then Result should contain floats [2.2250738585072014e-308, approximately 5e-324]
  CHECK(get_data<SQL_C_DOUBLE>(stmt2, 1) == Catch::Approx(2.2250738585072014e-308));
  double subnormal = get_data<SQL_C_DOUBLE>(stmt2, 2);
  CHECK(subnormal > 0.0);
  CHECK(subnormal < 1e-300);

  // When Query "SELECT 123456789012345.0::<type>, 1234567890123456.0::<type>" is executed
  auto stmt3 = conn.execute_fetch("SELECT 123456789012345.0::FLOAT, 1234567890123456.0::FLOAT");

  // Then Result should verify precision around 15 decimal digits
  CHECK(get_data<SQL_C_DOUBLE>(stmt3, 1) == Catch::Approx(123456789012345.0));
  CHECK(get_data<SQL_C_DOUBLE>(stmt3, 2) == Catch::Approx(1234567890123456.0));
}

TEST_CASE("should handle NULL values from literals for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::<type>, 42.5::<type>, NULL::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT, 42.5::FLOAT, NULL::FLOAT");

  // Then Result should contain [NULL, 42.5, NULL]
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 2) == std::optional<double>(42.5));
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 3) == std::nullopt);
}

TEST_CASE("should download large result set with multiple chunks from GENERATOR for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT seq8()::FLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY 1";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 50000 rows with all values returned as appropriate float type
  int row_count = 0;
  double expected = 0.0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    double val = 0.0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &val, sizeof(val), NULL);
    CHECK_ODBC(ret, stmt);

    REQUIRE(val == Catch::Approx(expected));
    expected += 1.0;
    row_count++;
  }

  REQUIRE(row_count == 50000);
}

// ============================================================================
// TABLE OPERATIONS
// ============================================================================

TEST_CASE("should select floats from table for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists with values [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
  conn.execute("CREATE TABLE float_table (col FLOAT)");
  conn.execute("INSERT INTO float_table VALUES (0.0), (123.456), (-789.012), (1.23e5), (-9.87e-3)");

  // When Query "SELECT * FROM float_table" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM float_table");

  // Then Result should contain floats [0.0, 123.456, -789.012, 123000.0, -0.00987]
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 0.0);

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(123.456));

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(-789.012));

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(123000.0));

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(-0.00987));
}

TEST_CASE("should handle special float values from table for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists with values [NaN, inf, -inf, 42.0, -42.0]
  conn.execute("CREATE TABLE float_special (col FLOAT)");
  conn.execute("INSERT INTO float_special SELECT 'NaN'::FLOAT");
  conn.execute("INSERT INTO float_special SELECT 'inf'::FLOAT");
  conn.execute("INSERT INTO float_special SELECT '-inf'::FLOAT");
  conn.execute("INSERT INTO float_special VALUES (42.0), (-42.0)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM float_special");

  // Then Result should contain [NaN, positive_infinity, negative_infinity, 42.0, -42.0]
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "NaN");

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(is_positive_infinity_str(get_data<SQL_C_CHAR>(stmt, 1)));

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(is_negative_infinity_str(get_data<SQL_C_CHAR>(stmt, 1)));

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 42.0);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == -42.0);
}

TEST_CASE("should handle float boundary values from table for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists with boundary values [1.7976931348623157e308, -1.7976931348623157e308,
  // 2.2250738585072014e-308, 5e-324, 123456789012345.0]
  conn.execute("CREATE TABLE float_boundary (col FLOAT)");
  conn.execute("INSERT INTO float_boundary VALUES (1.7976931348623157e308)");
  conn.execute("INSERT INTO float_boundary VALUES (-1.7976931348623157e308)");
  conn.execute("INSERT INTO float_boundary VALUES (2.2250738585072014e-308)");
  conn.execute("INSERT INTO float_boundary VALUES (5e-324)");
  conn.execute("INSERT INTO float_boundary VALUES (123456789012345.0)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM float_boundary");

  // Then Result should contain maximum, minimum, and precision boundary values preserved within float precision limits
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(1.7976931348623157e308));

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(-1.7976931348623157e308));

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(2.2250738585072014e-308));

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  double subnormal = get_data<SQL_C_DOUBLE>(stmt, 1);
  CHECK(subnormal > 0.0);
  CHECK(subnormal < 1e-300);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(123456789012345.0));
}

TEST_CASE("should handle NULL values from table for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists with values [NULL, 123.456, NULL, -789.012]
  conn.execute("CREATE TABLE float_null (col FLOAT)");
  conn.execute("INSERT INTO float_null VALUES (NULL), (123.456), (NULL), (-789.012)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM float_null");

  // Then Result should contain [NULL, 123.456, NULL, -789.012]
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 1) == std::nullopt);

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(123.456));

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 1) == std::nullopt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(-789.012));
}

TEST_CASE("should select large result set from table for float and synonyms", "[float]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists with 50000 sequential values
  conn.execute("CREATE TABLE float_large (col FLOAT)");
  conn.execute("INSERT INTO float_large SELECT seq8()::FLOAT FROM TABLE(GENERATOR(ROWCOUNT => 50000))");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM float_large ORDER BY col", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 50000 rows with all values returned as appropriate float type
  int row_count = 0;
  double expected = 0.0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    double val = 0.0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &val, sizeof(val), NULL);
    CHECK_ODBC(ret, stmt);

    REQUIRE(val == Catch::Approx(expected));
    expected += 1.0;
    row_count++;
  }

  REQUIRE(row_count == 50000);
}
