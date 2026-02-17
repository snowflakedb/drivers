
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cfloat>
#include <cmath>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "get_data.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

/// Helper: fetch a column with SQL_C_DEFAULT into an SQLDOUBLE.
/// Per ODBC spec, SQL_C_DEFAULT for SQL_DOUBLE resolves to SQL_C_DOUBLE.
inline SQLDOUBLE get_data_default_as_double(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  SQLDOUBLE value = 0.0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  return value;
}

// ============================================================================
// SQL_C_DEFAULT for FLOAT/DOUBLE columns
// Per ODBC spec, the "real" logical type maps to SQL_DOUBLE.
// SQL_C_DEFAULT for SQL_DOUBLE is SQL_C_DOUBLE.
// ============================================================================

TEST_CASE("REAL default conversion - basic values", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("DROP TABLE IF EXISTS test_real_default");
  conn.execute(
      "CREATE TABLE test_real_default ("
      "  f1 FLOAT, "
      "  f2 DOUBLE, "
      "  f3 FLOAT)");
  conn.execute("INSERT INTO test_real_default VALUES (1.5, -2.75, 0.0)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_real_default");

  CHECK(get_data_default_as_double(stmt, 1) == 1.5);
  CHECK(get_data_default_as_double(stmt, 2) == -2.75);
  CHECK(get_data_default_as_double(stmt, 3) == 0.0);
}

TEST_CASE("REAL default conversion - integer values stored as float", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 42::FLOAT, -100::FLOAT, 0::FLOAT, 1::FLOAT");

  CHECK(get_data_default_as_double(stmt, 1) == 42.0);
  CHECK(get_data_default_as_double(stmt, 2) == -100.0);
  CHECK(get_data_default_as_double(stmt, 3) == 0.0);
  CHECK(get_data_default_as_double(stmt, 4) == 1.0);
}

TEST_CASE("REAL default conversion - extreme values near DBL_MAX", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

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

  CHECK(get_data_default_as_double(stmt, 1) == 1.7976931348623157e308);

  SQLRETURN ret;
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_default_as_double(stmt, 1) == 1.7e308);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_default_as_double(stmt, 1) == 1.7976931348623151e308);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_default_as_double(stmt, 1) == -1.7976931348623151e308);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_default_as_double(stmt, 1) == -1.7e308);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_default_as_double(stmt, 1) == -1.7976931348623157e308);
}

TEST_CASE("REAL default conversion - very small values", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch(
      "SELECT 2.2250738585072014e-308::DOUBLE, "
      "       1e-307::DOUBLE, "
      "       -2.2250738585072014e-308::DOUBLE");

  double v1 = get_data_default_as_double(stmt, 1);
  double v2 = get_data_default_as_double(stmt, 2);
  double v3 = get_data_default_as_double(stmt, 3);

  CHECK(v1 > 0.0);
  CHECK(v1 < 1e-300);
  CHECK(v2 == 1e-307);
  CHECK(v3 < 0.0);
  CHECK(v3 > -1e-300);
}

TEST_CASE("REAL default conversion - FLOAT, DOUBLE, REAL synonyms produce same result", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

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

  CHECK(f == d);
  CHECK(d == r);
}

// ============================================================================
// SQL_C_DEFAULT must produce the same result as explicit SQL_C_DOUBLE
// ============================================================================

TEST_CASE("REAL SQL_C_DEFAULT matches explicit SQL_C_DOUBLE", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("DROP TABLE IF EXISTS test_real_default_vs_explicit");
  conn.execute("CREATE TABLE test_real_default_vs_explicit (val DOUBLE)");
  conn.execute(
      "INSERT INTO test_real_default_vs_explicit VALUES "
      "(1.5), (-2.75), (0.0), (999999.999), (1.7976931348623157e308)");

  // Fetch with SQL_C_DOUBLE explicitly
  auto stmt_explicit = conn.execute_fetch("SELECT * FROM test_real_default_vs_explicit");
  std::vector<double> explicit_results;
  explicit_results.push_back(get_data<SQL_C_DOUBLE>(stmt_explicit, 1));
  for (int i = 1; i < 5; ++i) {
    SQLRETURN ret = SQLFetch(stmt_explicit.getHandle());
    CHECK_ODBC(ret, stmt_explicit);
    explicit_results.push_back(get_data<SQL_C_DOUBLE>(stmt_explicit, 1));
  }

  // Fetch with SQL_C_DEFAULT
  auto stmt_default = conn.execute_fetch("SELECT * FROM test_real_default_vs_explicit");
  std::vector<double> default_results;
  default_results.push_back(get_data_default_as_double(stmt_default, 1));
  for (int i = 1; i < 5; ++i) {
    SQLRETURN ret = SQLFetch(stmt_default.getHandle());
    CHECK_ODBC(ret, stmt_default);
    default_results.push_back(get_data_default_as_double(stmt_default, 1));
  }

  // They must match exactly
  for (size_t i = 0; i < explicit_results.size(); ++i) {
    INFO("Row " << i << ": SQL_C_DOUBLE=" << explicit_results[i] << " vs SQL_C_DEFAULT=" << default_results[i]);
    CHECK(explicit_results[i] == default_results[i]);
  }
}

// ============================================================================
// Multiple rows
// ============================================================================

TEST_CASE("REAL default conversion - multiple rows", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("DROP TABLE IF EXISTS test_real_multi");
  conn.execute("CREATE TABLE test_real_multi (val DOUBLE)");
  conn.execute(
      "INSERT INTO test_real_multi VALUES "
      "(1.5), (-2.75), (0.0), (1e100), (-1e100)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_real_multi");

  std::vector<double> expected = {1.5, -2.75, 0.0, 1e100, -1e100};

  CHECK(get_data_default_as_double(stmt, 1) == expected[0]);
  for (size_t i = 1; i < expected.size(); ++i) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    INFO("Row " << i);
    CHECK(get_data_default_as_double(stmt, 1) == expected[i]);
  }
}

// ============================================================================
// Fractional values and zero
// ============================================================================

TEST_CASE("REAL default conversion - fractional values", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 0.1::FLOAT, 0.5::FLOAT, 0.333333333::FLOAT");

  double v1 = get_data_default_as_double(stmt, 1);
  double v2 = get_data_default_as_double(stmt, 2);
  double v3 = get_data_default_as_double(stmt, 3);

  CHECK(std::abs(v1 - 0.1) < 1e-15);
  CHECK(v2 == 0.5);
  CHECK(std::abs(v3 - 0.333333333) < 1e-8);
}

TEST_CASE("REAL zero is exactly zero", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 0.0::FLOAT");

  double val = get_data_default_as_double(stmt, 1);
  CHECK(val == 0.0);
}
