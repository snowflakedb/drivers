#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("should convert boolean to c_type", "[datatype][boolean][conversion][real]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto query = "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN";

  // Then <c_type> should return 1.0 for TRUE and 0.0 for FALSE
  {
    INFO("SQL_C_FLOAT");
    const auto stmt = conn.execute_fetch(query);
    REQUIRE(check_no_truncation<SQL_C_FLOAT>(stmt, 1) == 1.0f);
    REQUIRE(check_no_truncation<SQL_C_FLOAT>(stmt, 2) == 0.0f);
  }
  {
    INFO("SQL_C_DOUBLE");
    const auto stmt = conn.execute_fetch(query);
    REQUIRE(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == 1.0);
    REQUIRE(check_no_truncation<SQL_C_DOUBLE>(stmt, 2) == 0.0);
  }
}

TEST_CASE("should handle NULL boolean with floating point c_type", "[datatype][boolean][conversion][real]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::BOOLEAN" is executed
  const auto query = "SELECT NULL::BOOLEAN";

  // Then <c_type> should return SQL_NULL_DATA indicator
  {
    INFO("SQL_C_FLOAT");
    check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_FLOAT);
  }
  {
    INFO("SQL_C_DOUBLE");
    check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_DOUBLE);
  }
}
