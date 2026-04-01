#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// BASIC SQL_C_NUMERIC CONVERSIONS
// ============================================================================

TEST_CASE("DECFLOAT to SQL_C_NUMERIC", "[decfloat][conversion][c_numeric]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values are fetched as SQL_C_NUMERIC
  (void)0;
  // Then SQL_NUMERIC_STRUCT fields match expected sign, precision, scale, and val
  {
    INFO("positive integer");
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 42);
    check_numeric_val_zero_from(numeric, 1);
  }

  {
    INFO("negative value");
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT '-123'::DECFLOAT"), 1);
    CHECK(numeric.sign == 0);
    CHECK(numeric.val[0] == 123);
    check_numeric_val_zero_from(numeric, 1);
  }

  {
    INFO("zero");
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 0::DECFLOAT"), 1);
    CHECK(numeric.sign == 1);
    check_numeric_val_zero_from(numeric, 0);
  }
}

// ============================================================================
// FRACTIONAL TRUNCATION
// ============================================================================

TEST_CASE("DECFLOAT fractional truncation to SQL_C_NUMERIC", "[decfloat][conversion][c_numeric]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A fractional DECFLOAT value is fetched as SQL_C_NUMERIC with default scale=0
  auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 123.456::DECFLOAT"), 1);

  // Then Value is truncated to integer part with SQLSTATE 01S07
  CHECK(numeric_val_to_ull(numeric) == 123);
}

// ============================================================================
// OVERFLOW (extreme exponent)
// ============================================================================

TEST_CASE("DECFLOAT extreme exponent to SQL_C_NUMERIC returns 22003", "[decfloat][conversion][c_numeric][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DECFLOAT value with exponent exceeding u128 range is fetched as SQL_C_NUMERIC
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  {
    INFO("1e100 overflows u128 scaled value");
    check_numeric_out_of_range<SQL_C_NUMERIC>(conn.execute_fetch("SELECT '1e100'::DECFLOAT"), 1);
  }

  {
    INFO("negative extreme exponent");
    check_numeric_out_of_range<SQL_C_NUMERIC>(conn.execute_fetch("SELECT '-1e100'::DECFLOAT"), 1);
  }
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("DECFLOAT NULL to SQL_C_NUMERIC", "[decfloat][conversion][c_numeric][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DECFLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DECFLOAT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_NUMERIC);
}
