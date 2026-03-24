#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("SQL_C_BIT spec compliance", "[fixed][conversion][c_bit]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  {
    INFO("0 and 1 succeed");
    // When Various NUMBER/DECIMAL values are fetched as SQL_C_BIT
    auto stmt = conn.execute_fetch("SELECT 0::NUMBER(10,0), 1::NUMBER(10,0), 0.00::NUMBER(10,2)");
    // Then Values 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 1) == 0);
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 2) == 1);
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 3) == 0);
  }

  {
    INFO("negative integer is out of range (22003)");
    // When Various NUMBER/DECIMAL values are fetched as SQL_C_BIT
    auto stmt = conn.execute_fetch("SELECT -1::NUMBER(10,0)");
    // Then Values 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003
    check_numeric_out_of_range<SQL_C_BIT>(stmt, 1);
  }

  {
    INFO("value 2 returns 22003");
    // When Various NUMBER/DECIMAL values are fetched as SQL_C_BIT
    auto stmt = conn.execute_fetch("SELECT 2::NUMBER(1,0)");
    // Then Values 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003
    check_numeric_out_of_range<SQL_C_BIT>(stmt, 1);
  }

  {
    INFO("negative fractional value returns 22003");
    // When Various NUMBER/DECIMAL values are fetched as SQL_C_BIT
    auto stmt = conn.execute_fetch("SELECT -0.5::DECIMAL(3,1)");
    // Then Values 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003
    check_numeric_out_of_range<SQL_C_BIT>(stmt, 1);
  }

  {
    INFO("fractional 0.5 truncates to 0 with 01S07");
    // When Various NUMBER/DECIMAL values are fetched as SQL_C_BIT
    auto stmt = conn.execute_fetch("SELECT 0.5::DECIMAL(3,1)");
    // Then Values 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003
    CHECK(check_fractional_truncation<SQL_C_BIT>(stmt, 1) == 0);
  }

  {
    INFO("fractional 1.5 truncates to 1 with 01S07");
    // When Various NUMBER/DECIMAL values are fetched as SQL_C_BIT
    auto stmt = conn.execute_fetch("SELECT 1.5::DECIMAL(3,1)");
    // Then Values 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003
    CHECK(check_fractional_truncation<SQL_C_BIT>(stmt, 1) == 1);
  }

  {
    INFO("exact 1.0 does NOT produce warning");
    // When Various NUMBER/DECIMAL values are fetched as SQL_C_BIT
    auto stmt = conn.execute_fetch("SELECT 1.00::DECIMAL(5,2)");
    // Then Values 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 1) == 1);
  }
}

TEST_CASE("NUMBER NULL to SQL_C_BIT", "[fixed][conversion][c_bit][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL NUMBER value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,0)");
  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BIT);
}
