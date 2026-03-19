// REAL conversion to SQL_C_DEFAULT E2E tests
// SQL_C_DEFAULT for SQL_DOUBLE resolves to SQL_C_DOUBLE.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "TestTable.hpp"
#include "conversion_checks.hpp"

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

TEST_CASE("REAL default conversion - basic values", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT/DOUBLE values are inserted and fetched via SQL_C_DEFAULT
  conn.execute("DROP TABLE IF EXISTS test_real_default");
  conn.execute(
      "CREATE TABLE test_real_default ("
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

TEST_CASE("REAL default conversion - integer values stored as float", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Integer values stored as FLOAT are fetched via SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT 42::FLOAT, -100::FLOAT, 0::FLOAT, 1::FLOAT");

  // Then The correct double values are returned
  CHECK(get_data_default_as_double(stmt, 1) == 42.0);
  CHECK(get_data_default_as_double(stmt, 2) == -100.0);
  CHECK(get_data_default_as_double(stmt, 3) == 0.0);
  CHECK(get_data_default_as_double(stmt, 4) == 1.0);
}

TEST_CASE("REAL default conversion - extreme values near DBL_MAX", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Extreme values near DBL_MAX are inserted and fetched via SQL_C_DEFAULT
  conn.execute("DROP TABLE IF EXISTS test_real_extreme");
  conn.execute("CREATE TABLE test_real_extreme (val DOUBLE)");
  conn.execute(
      "INSERT INTO test_real_extreme VALUES "
      "(1.7976931348623157e308), "
      "(1.7e308), "
      "(1.7976931348623151e308), "
      "(-1.7976931348623151e308), "
      "(-1.7e308), "
      "(-1.7976931348623157e308)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_real_extreme");

  // Then The correct extreme double values are returned
  CHECK(get_data_default_as_double(stmt, 1) == 1.7976931348623157e308);

  SQLRETURN ret;
  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_SUCCESS);
  CHECK(get_data_default_as_double(stmt, 1) == 1.7e308);

  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_SUCCESS);
  CHECK(get_data_default_as_double(stmt, 1) == 1.7976931348623151e308);

  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_SUCCESS);
  CHECK(get_data_default_as_double(stmt, 1) == -1.7976931348623151e308);

  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_SUCCESS);
  CHECK(get_data_default_as_double(stmt, 1) == -1.7e308);

  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_SUCCESS);
  CHECK(get_data_default_as_double(stmt, 1) == -1.7976931348623157e308);
}

TEST_CASE("REAL default conversion - very small values", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

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

TEST_CASE("REAL default conversion - FLOAT, DOUBLE, REAL synonyms produce same result", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Same value is stored in FLOAT, DOUBLE, REAL columns and fetched via SQL_C_DEFAULT
  conn.execute("DROP TABLE IF EXISTS test_real_synonyms");
  conn.execute(
      "CREATE TABLE test_real_synonyms ("
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

TEST_CASE("REAL SQL_C_DEFAULT matches explicit SQL_C_DOUBLE", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Values are fetched with SQL_C_DOUBLE and SQL_C_DEFAULT
  conn.execute("DROP TABLE IF EXISTS test_real_default_vs_explicit");
  conn.execute("CREATE TABLE test_real_default_vs_explicit (val DOUBLE)");
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

TEST_CASE("REAL default conversion - multiple rows", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Multiple DOUBLE rows are fetched via SQL_C_DEFAULT
  conn.execute("DROP TABLE IF EXISTS test_real_multi");
  conn.execute("CREATE TABLE test_real_multi (val DOUBLE)");
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

TEST_CASE("REAL default conversion - fractional values", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

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

TEST_CASE("REAL zero is exactly zero", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Zero FLOAT value is fetched via SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT 0.0::FLOAT");

  double val = get_data_default_as_double(stmt, 1);

  // Then The value is exactly zero
  CHECK(val == 0.0);
}

TEST_CASE("REAL table column conversions", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

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

TEST_CASE("REAL NULL to SQL_C_DEFAULT", "[real][conversion][c_default][null]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL FLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT");
  // Then NULL FLOAT values return SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_DEFAULT);
}
