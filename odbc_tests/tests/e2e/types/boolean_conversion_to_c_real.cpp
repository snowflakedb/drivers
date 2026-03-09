#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "macros.hpp"

// ============================================================================
// Float / Double
// ============================================================================

TEST_CASE("should convert boolean to floating point types", "[datatype][boolean][conversion][real]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then SQL_C_FLOAT should return 1.0 for TRUE and 0.0 for FALSE
  REQUIRE(check_no_truncation<SQL_C_FLOAT>(stmt, 1) == 1.0f);
  REQUIRE(check_no_truncation<SQL_C_FLOAT>(stmt, 2) == 0.0f);

  // And SQL_C_DOUBLE should return 1.0 for TRUE and 0.0 for FALSE
  REQUIRE(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == 1.0);
  REQUIRE(check_no_truncation<SQL_C_DOUBLE>(stmt, 2) == 0.0);
}

// ============================================================================
// SQL_C_NUMERIC
// ============================================================================

TEST_CASE("should convert boolean to SQL_C_NUMERIC", "[datatype][boolean][conversion][real]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then SQL_C_NUMERIC should return value 1 for TRUE and 0 for FALSE with sign=1
  auto true_numeric = check_no_truncation<SQL_C_NUMERIC>(stmt, 1);
  REQUIRE(true_numeric.sign == 1);
  REQUIRE(true_numeric.val[0] == 1);
  for (int i = 1; i < 16; ++i) {
    REQUIRE(true_numeric.val[i] == 0);
  }

  auto false_numeric = check_no_truncation<SQL_C_NUMERIC>(stmt, 2);
  REQUIRE(false_numeric.sign == 1);
  for (unsigned char i : false_numeric.val) {
    REQUIRE(i == 0);
  }
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("should handle NULL boolean with numeric and binary C types", "[datatype][boolean][conversion][real]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::BOOLEAN" is executed
  // Then SQL_C_FLOAT should return SQL_NULL_DATA indicator
  // And SQL_C_DOUBLE should return SQL_NULL_DATA indicator
  // And SQL_C_NUMERIC should return SQL_NULL_DATA indicator
  auto check_null = [&](SQLSMALLINT c_type) {
    const auto stmt = conn.execute_fetch("SELECT NULL::BOOLEAN");
    char buffer[100] = {};
    SQLLEN indicator = 0;
    const SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, c_type, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == SQL_NULL_DATA);
  };

  check_null(SQL_C_FLOAT);
  check_null(SQL_C_DOUBLE);
  check_null(SQL_C_NUMERIC);
}
