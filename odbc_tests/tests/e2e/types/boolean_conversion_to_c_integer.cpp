#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("should convert boolean to signed integer c_type", "[datatype][boolean][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then <c_type> should return 1 for TRUE and 0 for FALSE
  SECTION("SQL_C_LONG") {
    REQUIRE(check_no_truncation<SQL_C_LONG>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_LONG>(stmt, 2) == 0);
  }
  SECTION("SQL_C_SLONG") {
    REQUIRE(check_no_truncation<SQL_C_SLONG>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_SLONG>(stmt, 2) == 0);
  }
  SECTION("SQL_C_SHORT") {
    REQUIRE(check_no_truncation<SQL_C_SHORT>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_SHORT>(stmt, 2) == 0);
  }
  SECTION("SQL_C_SSHORT") {
    REQUIRE(check_no_truncation<SQL_C_SSHORT>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_SSHORT>(stmt, 2) == 0);
  }
  SECTION("SQL_C_TINYINT") {
    REQUIRE(check_no_truncation<SQL_C_TINYINT>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_TINYINT>(stmt, 2) == 0);
  }
  SECTION("SQL_C_STINYINT") {
    REQUIRE(check_no_truncation<SQL_C_STINYINT>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_STINYINT>(stmt, 2) == 0);
  }
  SECTION("SQL_C_SBIGINT") {
    REQUIRE(check_no_truncation<SQL_C_SBIGINT>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_SBIGINT>(stmt, 2) == 0);
  }
}

TEST_CASE("should convert boolean to unsigned integer c_type", "[datatype][boolean][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then <c_type> should return 1 for TRUE and 0 for FALSE
  SECTION("SQL_C_ULONG") {
    REQUIRE(check_no_truncation<SQL_C_ULONG>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_ULONG>(stmt, 2) == 0);
  }
  SECTION("SQL_C_USHORT") {
    REQUIRE(check_no_truncation<SQL_C_USHORT>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_USHORT>(stmt, 2) == 0);
  }
  SECTION("SQL_C_UTINYINT") {
    REQUIRE(check_no_truncation<SQL_C_UTINYINT>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_UTINYINT>(stmt, 2) == 0);
  }
  SECTION("SQL_C_UBIGINT") {
    REQUIRE(check_no_truncation<SQL_C_UBIGINT>(stmt, 1) == 1);
    REQUIRE(check_no_truncation<SQL_C_UBIGINT>(stmt, 2) == 0);
  }
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("should handle NULL boolean with integer C types", "[datatype][boolean][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::BOOLEAN" is executed
  auto check_null = [&](SQLSMALLINT c_type) {
    const auto stmt = conn.execute_fetch("SELECT NULL::BOOLEAN");
    char buffer[100] = {};
    SQLLEN indicator = 0;
    const SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, c_type, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == SQL_NULL_DATA);
  };

  // Then All integer C type conversions should return SQL_NULL_DATA indicator
  check_null(SQL_C_LONG);
  check_null(SQL_C_SLONG);
  check_null(SQL_C_ULONG);
  check_null(SQL_C_SHORT);
  check_null(SQL_C_SSHORT);
  check_null(SQL_C_USHORT);
  check_null(SQL_C_TINYINT);
  check_null(SQL_C_STINYINT);
  check_null(SQL_C_UTINYINT);
  check_null(SQL_C_SBIGINT);
  check_null(SQL_C_UBIGINT);
}
