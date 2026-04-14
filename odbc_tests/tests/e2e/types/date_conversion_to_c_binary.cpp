#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("DATE to SQL_C_BINARY", "[date][conversion][c_binary]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
}

TEST_CASE("DATE to SQL_C_BINARY boundary values", "[date][conversion][c_binary]") {
  Connection conn;

  {
    INFO("epoch");
    auto stmt = conn.execute_fetch("SELECT '1970-01-01'::DATE");
    SQLCHAR buffer[256] = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
  }

  {
    INFO("pre-epoch");
    auto stmt = conn.execute_fetch("SELECT '1960-06-15'::DATE");
    SQLCHAR buffer[256] = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
  }

  {
    INFO("leap day");
    auto stmt = conn.execute_fetch("SELECT '2000-02-29'::DATE");
    SQLCHAR buffer[256] = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
  }

  {
    INFO("end of year");
    auto stmt = conn.execute_fetch("SELECT '1999-12-31'::DATE");
    SQLCHAR buffer[256] = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
  }
}

TEST_CASE("DATE to SQL_C_BINARY consistent size", "[date][conversion][c_binary]") {
  Connection conn;

  auto stmt1 = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  SQLCHAR buf1[256] = {};
  SQLLEN ind1 = 0;
  REQUIRE(SQLGetData(stmt1.getHandle(), 1, SQL_C_BINARY, buf1, sizeof(buf1), &ind1) == SQL_SUCCESS);

  auto stmt2 = conn.execute_fetch("SELECT '1960-06-15'::DATE");
  SQLCHAR buf2[256] = {};
  SQLLEN ind2 = 0;
  REQUIRE(SQLGetData(stmt2.getHandle(), 1, SQL_C_BINARY, buf2, sizeof(buf2), &ind2) == SQL_SUCCESS);

  CHECK(ind1 == ind2);
}

TEST_CASE("DATE NULL to SQL_C_BINARY", "[date][conversion][c_binary][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
