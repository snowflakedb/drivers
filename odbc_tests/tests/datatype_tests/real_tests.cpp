
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cfloat>
#include <cmath>
#include <cstring>
#include <optional>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "TestTable.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

/// Helper: fetch a column with SQL_C_DEFAULT into an SQLDOUBLE.
/// Per ODBC spec, SQL_C_DEFAULT for SQL_DOUBLE resolves to SQL_C_DOUBLE.
inline SQLDOUBLE get_data_default_as_double(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  SQLDOUBLE value = 0.0;
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQLDOUBLE));
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

  // They must match exactly
  for (size_t i = 0; i < explicit_results.size(); ++i) {
    INFO("Row " << i << ": SQL_C_DOUBLE=" << explicit_results[i] << " vs SQL_C_DEFAULT=" << default_results[i]);
    CHECK(explicit_results[i] == default_results[i]);
  }
}

// ============================================================================
// Explicit C type conversions from FLOAT columns
// ============================================================================

TEST_CASE("REAL explicit SQL_C_DOUBLE", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 123.456::FLOAT");

  double val = check_no_truncation<SQL_C_DOUBLE>(stmt, 1);
  CHECK_THAT(val, Catch::Matchers::WithinRel(123.456));
}

TEST_CASE("REAL explicit SQL_C_FLOAT", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 123.5::FLOAT");

  float val = check_no_truncation<SQL_C_FLOAT>(stmt, 1);
  CHECK_THAT(val, Catch::Matchers::WithinRel(123.5f));
}

TEST_CASE("REAL explicit SQL_C_CHAR", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 123.456::FLOAT, -99.5::FLOAT, 0::FLOAT");

  std::string s1 = check_char_success(stmt, 1);
  std::string s2 = check_char_success(stmt, 2);
  std::string s3 = check_char_success(stmt, 3);

  CHECK_THAT(std::stod(s1), Catch::Matchers::WithinRel(123.456));
  CHECK_THAT(std::stod(s2), Catch::Matchers::WithinRel(-99.5));
  CHECK_THAT(std::stod(s3), Catch::Matchers::WithinAbs(0.0, 1e-15));
}

TEST_CASE("REAL explicit integer conversions truncate fractional part", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  const std::string query = "SELECT 123.789::FLOAT";

  CHECK(check_fractional_truncation<SQL_C_LONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SLONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_ULONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SSHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_USHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_TINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_STINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_UTINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SBIGINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_UBIGINT>(conn.execute_fetch(query), 1) == 123);
}

TEST_CASE("REAL explicit integer conversions - negative value", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  const std::string query = "SELECT -42.9::FLOAT";

  CHECK(check_fractional_truncation<SQL_C_LONG>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_SLONG>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_SHORT>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_SSHORT>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_TINYINT>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_STINYINT>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_SBIGINT>(conn.execute_fetch(query), 1) == -42);
}

TEST_CASE("REAL explicit SQL_C_BIT - basic", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  CHECK(check_no_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 0.0::FLOAT"), 1) == 0);
  CHECK(check_no_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 1.0::FLOAT"), 1) == 1);
  CHECK(check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 0.5::FLOAT"), 1) == 0);
  CHECK(check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 1.5::FLOAT"), 1) == 1);
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 5.5::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -1.5::FLOAT"), 1);
}

TEST_CASE("REAL explicit SQL_C_SBIGINT with large values", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 9007199254740992::FLOAT");

  // 2^53 = 9007199254740992: largest integer exactly representable as f64
  CHECK(check_no_truncation<SQL_C_SBIGINT>(stmt, 1) == 9007199254740992LL);
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
    CHECK(ret == SQL_SUCCESS);
    INFO("Row " << i);
    CHECK(get_data_default_as_double(stmt, 1) == expected[i]);
  }
}

// ============================================================================
// Precision boundary
// ============================================================================

TEST_CASE("REAL precision - Snowflake FLOAT has ~15 significant digits", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 1.23456789012345::FLOAT");
  double val = check_no_truncation<SQL_C_DOUBLE>(stmt, 1);
  CHECK_THAT(val, Catch::Matchers::WithinRel(1.23456789012345));
}

TEST_CASE("REAL default conversion - fractional values", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 0.1::FLOAT, 0.5::FLOAT, 0.333333333::FLOAT");

  double v1 = get_data_default_as_double(stmt, 1);
  double v2 = get_data_default_as_double(stmt, 2);
  double v3 = get_data_default_as_double(stmt, 3);

  CHECK_THAT(v1, Catch::Matchers::WithinRel(0.1));
  CHECK(v2 == 0.5);
  CHECK_THAT(v3, Catch::Matchers::WithinRel(0.333333333));
}

TEST_CASE("REAL zero is exactly zero", "[datatype][real][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 0.0::FLOAT");

  double val = get_data_default_as_double(stmt, 1);
  CHECK(val == 0.0);
}

// ============================================================================
// ODBC Spec: SQLSTATE 01S07 — Fractional truncation warning
// Converting a FLOAT value with fractional digits to an integer C type
// should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01S07.
// ============================================================================

TEST_CASE("REAL fractional truncation returns 01S07", "[datatype][real][01S07]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_LONG
  {
    auto stmt = conn.execute_fetch("SELECT 123.45::FLOAT");
    auto value = check_fractional_truncation<SQL_C_LONG>(stmt, 1);
    CHECK(value == 123);
  }

  // SQL_C_SHORT
  {
    auto stmt = conn.execute_fetch("SELECT 9.99::FLOAT");
    auto value = check_fractional_truncation<SQL_C_SHORT>(stmt, 1);
    CHECK(value == 9);
  }

  // SQL_C_STINYINT
  {
    auto stmt = conn.execute_fetch("SELECT 1.5::FLOAT");
    auto value = check_fractional_truncation<SQL_C_STINYINT>(stmt, 1);
    CHECK(value == 1);
  }

  // SQL_C_SBIGINT
  {
    auto stmt = conn.execute_fetch("SELECT 999.001::FLOAT");
    auto value = check_fractional_truncation<SQL_C_SBIGINT>(stmt, 1);
    CHECK(value == 999);
  }

  // exact integer does NOT produce 01S07
  {
    auto stmt = conn.execute_fetch("SELECT 100.0::FLOAT");
    auto value = check_no_truncation<SQL_C_LONG>(stmt, 1);
    CHECK(value == 100);
  }

  // negative fractional truncates toward zero
  {
    auto stmt = conn.execute_fetch("SELECT -42.9::FLOAT");
    auto value = check_fractional_truncation<SQL_C_LONG>(stmt, 1);
    CHECK(value == -42);
  }
}

// ============================================================================
// ODBC Spec: SQLSTATE 22003 — Numeric value out of range (integer overflow)
// When a FLOAT value exceeds the target integer C type range, the driver
// should return SQL_ERROR with SQLSTATE 22003.
// ============================================================================

TEST_CASE("REAL overflow returns 22003", "[datatype][real][22003]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_STINYINT - above i8 max
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 128.0::FLOAT"), 1);

  // SQL_C_STINYINT - below i8 min
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT -129.0::FLOAT"), 1);

  // SQL_C_UTINYINT - negative
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT -1.0::FLOAT"), 1);

  // SQL_C_UTINYINT - above u8 max
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 256.0::FLOAT"), 1);

  // SQL_C_SHORT - above i16 max
  check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT 32768.0::FLOAT"), 1);

  // SQL_C_USHORT - negative
  check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT -1.0::FLOAT"), 1);

  // SQL_C_LONG - above i32 max
  check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch("SELECT 2147483648.0::FLOAT"), 1);

  // SQL_C_ULONG - negative
  check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT -1.0::FLOAT"), 1);
}

// ============================================================================
// ODBC Spec: SQL_C_BIT — full spec-compliant behavior for FLOAT source
// Per ODBC spec for SQL_C_BIT:
//   Exact 0 or 1                    -> SQL_SUCCESS
//   Value > 0, < 2, != 1 (fraction) -> SQL_SUCCESS_WITH_INFO, 01S07
//   Value < 0 or >= 2               -> SQL_ERROR, 22003
// ============================================================================

TEST_CASE("REAL SQL_C_BIT spec compliance", "[datatype][real][bit]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // 0.0 and 1.0 succeed
  {
    auto stmt = conn.execute_fetch("SELECT 0.0::FLOAT, 1.0::FLOAT");
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 1) == 0);
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 2) == 1);
  }

  // value 2 returns 22003
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 2.0::FLOAT"), 1);

  // negative value returns 22003
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -1.0::FLOAT"), 1);

  // fractional 0.5 truncates to 0 with 01S07
  {
    auto value = check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 0.5::FLOAT"), 1);
    CHECK(value == 0);
  }

  // fractional 1.5 truncates to 1 with 01S07
  {
    auto value = check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 1.5::FLOAT"), 1);
    CHECK(value == 1);
  }

  // large positive value returns 22003
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 100.0::FLOAT"), 1);
}

// ============================================================================
// NULL handling for REAL/FLOAT columns
// ============================================================================

TEST_CASE("REAL NULL handling", "[datatype][real][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_DOUBLE returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_DOUBLE);

  // SQL_C_FLOAT returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DOUBLE"), 1, SQL_C_FLOAT);

  // SQL_C_LONG returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_LONG);

  // SQL_C_CHAR returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_CHAR);

  // SQL_C_DEFAULT returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_DEFAULT);

  // SQL_C_BIT returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_BIT);

  // SQL_C_SBIGINT returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_SBIGINT);
}

TEST_CASE("REAL NULL mixed with non-NULL in multiple rows", "[datatype][real][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  TestTable table(conn, "test_real_null", "val FLOAT", "(1.5), (NULL), (-2.75), (NULL), (0.0)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  std::vector<std::optional<double>> expected = {1.5, std::nullopt, -2.75, std::nullopt, 0.0};
  SQLDOUBLE value = 0;
  SQLLEN indicator = 0;

  for (size_t row = 0; row < expected.size(); ++row) {
    if (row > 0) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK(ret == SQL_SUCCESS);
    }
    INFO("Row " << row);
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    if (expected[row].has_value()) {
      CHECK(indicator != SQL_NULL_DATA);
      CHECK(value == expected[row].value());
    } else {
      CHECK(indicator == SQL_NULL_DATA);
    }
  }
}

TEST_CASE("REAL SQLGetData NULL without indicator returns 22002", "[datatype][real][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT");

  SQLDOUBLE value = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &value, sizeof(value), nullptr);
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22002");
}

// ============================================================================
// SQL_C_WCHAR (wide character) conversion for REAL
// ============================================================================

TEST_CASE("REAL to SQL_C_WCHAR", "[datatype][real][wchar]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // basic values
  {
    auto stmt = conn.execute_fetch("SELECT 123.456::FLOAT, -99.5::FLOAT, 0.0::FLOAT");
    auto s1 = check_wchar_success(stmt, 1);
    auto s2 = check_wchar_success(stmt, 2);
    auto s3 = check_wchar_success(stmt, 3);

    CHECK(!s1.empty());
    CHECK(!s2.empty());
    CHECK(!s3.empty());
  }

  // matches SQL_C_CHAR
  {
    const std::string query = "SELECT 3.125::FLOAT";
    auto char_str = check_char_success(conn.execute_fetch(query), 1);
    auto wchar_str = check_wchar_success(conn.execute_fetch(query), 1);
    std::u16string expected_wchar(char_str.begin(), char_str.end());
    CHECK(wchar_str == expected_wchar);
  }
}

// ============================================================================
// Unsigned integer conversions from FLOAT
// ============================================================================

TEST_CASE("REAL explicit unsigned integer conversions", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  const std::string q_exact = "SELECT 42.0::FLOAT";
  const std::string q_frac = "SELECT 42.9::FLOAT";
  const std::string q_zero = "SELECT 0.0::FLOAT";

  // positive value to unsigned types
  {
    CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch(q_exact), 1) == 42u);
    CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch(q_exact), 1) == 42u);
    CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch(q_exact), 1) == 42u);
    CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_exact), 1) == 42u);
  }

  // fractional value to unsigned types truncates with 01S07
  {
    CHECK(check_fractional_truncation<SQL_C_ULONG>(conn.execute_fetch(q_frac), 1) == 42u);
    CHECK(check_fractional_truncation<SQL_C_USHORT>(conn.execute_fetch(q_frac), 1) == 42u);
    CHECK(check_fractional_truncation<SQL_C_UTINYINT>(conn.execute_fetch(q_frac), 1) == 42u);
    CHECK(check_fractional_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_frac), 1) == 42u);
  }

  // zero to unsigned types
  {
    CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch(q_zero), 1) == 0u);
    CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch(q_zero), 1) == 0u);
    CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch(q_zero), 1) == 0u);
    CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_zero), 1) == 0u);
  }
}

// ============================================================================
// SQL_C_CHAR buffer truncation for REAL
// ============================================================================

TEST_CASE("REAL SQL_C_CHAR buffer handling", "[datatype][real][char][buffer]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("large buffer succeeds") {
    auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");

    char buffer[100];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
    CHECK_THAT(std::stod(std::string(buffer, indicator)), Catch::Matchers::WithinRel(42.5));
  }

  SECTION("fractional-only truncation returns 01004") {
    SKIP_OLD_DRIVER(
        "BD#17",
        "Old driver returns SQL_ERROR instead of SQL_SUCCESS_WITH_INFO for small SQL_C_CHAR buffer on FLOAT columns");

    auto stmt = conn.execute_fetch("SELECT 3.14159::FLOAT");

    char small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01004");
  }

  SECTION("whole digits lost returns 22003") {
    auto stmt = conn.execute_fetch("SELECT 123456.789::FLOAT");

    char small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }
}

// ============================================================================
// SQL_C_WCHAR buffer truncation for REAL
// ============================================================================

TEST_CASE("REAL SQL_C_WCHAR buffer handling", "[datatype][real][wchar][buffer]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("large buffer succeeds") {
    auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");
    auto result = check_wchar_success(stmt, 1);
    CHECK(!result.empty());
  }

  SECTION("fractional-only truncation returns 01004") {
    SKIP_OLD_DRIVER(
        "BD#17",
        "Old driver returns SQL_ERROR instead of SQL_SUCCESS_WITH_INFO for small SQL_C_WCHAR buffer on FLOAT columns");

    auto stmt = conn.execute_fetch("SELECT 3.14159::FLOAT");

    char16_t small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01004");
  }

  SECTION("whole digits lost returns 22003") {
    auto stmt = conn.execute_fetch("SELECT 123456.789::FLOAT");

    char16_t small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }
}

// ============================================================================
// Table-based tests with FLOAT column types
// ============================================================================

TEST_CASE("REAL table column conversions", "[datatype][real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  TestTable table(conn, "test_real_conversions", "f FLOAT, d DOUBLE, r REAL", "(1.5, -2.75, 100.0)");

  // SQL_C_DOUBLE from all column types
  {
    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == 1.5);
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 2) == -2.75);
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 3) == 100.0);
  }

  // SQL_C_LONG truncates fractional with 01S07
  {
    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 1) == 1);
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 2) == -2);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 3) == 100);
  }

  // SQL_C_CHAR returns string representation
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

// ============================================================================
// SQL_C_NUMERIC conversion from REAL
// The old driver (via Simba SDK) supports SQL_C_NUMERIC for SQL_DOUBLE.
// ============================================================================

TEST_CASE("REAL to SQL_C_NUMERIC", "[datatype][real][numeric]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // positive integer value
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 42.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 42);
    for (int i = 1; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // negative integer value
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -7.0::FLOAT"), 1);
    CHECK(numeric.sign == 0);
    CHECK(numeric.val[0] == 7);
    for (int i = 1; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // zero
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 0.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    for (int i = 0; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // fractional value truncates with 01S07
  {
    auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 123.456::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    unsigned long long stored = 0;
    for (int i = 7; i >= 0; --i) {
      stored = (stored << 8) | numeric.val[i];
    }
    CHECK(stored == 123);
  }

  // large integer value
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 1000000.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    unsigned long long stored = 0;
    for (int i = 7; i >= 0; --i) {
      stored = (stored << 8) | numeric.val[i];
    }
    CHECK(stored == 1000000);
  }

  // negative fractional value truncates with 01S07
  {
    auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -99.9::FLOAT"), 1);
    CHECK(numeric.sign == 0);
    unsigned long long stored = 0;
    for (int i = 7; i >= 0; --i) {
      stored = (stored << 8) | numeric.val[i];
    }
    CHECK(stored == 99);
  }

  // value 1 has correct val bytes
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 1.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 1);
    for (int i = 1; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // value 255 uses single byte
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 255.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 255);
    for (int i = 1; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // value 256 spans two bytes
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 256.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 0);
    CHECK(numeric.val[1] == 1);
    for (int i = 2; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // NULL returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_NUMERIC);
}

// ============================================================================
// Edge cases: negative zero, boundary values
// ============================================================================

TEST_CASE("REAL negative zero", "[datatype][real][edge]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT -0.0::FLOAT");
  double val = check_no_truncation<SQL_C_DOUBLE>(stmt, 1);
  CHECK(val == 0.0);
}

TEST_CASE("REAL integer boundary values for overflow", "[datatype][real][edge][22003]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // exactly at i8 max succeeds
  CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch("SELECT 127.0::FLOAT"), 1) == 127);

  // exactly at i8 min succeeds
  CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch("SELECT -128.0::FLOAT"), 1) == -128);

  // exactly at i16 max succeeds
  CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch("SELECT 32767.0::FLOAT"), 1) == 32767);

  // exactly at i16 min succeeds
  CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch("SELECT -32768.0::FLOAT"), 1) == -32768);

  // exactly at u8 max succeeds
  CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 255.0::FLOAT"), 1) == 255);

  // exactly at u16 max succeeds
  CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch("SELECT 65535.0::FLOAT"), 1) == 65535);

  // one past i8 max overflows
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 128.0::FLOAT"), 1);

  // one past i8 min overflows
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT -129.0::FLOAT"), 1);

  // one past u8 max overflows
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 256.0::FLOAT"), 1);

  // one past u16 max overflows
  check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT 65536.0::FLOAT"), 1);

  // fractional value well within i8 range truncates with 01S07
  CHECK(check_fractional_truncation<SQL_C_STINYINT>(conn.execute_fetch("SELECT 126.9::FLOAT"), 1) == 126);

  // fractional value above i8 max overflows
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 128.1::FLOAT"), 1);
}

TEST_CASE("REAL SQL_C_FLOAT precision loss", "[datatype][real][edge]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // value representable in f32 matches
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch("SELECT 0.5::FLOAT"), 1) == 0.5f);

  // large value representable in f32
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch("SELECT 1000000.0::FLOAT"), 1) == 1000000.0f);
}

TEST_CASE("REAL SQL_C_FLOAT overflow returns 22003", "[datatype][real][22003]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // positive overflow
  check_numeric_out_of_range<SQL_C_FLOAT>(conn.execute_fetch("SELECT 1e300::FLOAT"), 1);

  // negative overflow
  check_numeric_out_of_range<SQL_C_FLOAT>(conn.execute_fetch("SELECT -1e300::FLOAT"), 1);

  // large value within f32 range succeeds
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch("SELECT 1e38::FLOAT"), 1) == 1e38f);
}

// ============================================================================
// SQL_C_BINARY conversion from REAL
// Per ODBC spec, SQL_C_BINARY for SQL_REAL/SQL_FLOAT/SQL_DOUBLE writes the
// value as SQL_NUMERIC_STRUCT into the buffer.
// ============================================================================

inline SQL_NUMERIC_STRUCT get_real_binary_as_numeric(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_NUMERIC_STRUCT));
  return *reinterpret_cast<SQL_NUMERIC_STRUCT*>(buffer);
}

inline SQL_NUMERIC_STRUCT get_real_binary_as_numeric_with_truncation(const StatementHandleWrapper& stmt,
                                                                     SQLUSMALLINT col) {
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == sizeof(SQL_NUMERIC_STRUCT));
  auto records = get_diag_rec(stmt);
  CHECK(records.size() == 1);
  CHECK(records[0].sqlState == "01S07");
  return *reinterpret_cast<SQL_NUMERIC_STRUCT*>(buffer);
}

inline void check_real_numeric_val_zero_from(const SQL_NUMERIC_STRUCT& numeric, int start) {
  for (int i = start; i < 16; ++i) {
    INFO("val[" << i << "] should be 0");
    CHECK(numeric.val[i] == 0);
  }
}

inline unsigned long long real_numeric_val_to_ull(const SQL_NUMERIC_STRUCT& n) {
  unsigned long long result = 0;
  for (int i = 7; i >= 0; --i) {
    result = (result << 8) | n.val[i];
  }
  return result;
}

TEST_CASE("REAL to SQL_C_BINARY", "[datatype][real][binary]") {
  SKIP_OLD_DRIVER("BD#16",
                  "Old driver returns raw f64 bytes instead of SQL_NUMERIC_STRUCT for SQL_C_BINARY on FLOAT columns");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // positive integer value
  {
    auto num = get_real_binary_as_numeric(conn.execute_fetch("SELECT 42.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(num.val[0] == 42);
    check_real_numeric_val_zero_from(num, 1);
  }

  // negative integer value
  {
    auto num = get_real_binary_as_numeric(conn.execute_fetch("SELECT -7.0::FLOAT"), 1);
    CHECK(num.sign == 0);
    CHECK(num.val[0] == 7);
    check_real_numeric_val_zero_from(num, 1);
  }

  // zero
  {
    auto num = get_real_binary_as_numeric(conn.execute_fetch("SELECT 0.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    check_real_numeric_val_zero_from(num, 0);
  }

  // fractional value truncates with 01S07
  {
    auto num = get_real_binary_as_numeric_with_truncation(conn.execute_fetch("SELECT 123.456::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(num.scale == 0);
    CHECK(real_numeric_val_to_ull(num) == 123);
  }

  // large integer value
  {
    auto num = get_real_binary_as_numeric(conn.execute_fetch("SELECT 1000000.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(real_numeric_val_to_ull(num) == 1000000);
  }

  // negative fractional truncates with 01S07
  {
    auto num = get_real_binary_as_numeric_with_truncation(conn.execute_fetch("SELECT -99.9::FLOAT"), 1);
    CHECK(num.sign == 0);
    CHECK(real_numeric_val_to_ull(num) == 99);
  }

  // value 255 uses single byte
  {
    auto num = get_real_binary_as_numeric(conn.execute_fetch("SELECT 255.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(num.val[0] == 255);
    check_real_numeric_val_zero_from(num, 1);
  }

  // value 256 spans two bytes
  {
    auto num = get_real_binary_as_numeric(conn.execute_fetch("SELECT 256.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(num.val[0] == 0);
    CHECK(num.val[1] == 1);
    check_real_numeric_val_zero_from(num, 2);
  }

  // NULL returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_BINARY);
}

TEST_CASE("REAL SQL_C_BINARY buffer too small returns 22003", "[datatype][real][binary][22003]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 42.0::FLOAT");

  char tiny_buffer[4];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, tiny_buffer, sizeof(tiny_buffer), &indicator);

  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22003");
}

// ============================================================================
// BIT — negative fractional values must return 22003
// Per ODBC spec, any value < 0 is out of range for SQL_C_BIT.
// ============================================================================

TEST_CASE("REAL SQL_C_BIT rejects negative fractions", "[datatype][real][bit][edge]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // -0.5 returns 22003
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -0.5::FLOAT"), 1);

  // -0.001 returns 22003
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -0.001::FLOAT"), 1);

  // -0.9999 returns 22003
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -0.9999::FLOAT"), 1);
}

// ============================================================================
// Numeric / Binary — negative fractional values that truncate to zero
// must NOT produce negative-zero (sign must be 1/positive when val=0).
// ============================================================================

TEST_CASE("REAL SQL_C_NUMERIC no negative zero", "[datatype][real][numeric][edge]") {
  SKIP_OLD_DRIVER("BD#19", "Old driver produces negative zero in SQL_NUMERIC_STRUCT for negative fractional values");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // -0.5 produces positive zero
  {
    auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -0.5::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 0);
  }

  // -0.001 produces positive zero
  {
    auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -0.001::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 0);
  }
}

TEST_CASE("REAL SQL_C_BINARY no negative zero", "[datatype][real][binary][edge]") {
  SKIP_OLD_DRIVER("BD#12", "SNOW-3127864: Old driver fix to be merged to return the proper size for SQL_NUMERIC");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT -0.5::FLOAT");

  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01S07");
  CHECK(indicator == sizeof(SQL_NUMERIC_STRUCT));
  auto* numeric = reinterpret_cast<SQL_NUMERIC_STRUCT*>(buffer);
  CHECK(numeric->sign == 1);
  CHECK(numeric->val[0] == 0);
}

TEST_CASE("REAL explicit SQL_C_CHAR for special values", "[datatype][real][char][edge]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // integer value has no decimal point
  {
    std::string s = check_char_success(conn.execute_fetch("SELECT 42.0::FLOAT"), 1);
    CHECK_THAT(std::stod(s), Catch::Matchers::WithinAbs(42.0, 1e-15));
  }

  // negative value
  {
    std::string s = check_char_success(conn.execute_fetch("SELECT -0.001::FLOAT"), 1);
    CHECK_THAT(std::stod(s), Catch::Matchers::WithinRel(-0.001));
  }

  // very small positive value
  {
    std::string s = check_char_success(conn.execute_fetch("SELECT 1e-300::FLOAT"), 1);
    double parsed = std::stod(s);
    CHECK(parsed > 0.0);
    CHECK(parsed < 1e-299);
  }

  // very large value
  {
    std::string s = check_char_success(conn.execute_fetch("SELECT 1e300::FLOAT"), 1);
    double parsed = std::stod(s);
    CHECK(parsed > 9e299);
  }
}

// ============================================================================
// Negative fractional values to unsigned integer types
// Values like -0.1 truncate to -0.0 (IEEE 754).
// ============================================================================

TEST_CASE("REAL negative fraction to unsigned integer types", "[datatype][real][unsigned][edge]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_UTINYINT with -0.1
  {
    auto stmt = conn.execute_fetch("SELECT -0.1::FLOAT");
    typename MetaOfSqlCType<SQL_C_UTINYINT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_UTINYINT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << (int)value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_USHORT with -0.1
  {
    auto stmt = conn.execute_fetch("SELECT -0.1::FLOAT");
    typename MetaOfSqlCType<SQL_C_USHORT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_USHORT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_ULONG with -0.1
  {
    auto stmt = conn.execute_fetch("SELECT -0.1::FLOAT");
    typename MetaOfSqlCType<SQL_C_ULONG>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_ULONG, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_UBIGINT with -0.1
  {
    auto stmt = conn.execute_fetch("SELECT -0.1::FLOAT");
    typename MetaOfSqlCType<SQL_C_UBIGINT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_UBIGINT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_UTINYINT with -0.9
  {
    auto stmt = conn.execute_fetch("SELECT -0.9::FLOAT");
    typename MetaOfSqlCType<SQL_C_UTINYINT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_UTINYINT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << (int)value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_USHORT with -0.9
  {
    auto stmt = conn.execute_fetch("SELECT -0.9::FLOAT");
    typename MetaOfSqlCType<SQL_C_USHORT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_USHORT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }
}

// ============================================================================
// NaN handling for integer, bit, numeric, and binary targets
// NaN must not silently convert to 0. Per ODBC spec, NaN should produce
// SQL_ERROR with SQLSTATE 22003 for integer/bit/numeric/binary targets.
// For CHAR/WCHAR targets, NaN should produce the string "NaN".
// ============================================================================

TEST_CASE("REAL NaN to integer types returns error", "[datatype][real][nan][edge]") {
  SKIP_OLD_DRIVER("BD#18", "Old driver silently converts NaN to 0 for integer targets");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_SLONG
  check_numeric_out_of_range<SQL_C_SLONG>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);

  // SQL_C_ULONG
  check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);

  // SQL_C_SHORT
  check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);

  // SQL_C_USHORT
  check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);

  // SQL_C_STINYINT
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);

  // SQL_C_UTINYINT
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);

  // SQL_C_SBIGINT
  check_numeric_out_of_range<SQL_C_SBIGINT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);

  // SQL_C_UBIGINT
  check_numeric_out_of_range<SQL_C_UBIGINT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
}

TEST_CASE("REAL NaN to BIT returns error", "[datatype][real][nan][edge]") {
  SKIP_OLD_DRIVER("BD#18", "Old driver silently converts NaN to 0 for BIT target");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
}

TEST_CASE("REAL NaN to NUMERIC returns error", "[datatype][real][nan][edge]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  check_numeric_out_of_range<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
}

TEST_CASE("REAL NaN to CHAR produces NaN string", "[datatype][real][nan][edge]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 'NaN'::FLOAT");
  char buf[64] = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buf, sizeof(buf), &indicator);
  CHECK(ret == SQL_SUCCESS);
  std::string result(buf);
  CHECK(result == "NaN");
}
