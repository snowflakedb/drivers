#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// BASIC BIT CONVERSIONS
// ============================================================================

TEST_CASE("DECFLOAT to SQL_C_BIT spec compliance", "[decfloat][conversion][c_bit]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Various DECFLOAT values are fetched as SQL_C_BIT
  (void)0;
  // Then 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003
  {
    INFO("0 succeeds");
    CHECK(check_no_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 0::DECFLOAT"), 1) == 0);
  }
  {
    INFO("1 succeeds");
    CHECK(check_no_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 1::DECFLOAT"), 1) == 1);
  }
  {
    INFO("value 2 returns 22003");
    check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 2::DECFLOAT"), 1);
  }
  {
    INFO("negative integer returns 22003");
    check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT '-1'::DECFLOAT"), 1);
  }
  {
    INFO("fractional 0.5 truncates to 0 with 01S07");
    CHECK(check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 0.5::DECFLOAT"), 1) == 0);
  }
  {
    INFO("fractional 1.5 truncates to 1 with 01S07");
    CHECK(check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 1.5::DECFLOAT"), 1) == 1);
  }
  {
    INFO("negative fractional returns 22003");
    check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT '-0.5'::DECFLOAT"), 1);
  }
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("DECFLOAT NULL to SQL_C_BIT", "[decfloat][conversion][c_bit][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DECFLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DECFLOAT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BIT);
}
