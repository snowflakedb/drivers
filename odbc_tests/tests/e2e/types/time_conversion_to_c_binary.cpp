#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("TIME to SQL_C_BINARY", "[time][conversion][c_binary]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
}

TEST_CASE("TIME to SQL_C_BINARY midnight", "[time][conversion][c_binary]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT '00:00:00'::TIME");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
}

TEST_CASE("TIME to SQL_C_BINARY end of day", "[time][conversion][c_binary]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT '23:59:59'::TIME");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
}

TEST_CASE("TIME to SQL_C_BINARY with fractional seconds", "[time][conversion][c_binary]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT '10:30:00.123456789'::TIME");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
}

TEST_CASE("TIME to SQL_C_BINARY consistent size", "[time][conversion][c_binary]") {
  Connection conn;

  auto stmt1 = conn.execute_fetch("SELECT '14:30:45'::TIME");
  SQLCHAR buf1[256] = {};
  SQLLEN ind1 = 0;
  REQUIRE(SQLGetData(stmt1.getHandle(), 1, SQL_C_BINARY, buf1, sizeof(buf1), &ind1) == SQL_SUCCESS);

  auto stmt2 = conn.execute_fetch("SELECT '00:00:00'::TIME");
  SQLCHAR buf2[256] = {};
  SQLLEN ind2 = 0;
  REQUIRE(SQLGetData(stmt2.getHandle(), 1, SQL_C_BINARY, buf2, sizeof(buf2), &ind2) == SQL_SUCCESS);

  CHECK(ind1 == ind2);
}

TEST_CASE("TIME NULL to SQL_C_BINARY", "[time][conversion][c_binary][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::TIME");

  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
