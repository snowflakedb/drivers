#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// BASIC INTEGER CONVERSIONS
// ============================================================================

TEST_CASE("DECFLOAT to integer C types", "[decfloat][conversion][c_integer]") {
  // Given Snowflake client is logged in
  Connection conn;
  const std::string query = "SELECT 42::DECFLOAT";

  // When A small DECFLOAT integer is fetched as various integer C types
  (void)0;
  // Then All integer C types return 42
  {
    INFO("SQL_C_LONG");
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_SLONG");
    CHECK(check_no_truncation<SQL_C_SLONG>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_ULONG");
    CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_SHORT");
    CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_SSHORT");
    CHECK(check_no_truncation<SQL_C_SSHORT>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_USHORT");
    CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_SBIGINT");
    CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_UBIGINT");
    CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_TINYINT");
    CHECK(check_no_truncation<SQL_C_TINYINT>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_STINYINT");
    CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch(query), 1) == 42);
  }
  {
    INFO("SQL_C_UTINYINT");
    CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch(query), 1) == 42);
  }
}

// ============================================================================
// FRACTIONAL TRUNCATION
// ============================================================================

TEST_CASE("DECFLOAT fractional truncation to integer C types", "[decfloat][conversion][c_integer]") {
  // Given Snowflake client is logged in
  Connection conn;
  const std::string query = "SELECT 123.456::DECFLOAT";

  // When A fractional DECFLOAT value is fetched as integer C types
  (void)0;
  // Then Integer C types return 123 with SQLSTATE 01S07
  {
    INFO("SQL_C_LONG");
    CHECK(check_fractional_truncation<SQL_C_LONG>(conn.execute_fetch(query), 1) == 123);
  }
  {
    INFO("SQL_C_SBIGINT");
    CHECK(check_fractional_truncation<SQL_C_SBIGINT>(conn.execute_fetch(query), 1) == 123);
  }
}

// ============================================================================
// OVERFLOW
// ============================================================================

TEST_CASE("DECFLOAT overflow to integer C types", "[decfloat][conversion][c_integer][overflow]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A large DECFLOAT value is fetched as integer C types
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  {
    INFO("value exceeding LONG range");
    check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch("SELECT '99999999999999999999'::DECFLOAT"), 1);
  }
  {
    INFO("extreme exponent exceeding SBIGINT range");
    check_numeric_out_of_range<SQL_C_SBIGINT>(conn.execute_fetch("SELECT '1E+16384'::DECFLOAT"), 1);
  }
  {
    INFO("negative overflow for unsigned type");
    check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT '-1'::DECFLOAT"), 1);
  }
}

// ============================================================================
// TYPE BOUNDARIES
// ============================================================================

TEST_CASE("DECFLOAT integer type boundaries", "[decfloat][conversion][c_integer][limits]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values at exact type boundaries are fetched as integer C types
  (void)0;
  // Then Values at exact min/max boundaries are returned correctly
  {
    INFO("SQL_C_LONG max (2147483647)");
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT '2147483647'::DECFLOAT"), 1) == 2147483647);
  }
  {
    INFO("SQL_C_LONG min (-2147483648)");
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT '-2147483648'::DECFLOAT"), 1) ==
          (-2147483647 - 1));
  }
  {
    INFO("SQL_C_SHORT max (32767)");
    CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch("SELECT '32767'::DECFLOAT"), 1) == 32767);
  }
  {
    INFO("SQL_C_SHORT min (-32768)");
    CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch("SELECT '-32768'::DECFLOAT"), 1) == -32768);
  }
  {
    INFO("SQL_C_STINYINT max (127)");
    CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch("SELECT '127'::DECFLOAT"), 1) == 127);
  }
  {
    INFO("SQL_C_STINYINT min (-128)");
    CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch("SELECT '-128'::DECFLOAT"), 1) == -128);
  }
  {
    INFO("SQL_C_UTINYINT max (255)");
    CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch("SELECT '255'::DECFLOAT"), 1) == 255);
  }
  {
    INFO("SQL_C_USHORT max (65535)");
    CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch("SELECT '65535'::DECFLOAT"), 1) == 65535);
  }
  {
    INFO("SQL_C_ULONG max (4294967295)");
    CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch("SELECT '4294967295'::DECFLOAT"), 1) == 4294967295U);
  }
  {
    INFO("SQL_C_SBIGINT max");
    CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch("SELECT '9223372036854775807'::DECFLOAT"), 1) ==
          9223372036854775807LL);
  }
  {
    INFO("SQL_C_SBIGINT min");
    CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch("SELECT '-9223372036854775808'::DECFLOAT"), 1) ==
          (-9223372036854775807LL - 1));
  }
  {
    INFO("SQL_C_UBIGINT max");
    CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch("SELECT '18446744073709551615'::DECFLOAT"), 1) ==
          18446744073709551615ULL);
  }
}

// ============================================================================
// PER-TYPE OVERFLOW (22003)
// ============================================================================

TEST_CASE("DECFLOAT per-type integer overflow", "[decfloat][conversion][c_integer][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values just beyond type boundaries are fetched as integer C types
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003 for each type
  {
    INFO("SQL_C_LONG above max");
    check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch("SELECT '2147483648'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_LONG below min");
    check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch("SELECT '-2147483649'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_SHORT above max");
    check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT '32768'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_SHORT below min");
    check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT '-32769'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_STINYINT above max");
    check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT '128'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_STINYINT below min");
    check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT '-129'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_UTINYINT above max");
    check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT '256'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_UTINYINT negative");
    check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT '-1'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_USHORT above max");
    check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT '65536'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_USHORT negative");
    check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT '-1'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_ULONG above max");
    check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT '4294967296'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_SBIGINT above max");
    check_numeric_out_of_range<SQL_C_SBIGINT>(conn.execute_fetch("SELECT '9223372036854775808'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_SBIGINT below min");
    check_numeric_out_of_range<SQL_C_SBIGINT>(conn.execute_fetch("SELECT '-9223372036854775809'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_UBIGINT negative");
    check_numeric_out_of_range<SQL_C_UBIGINT>(conn.execute_fetch("SELECT '-1'::DECFLOAT"), 1);
  }
  {
    INFO("SQL_C_UBIGINT above max");
    check_numeric_out_of_range<SQL_C_UBIGINT>(conn.execute_fetch("SELECT '18446744073709551616'::DECFLOAT"), 1);
  }
}

// ============================================================================
// FRACTIONAL TRUNCATION - ALL TYPES
// ============================================================================

TEST_CASE("DECFLOAT fractional truncation for all integer types", "[decfloat][conversion][c_integer][01S07]") {
  // Given Snowflake client is logged in
  Connection conn;
  const std::string query = "SELECT 123.789::DECFLOAT";

  // When A fractional DECFLOAT value is fetched as each integer C type
  (void)0;
  // Then All integer C types return 123 with SQLSTATE 01S07
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

// ============================================================================
// POSITIVE EXPONENT
// ============================================================================

TEST_CASE("DECFLOAT with positive exponent to integer", "[decfloat][conversion][c_integer]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values with positive exponents are fetched as SQL_C_LONG
  (void)0;
  // Then The exponent is applied correctly to produce the integer result
  {
    INFO("42E2 = 4200");
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT '42E2'::DECFLOAT"), 1) == 4200);
  }
  {
    INFO("1E5 = 100000");
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT '1E5'::DECFLOAT"), 1) == 100000);
  }
  {
    INFO("-5E3 = -5000");
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT '-5E3'::DECFLOAT"), 1) == -5000);
  }
}

// ============================================================================
// TINY VALUE TRUNCATION
// ============================================================================

TEST_CASE("DECFLOAT tiny value to integer truncates to zero", "[decfloat][conversion][c_integer][01S07]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A very small fractional DECFLOAT value is fetched as SQL_C_LONG
  (void)0;
  // Then Value truncates to 0 with SQLSTATE 01S07
  CHECK(check_fractional_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT '0.000001'::DECFLOAT"), 1) == 0);
}

// ============================================================================
// SQLBindCol
// ============================================================================

TEST_CASE("DECFLOAT using SQLBindCol for SQL_C_LONG", "[decfloat][conversion][c_integer][bindcol]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values are fetched using SQLBindCol for SQL_C_LONG
  const auto stmt = conn.createStatement();
  SQLINTEGER val1 = 0, val2 = 0;
  SQLLEN ind1 = 0, ind2 = 0;

  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &val1, sizeof(val1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_LONG, &val2, sizeof(val2), &ind2);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 42::DECFLOAT, '-100'::DECFLOAT"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Bound integers contain correct DECFLOAT values
  CHECK(ind1 == sizeof(SQLINTEGER));
  CHECK(val1 == 42);
  CHECK(ind2 == sizeof(SQLINTEGER));
  CHECK(val2 == -100);
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("DECFLOAT NULL to integer C types", "[decfloat][conversion][c_integer][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DECFLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DECFLOAT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_LONG);
}
