// REAL conversion to SQL_C_DEFAULT E2E tests
// SQL_C_DEFAULT for SQL_DOUBLE resolves to SQL_C_DOUBLE.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cmath>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "SchemaFixtures.hpp"
#include "TestTable.hpp"
#include "conversion_checks.hpp"
#include "test_setup.hpp"

/// Helper: fetch a column with SQL_C_DEFAULT into an SQLDOUBLE.
/// Per ODBC spec, SQL_C_DEFAULT for SQL_DOUBLE resolves to SQL_C_DOUBLE.
inline SQLDOUBLE get_data_default_as_double(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  SQLDOUBLE value = 0.0;
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == sizeof(SQLDOUBLE));
  return value;
}

// ============================================================================
// SQL_C_DEFAULT for FLOAT/DOUBLE columns
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "REAL default conversion - basic values", "[e2e][types][real]") {
  // Given A Snowflake connection

  // When FLOAT/DOUBLE values are inserted and fetched via SQL_C_DEFAULT
  conn.execute(
      "CREATE TEMPORARY TABLE test_real_default ("
      "  f1 FLOAT, "
      "  f2 DOUBLE, "
      "  f3 FLOAT)");
  conn.execute("INSERT INTO test_real_default VALUES (1.5, -2.75, 0.0)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_real_default");

  // Then The correct double values are returned
  CHECK(get_data_default_as_double(stmt, 1) == 1.5);
  CHECK(get_data_default_as_double(stmt, 2) == -2.75);
  CHECK(get_data_default_as_double(stmt, 3) == 0.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL default conversion - integer values stored as float", "[e2e][types][real]") {
  // Given A Snowflake connection

  // When Integer values stored as FLOAT are fetched via SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT 42::FLOAT, -100::FLOAT, 0::FLOAT, 1::FLOAT");

  // Then The correct double values are returned
  CHECK(get_data_default_as_double(stmt, 1) == 42.0);
  CHECK(get_data_default_as_double(stmt, 2) == -100.0);
  CHECK(get_data_default_as_double(stmt, 3) == 0.0);
  CHECK(get_data_default_as_double(stmt, 4) == 1.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL default conversion - extreme values near DBL_MAX", "[e2e][types][real]") {
  // JSON serializes DOUBLE as a decimal string capped at ~15 significant digits.
  // That both collapses the values below into each other and pushes the
  // DBL_MAX-magnitude ones outside the f64 range. The two tests that follow pin
  // what JSON does instead, so this coverage is not simply dropped there.
  SKIP_FOR_JSON_RESULT_SET("JSON truncates DOUBLE to ~15 significant digits, losing DBL_MAX precision");

  // Given A Snowflake connection

  // When Extreme values near DBL_MAX are inserted and fetched via SQL_C_DEFAULT
  conn.execute("CREATE TEMPORARY TABLE test_real_extreme (val DOUBLE)");
  conn.execute(
      "INSERT INTO test_real_extreme VALUES "
      "(1.7976931348623157e308), "
      "(1.7e308), "
      "(1.7976931348623151e308), "
      "(-1.7976931348623151e308), "
      "(-1.7e308), "
      "(-1.7976931348623157e308)");

  // ORDER BY makes the row order deterministic — without it the per-row
  // assertions below rely on an unspecified scan order.
  auto stmt = conn.execute_fetch("SELECT * FROM test_real_extreme ORDER BY val");

  // Then The correct extreme double values are returned, in ascending order
  const std::vector<double> expected = {-1.7976931348623157e308, -1.7976931348623151e308, -1.7e308, 1.7e308,
                                        1.7976931348623151e308,  1.7976931348623157e308};

  CHECK(get_data_default_as_double(stmt, 1) == expected[0]);
  for (size_t i = 1; i < expected.size(); ++i) {
    INFO("Row " << i);
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_SUCCESS);
    CHECK(get_data_default_as_double(stmt, 1) == expected[i]);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "REAL default conversion - JSON truncation overflows DBL_MAX-magnitude values to infinity",
                 "[e2e][types][real]") {
  RUN_ONLY_FOR_JSON_RESULT_SET("Arrow carries exact IEEE-754 doubles, so no truncation happens there");

  // Given A Snowflake connection

  // When DBL_MAX-magnitude DOUBLE values are fetched via SQL_C_DEFAULT over JSON
  //
  // The server caps the decimal representation at ~15 significant digits, so
  // 1.7976931348623157e308 (DBL_MAX) arrives as "1.79769313486232e+308". That
  // decimal is *larger* than DBL_MAX — it exceeds it by ~4.3e293, well over the
  // ~1.0e292 half-ULP rounding boundary — so a correctly-rounded parse
  // overflows and yields infinity. The legacy ODBC driver ends up in the same
  // place (picojson strtod, no ERANGE check), so this is not a UD regression,
  // but it does mean a stored DBL_MAX cannot be read back over JSON.
  auto stmt = conn.execute_fetch(
      "SELECT 1.7976931348623157e308::DOUBLE, "
      "       -1.7976931348623157e308::DOUBLE, "
      "       1.7976931348623151e308::DOUBLE, "
      "       1.7e308::DOUBLE");

  double max_pos = get_data_default_as_double(stmt, 1);
  double max_neg = get_data_default_as_double(stmt, 2);
  double near_max = get_data_default_as_double(stmt, 3);
  double representable = get_data_default_as_double(stmt, 4);

  // Then DBL_MAX-magnitude values overflow to signed infinity
  CHECK(std::isinf(max_pos));
  CHECK(max_pos > 0);
  CHECK(std::isinf(max_neg));
  CHECK(max_neg < 0);

  // And 1.7976931348623151e308 truncates to the same decimal as DBL_MAX, so the
  // two are indistinguishable over JSON
  CHECK(near_max == max_pos);

  // And a value exactly representable in 15 significant digits is unaffected
  CHECK(representable == 1.7e308);
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "REAL default conversion - extreme values within 15 significant digits round-trip exactly in JSON",
                 "[e2e][types][real]") {
  RUN_ONLY_FOR_JSON_RESULT_SET("pins the JSON precision boundary; Arrow is exact for all doubles");

  // Given A Snowflake connection

  // When Extreme DOUBLE values that fit in 15 significant digits are fetched
  // via SQL_C_DEFAULT over JSON
  conn.execute("CREATE TEMPORARY TABLE test_real_json_extreme (val DOUBLE)");
  conn.execute(
      "INSERT INTO test_real_json_extreme VALUES "
      "(1.79769313486231e308), "
      "(1.7e308), "
      "(-1.7e308), "
      "(-1.79769313486231e308)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_real_json_extreme ORDER BY val");

  // Then Each value survives the JSON round-trip exactly and stays finite
  const std::vector<double> expected = {-1.79769313486231e308, -1.7e308, 1.7e308, 1.79769313486231e308};

  CHECK(get_data_default_as_double(stmt, 1) == expected[0]);
  for (size_t i = 1; i < expected.size(); ++i) {
    INFO("Row " << i);
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_SUCCESS);
    CHECK(get_data_default_as_double(stmt, 1) == expected[i]);
  }

  // And none of them overflowed
  auto stmt_finite = conn.execute_fetch("SELECT MAX(val), MIN(val) FROM test_real_json_extreme");
  CHECK_FALSE(std::isinf(get_data_default_as_double(stmt_finite, 1)));
  CHECK_FALSE(std::isinf(get_data_default_as_double(stmt_finite, 2)));
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL default conversion - very small values", "[e2e][types][real]") {
  // Given A Snowflake connection

  // When Very small DOUBLE values are fetched via SQL_C_DEFAULT
  auto stmt = conn.execute_fetch(
      "SELECT 2.2250738585072014e-308::DOUBLE, "
      "       1e-307::DOUBLE, "
      "       -2.2250738585072014e-308::DOUBLE");

  double v1 = get_data_default_as_double(stmt, 1);
  double v2 = get_data_default_as_double(stmt, 2);
  double v3 = get_data_default_as_double(stmt, 3);

  // Then The correct small double values are returned
  CHECK(v1 > 0.0);
  CHECK(v1 < 1e-300);
  CHECK(v2 == 1e-307);
  CHECK(v3 < 0.0);
  CHECK(v3 > -1e-300);
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL default conversion - FLOAT, DOUBLE, REAL synonyms produce same result",
                 "[e2e][types][real]") {
  // Given A Snowflake connection

  // When Same value is stored in FLOAT, DOUBLE, REAL columns and fetched via SQL_C_DEFAULT
  conn.execute(
      "CREATE TEMPORARY TABLE test_real_synonyms ("
      "  f FLOAT, "
      "  d DOUBLE, "
      "  r REAL)");
  conn.execute("INSERT INTO test_real_synonyms VALUES (123.456, 123.456, 123.456)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_real_synonyms");

  double f = get_data_default_as_double(stmt, 1);
  double d = get_data_default_as_double(stmt, 2);
  double r = get_data_default_as_double(stmt, 3);

  // Then All three produce the same double value
  CHECK(f == d);
  CHECK(d == r);
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL SQL_C_DEFAULT matches explicit SQL_C_DOUBLE", "[e2e][types][real]") {
  // Given A Snowflake connection

  // When Values are fetched with SQL_C_DOUBLE and SQL_C_DEFAULT
  conn.execute("CREATE TEMPORARY TABLE test_real_default_vs_explicit (val DOUBLE)");
  conn.execute(
      "INSERT INTO test_real_default_vs_explicit VALUES "
      "(1.5), (-2.75), (0.0), (999999.999), (1.7976931348623157e308)");

  // Fetch with SQL_C_DOUBLE explicitly
  auto stmt_explicit = conn.execute_fetch("SELECT * FROM test_real_default_vs_explicit");
  std::vector<double> explicit_results;
  explicit_results.push_back(check_no_truncation<SQL_C_DOUBLE>(stmt_explicit, 1));
  for (int i = 1; i < 5; ++i) {
    SQLRETURN ret = SQLFetch(stmt_explicit.getHandle());
    CHECK(ret == SQL_SUCCESS);
    explicit_results.push_back(check_no_truncation<SQL_C_DOUBLE>(stmt_explicit, 1));
  }

  // Fetch with SQL_C_DEFAULT
  auto stmt_default = conn.execute_fetch("SELECT * FROM test_real_default_vs_explicit");
  std::vector<double> default_results;
  default_results.push_back(get_data_default_as_double(stmt_default, 1));
  for (int i = 1; i < 5; ++i) {
    SQLRETURN ret = SQLFetch(stmt_default.getHandle());
    CHECK(ret == SQL_SUCCESS);
    default_results.push_back(get_data_default_as_double(stmt_default, 1));
  }

  // Then Results match exactly
  for (size_t i = 0; i < explicit_results.size(); ++i) {
    INFO("Row " << i << ": SQL_C_DOUBLE=" << explicit_results[i] << " vs SQL_C_DEFAULT=" << default_results[i]);
    CHECK(explicit_results[i] == default_results[i]);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL default conversion - multiple rows", "[e2e][types][real]") {
  // Given A Snowflake connection

  // When Multiple DOUBLE rows are fetched via SQL_C_DEFAULT
  conn.execute("CREATE TEMPORARY TABLE test_real_multi (val DOUBLE)");
  conn.execute(
      "INSERT INTO test_real_multi VALUES "
      "(1.5), (-2.75), (0.0), (1e100), (-1e100)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_real_multi");

  std::vector<double> expected = {1.5, -2.75, 0.0, 1e100, -1e100};

  // Then Each row returns the correct double value
  CHECK(get_data_default_as_double(stmt, 1) == expected[0]);
  for (size_t i = 1; i < expected.size(); ++i) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_SUCCESS);
    INFO("Row " << i);
    CHECK(get_data_default_as_double(stmt, 1) == expected[i]);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL default conversion - fractional values", "[e2e][types][real]") {
  // Given A Snowflake connection

  // When Fractional FLOAT values are fetched via SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT 0.1::FLOAT, 0.5::FLOAT, 0.333333333::FLOAT");

  double v1 = get_data_default_as_double(stmt, 1);
  double v2 = get_data_default_as_double(stmt, 2);
  double v3 = get_data_default_as_double(stmt, 3);

  // Then The correct fractional double values are returned
  CHECK_THAT(v1, Catch::Matchers::WithinRel(0.1));
  CHECK(v2 == 0.5);
  CHECK_THAT(v3, Catch::Matchers::WithinRel(0.333333333));
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL zero is exactly zero", "[e2e][types][real]") {
  // Given A Snowflake connection

  // When Zero FLOAT value is fetched via SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT 0.0::FLOAT");

  double val = get_data_default_as_double(stmt, 1);

  // Then The value is exactly zero
  CHECK(val == 0.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL table column conversions", "[e2e][types][real]") {
  // Given A Snowflake connection

  // When A table with FLOAT, DOUBLE, REAL columns is queried
  TestTable table(conn, "test_real_conversions", "f FLOAT, d DOUBLE, r REAL", "(1.5, -2.75, 100.0)");

  // Then SQL_C_DOUBLE from all column types returns correct values
  {
    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == 1.5);
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 2) == -2.75);
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 3) == 100.0);
  }

  // And SQL_C_LONG truncates fractional with 01S07
  {
    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 1) == 1);
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 2) == -2);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 3) == 100);
  }

  // And SQL_C_CHAR returns string representation
  {
    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    std::string s1 = check_char_success(stmt, 1);
    std::string s2 = check_char_success(stmt, 2);
    std::string s3 = check_char_success(stmt, 3);
    CHECK_THAT(std::stod(s1), Catch::Matchers::WithinRel(1.5));
    CHECK_THAT(std::stod(s2), Catch::Matchers::WithinRel(-2.75));
    CHECK_THAT(std::stod(s3), Catch::Matchers::WithinRel(100.0));
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL NULL to SQL_C_DEFAULT", "[real][conversion][c_default][null]") {
  // Given A Snowflake connection

  // When A NULL FLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT");
  // Then NULL FLOAT values return SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_DEFAULT);
}
