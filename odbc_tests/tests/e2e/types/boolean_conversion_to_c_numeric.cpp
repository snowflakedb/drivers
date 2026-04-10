#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("should convert boolean to SQL_C_NUMERIC", "[datatype][boolean][conversion][c_numeric]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then SQL_C_NUMERIC should return value 1 for TRUE and 0 for FALSE with sign=1
  auto true_numeric = check_no_truncation<SQL_C_NUMERIC>(stmt, 1);
  REQUIRE(true_numeric.sign == 1);
  REQUIRE(true_numeric.val[0] == 1);
  check_numeric_val_zero_from(true_numeric, 1);

  auto false_numeric = check_no_truncation<SQL_C_NUMERIC>(stmt, 2);
  REQUIRE(false_numeric.sign == 1);
  check_numeric_val_zero_from(false_numeric, 0);
}

TEST_CASE("should handle NULL boolean with SQL_C_NUMERIC", "[datatype][boolean][conversion][c_numeric]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT NULL::BOOLEAN");

  // Then SQL_C_NUMERIC should return SQL_NULL_DATA indicator
  check_null_via_get_data(stmt, 1, SQL_C_NUMERIC);
}
