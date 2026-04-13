#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SQL_C_DOUBLE
// ============================================================================

TEST_CASE("DECFLOAT to SQL_C_DOUBLE", "[decfloat][conversion][c_real]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values in float64 range are fetched as SQL_C_DOUBLE
  (void)0;
  // Then SQL_C_DOUBLE returns approximately correct values
  {
    INFO("zero");
    auto val = check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch("SELECT 0::DECFLOAT"), 1);
    CHECK_THAT(val, Catch::Matchers::WithinAbs(0.0, 1e-15));
  }
  {
    INFO("positive fractional");
    auto val = check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch("SELECT 123.456::DECFLOAT"), 1);
    CHECK_THAT(val, Catch::Matchers::WithinRel(123.456));
  }
  {
    INFO("negative fractional");
    auto val = check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch("SELECT '-42.5'::DECFLOAT"), 1);
    CHECK_THAT(val, Catch::Matchers::WithinRel(-42.5));
  }
}

// ============================================================================
// SQL_C_FLOAT
// ============================================================================

TEST_CASE("DECFLOAT to SQL_C_FLOAT", "[decfloat][conversion][c_real]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values in float32 range are fetched as SQL_C_FLOAT
  (void)0;
  // Then SQL_C_FLOAT returns approximately correct values
  {
    INFO("zero");
    auto val = check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch("SELECT 0::DECFLOAT"), 1);
    CHECK(val == 0.0f);
  }
  {
    INFO("positive");
    auto val = check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);
    CHECK_THAT(static_cast<double>(val), Catch::Matchers::WithinRel(42.0));
  }
}

// ============================================================================
// PRECISION LOSS
// ============================================================================

TEST_CASE("DECFLOAT precision loss to SQL_C_DOUBLE", "[decfloat][conversion][c_real][precision]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A 38-digit DECFLOAT value is fetched as SQL_C_DOUBLE
  auto val = check_no_truncation<SQL_C_DOUBLE>(
      conn.execute_fetch("SELECT '12345678901234567890123456789012345678'::DECFLOAT"), 1);

  // Then Value is approximately correct but precision beyond float64 is lost
  CHECK_THAT(val, Catch::Matchers::WithinRel(1.2345678901234568e37, 1e-10));
}

// ============================================================================
// OVERFLOW
// ============================================================================

TEST_CASE("DECFLOAT SQL_C_FLOAT overflow", "[decfloat][conversion][c_real][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DECFLOAT value beyond float32 range but within float64 range is fetched as SQL_C_FLOAT
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  check_numeric_out_of_range<SQL_C_FLOAT>(conn.execute_fetch("SELECT '3.5E38'::DECFLOAT"), 1);
}

TEST_CASE("DECFLOAT SQL_C_DOUBLE overflow", "[decfloat][conversion][c_real][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DECFLOAT value beyond float64 range is fetched as SQL_C_DOUBLE
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  check_numeric_out_of_range<SQL_C_DOUBLE>(conn.execute_fetch("SELECT '1E+309'::DECFLOAT"), 1);
}

// ============================================================================
// SQLBindCol
// ============================================================================

TEST_CASE("DECFLOAT using SQLBindCol for SQL_C_DOUBLE", "[decfloat][conversion][c_real][bindcol]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values are fetched using SQLBindCol for SQL_C_DOUBLE
  const auto stmt = conn.createStatement();
  double val1 = 0, val2 = 0;
  SQLLEN ind1 = 0, ind2 = 0;

  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_DOUBLE, &val1, sizeof(val1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_DOUBLE, &val2, sizeof(val2), &ind2);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 0::DECFLOAT, 123.456::DECFLOAT"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Bound doubles contain correct DECFLOAT values
  CHECK(ind1 == sizeof(double));
  CHECK(val1 == 0.0);
  CHECK(ind2 == sizeof(double));
  CHECK_THAT(val2, Catch::Matchers::WithinRel(123.456));
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("DECFLOAT NULL to floating-point C types", "[decfloat][conversion][c_real][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DECFLOAT value is queried
  (void)0;
  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_DOUBLE);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_FLOAT);
}
