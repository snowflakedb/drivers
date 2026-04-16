#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("TIME to SQL_C_BINARY", "[time][conversion][c_binary]") {
  SKIP_OLD_DRIVER("BD#43", "old driver does not support TIME to SQL_C_BINARY conversion");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME value is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  SQL_TIME_STRUCT time = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &time, sizeof(time), &indicator);

  // Then SQL_TIME_STRUCT fields match the source time
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_TIME_STRUCT));
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 45);
}

TEST_CASE("TIME to SQL_C_BINARY struct field verification", "[time][conversion][c_binary]") {
  SKIP_OLD_DRIVER("BD#43", "old driver does not support TIME to SQL_C_BINARY conversion");
  // Given Snowflake client is logged in
  Connection conn;

  {
    // When midnight TIME is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '00:00:00'::TIME");
    SQL_TIME_STRUCT time = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &time, sizeof(time), &indicator);
    // Then SQL_TIME_STRUCT fields match
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(time.hour == 0);
    CHECK(time.minute == 0);
    CHECK(time.second == 0);
  }

  {
    // When end of day TIME is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '23:59:59'::TIME");
    SQL_TIME_STRUCT time = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &time, sizeof(time), &indicator);
    // Then SQL_TIME_STRUCT fields match
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(time.hour == 23);
    CHECK(time.minute == 59);
    CHECK(time.second == 59);
  }

  {
    // When single-digit TIME is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '01:02:03'::TIME");
    SQL_TIME_STRUCT time = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &time, sizeof(time), &indicator);
    // Then SQL_TIME_STRUCT fields match
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(time.hour == 1);
    CHECK(time.minute == 2);
    CHECK(time.second == 3);
  }

  {
    // When fractional TIME is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '10:30:00.123456789'::TIME");
    SQL_TIME_STRUCT time = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &time, sizeof(time), &indicator);
    // Then SQL_TIME_STRUCT fields match (fractional seconds dropped)
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(time.hour == 10);
    CHECK(time.minute == 30);
    CHECK(time.second == 0);
  }
}

TEST_CASE("TIME to SQL_C_BINARY exact buffer fit", "[time][conversion][c_binary]") {
  SKIP_OLD_DRIVER("BD#43", "old driver does not support TIME to SQL_C_BINARY conversion");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME value is fetched into a buffer of exactly sizeof(SQL_TIME_STRUCT)
  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  SQLCHAR buffer[sizeof(SQL_TIME_STRUCT)] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned with correct struct fields
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_TIME_STRUCT));

  SQL_TIME_STRUCT time;
  std::memcpy(&time, buffer, sizeof(time));
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 45);
}

TEST_CASE("TIME to SQL_C_BINARY buffer too small", "[time][conversion][c_binary][22003]") {
  SKIP_OLD_DRIVER("BD#43", "old driver does not support TIME to SQL_C_BINARY conversion");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME value is fetched into a buffer smaller than sizeof(SQL_TIME_STRUCT)
  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  SQLCHAR buffer[2] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "22003");
}

TEST_CASE("TIME to SQL_C_BINARY consistent size", "[time][conversion][c_binary]") {
  SKIP_OLD_DRIVER("BD#43", "old driver does not support TIME to SQL_C_BINARY conversion");
  // Given Snowflake client is logged in
  Connection conn;

  // When Different TIME values are fetched as SQL_C_BINARY
  auto stmt1 = conn.execute_fetch("SELECT '14:30:45'::TIME");
  SQL_TIME_STRUCT t1 = {};
  SQLLEN ind1 = 0;
  REQUIRE(SQLGetData(stmt1.getHandle(), 1, SQL_C_BINARY, &t1, sizeof(t1), &ind1) == SQL_SUCCESS);

  auto stmt2 = conn.execute_fetch("SELECT '00:00:00'::TIME");
  SQL_TIME_STRUCT t2 = {};
  SQLLEN ind2 = 0;
  REQUIRE(SQLGetData(stmt2.getHandle(), 1, SQL_C_BINARY, &t2, sizeof(t2), &ind2) == SQL_SUCCESS);

  // Then The indicator equals sizeof(SQL_TIME_STRUCT) for all times
  CHECK(ind1 == ind2);
  CHECK(ind1 == sizeof(SQL_TIME_STRUCT));
}

TEST_CASE("TIME NULL to SQL_C_BINARY", "[time][conversion][c_binary][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIME value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIME");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
