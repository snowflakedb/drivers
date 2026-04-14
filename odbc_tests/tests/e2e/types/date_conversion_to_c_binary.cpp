#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("DATE to SQL_C_BINARY", "[date][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  SQL_DATE_STRUCT date = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &date, sizeof(date), &indicator);

  // Then SQL_DATE_STRUCT fields match the source date
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_DATE_STRUCT));
  CHECK(date.year == 2024);
  CHECK(date.month == 1);
  CHECK(date.day == 15);
}

TEST_CASE("DATE to SQL_C_BINARY struct field verification", "[date][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  {
    // When epoch DATE is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '1970-01-01'::DATE");
    SQL_DATE_STRUCT date = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &date, sizeof(date), &indicator);
    // Then SQL_DATE_STRUCT year, month, day fields match
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(date.year == 1970);
    CHECK(date.month == 1);
    CHECK(date.day == 1);
  }

  {
    // When pre-epoch DATE is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '1960-06-15'::DATE");
    SQL_DATE_STRUCT date = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &date, sizeof(date), &indicator);
    // Then SQL_DATE_STRUCT year, month, day fields match
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(date.year == 1960);
    CHECK(date.month == 6);
    CHECK(date.day == 15);
  }

  {
    // When leap day DATE is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '2000-02-29'::DATE");
    SQL_DATE_STRUCT date = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &date, sizeof(date), &indicator);
    // Then SQL_DATE_STRUCT year, month, day fields match
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(date.year == 2000);
    CHECK(date.month == 2);
    CHECK(date.day == 29);
  }

  {
    // When end of year DATE is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '1999-12-31'::DATE");
    SQL_DATE_STRUCT date = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &date, sizeof(date), &indicator);
    // Then SQL_DATE_STRUCT year, month, day fields match
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(date.year == 1999);
    CHECK(date.month == 12);
    CHECK(date.day == 31);
  }

  {
    // When far future DATE is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '9999-12-31'::DATE");
    SQL_DATE_STRUCT date = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &date, sizeof(date), &indicator);
    // Then SQL_DATE_STRUCT year, month, day fields match
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(date.year == 9999);
    CHECK(date.month == 12);
    CHECK(date.day == 31);
  }

  {
    // When far past DATE is fetched as SQL_C_BINARY
    auto stmt = conn.execute_fetch("SELECT '0001-01-01'::DATE");
    SQL_DATE_STRUCT date = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &date, sizeof(date), &indicator);
    // Then SQL_DATE_STRUCT year, month, day fields match
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(date.year == 1);
    CHECK(date.month == 1);
    CHECK(date.day == 1);
  }
}

TEST_CASE("DATE to SQL_C_BINARY exact buffer fit", "[date][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched into a buffer of exactly sizeof(SQL_DATE_STRUCT)
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  SQLCHAR buffer[sizeof(SQL_DATE_STRUCT)] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned with correct struct fields
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_DATE_STRUCT));

  SQL_DATE_STRUCT date;
  std::memcpy(&date, buffer, sizeof(date));
  CHECK(date.year == 2024);
  CHECK(date.month == 1);
  CHECK(date.day == 15);
}

TEST_CASE("DATE to SQL_C_BINARY buffer too small", "[date][conversion][c_binary][22003]") {
  SKIP_OLD_DRIVER("BD#39", "old driver does not return 22003 for undersized binary buffer");
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched into a buffer smaller than sizeof(SQL_DATE_STRUCT)
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  SQLCHAR buffer[2] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "22003");
}

TEST_CASE("DATE to SQL_C_BINARY consistent size", "[date][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Different DATE values are fetched as SQL_C_BINARY
  auto stmt1 = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  SQL_DATE_STRUCT d1 = {};
  SQLLEN ind1 = 0;
  REQUIRE(SQLGetData(stmt1.getHandle(), 1, SQL_C_BINARY, &d1, sizeof(d1), &ind1) == SQL_SUCCESS);

  auto stmt2 = conn.execute_fetch("SELECT '1960-06-15'::DATE");
  SQL_DATE_STRUCT d2 = {};
  SQLLEN ind2 = 0;
  REQUIRE(SQLGetData(stmt2.getHandle(), 1, SQL_C_BINARY, &d2, sizeof(d2), &ind2) == SQL_SUCCESS);

  // Then The indicator equals sizeof(SQL_DATE_STRUCT) for all dates
  CHECK(ind1 == ind2);
  CHECK(ind1 == sizeof(SQL_DATE_STRUCT));
}

TEST_CASE("DATE NULL to SQL_C_BINARY", "[date][conversion][c_binary][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DATE value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
