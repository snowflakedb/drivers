#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

// Per ODBC spec (SQL to C: Date): when DATE is converted to SQL_C_TYPE_TIMESTAMP,
// the date fields are populated and the time fields are set to zero.

TEST_CASE("DATE to SQL_C_TYPE_TIMESTAMP", "[date][conversion][c_timestamp]") {
  Connection conn;

  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-01-15'::DATE"), 1);

  CHECK(ts.year == 2024);
  CHECK(ts.month == 1);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 0);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
  CHECK(ts.fraction == 0);
}

TEST_CASE("DATE to SQL_C_TYPE_TIMESTAMP boundary values", "[date][conversion][c_timestamp]") {
  Connection conn;

  {
    INFO("pre-epoch");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1960-06-15'::DATE"), 1);
    CHECK(ts.year == 1960);
    CHECK(ts.month == 6);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("leap day");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2000-02-29'::DATE"), 1);
    CHECK(ts.year == 2000);
    CHECK(ts.month == 2);
    CHECK(ts.day == 29);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("epoch");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1970-01-01'::DATE"), 1);
    CHECK(ts.year == 1970);
    CHECK(ts.month == 1);
    CHECK(ts.day == 1);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("end of year");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1999-12-31'::DATE"), 1);
    CHECK(ts.year == 1999);
    CHECK(ts.month == 12);
    CHECK(ts.day == 31);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("first day of year");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2025-01-01'::DATE"), 1);
    CHECK(ts.year == 2025);
    CHECK(ts.month == 1);
    CHECK(ts.day == 1);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }
}

TEST_CASE("DATE NULL to SQL_C_TYPE_TIMESTAMP", "[date][conversion][c_timestamp][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  check_null_via_get_data(stmt, 1, SQL_C_TYPE_TIMESTAMP);
}
