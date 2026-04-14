#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

// Per ODBC spec (SQL to C: Time): when TIME is converted to SQL_C_TYPE_TIMESTAMP,
// the date fields are set to the current date and the fractional seconds field is
// set to zero.

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP", "[time][conversion][c_timestamp]") {
  Connection conn;

  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '14:30:45'::TIME"), 1);

  CHECK(ts.year > 0);
  CHECK(ts.month >= 1);
  CHECK(ts.month <= 12);
  CHECK(ts.day >= 1);
  CHECK(ts.day <= 31);
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP midnight", "[time][conversion][c_timestamp]") {
  Connection conn;

  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '00:00:00'::TIME"), 1);

  CHECK(ts.year > 0);
  CHECK(ts.month >= 1);
  CHECK(ts.day >= 1);
  CHECK(ts.hour == 0);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP end of day", "[time][conversion][c_timestamp]") {
  Connection conn;

  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '23:59:59'::TIME"), 1);

  CHECK(ts.year > 0);
  CHECK(ts.hour == 23);
  CHECK(ts.minute == 59);
  CHECK(ts.second == 59);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP with fractional truncation", "[time][conversion][c_timestamp][truncation]") {
  SKIP_OLD_DRIVER("BD#42", "old driver does not report 01S07 for fractional seconds");
  Connection conn;

  auto ts = check_fractional_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '14:30:45.123'::TIME"), 1);

  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP with high-precision fractional truncation",
          "[time][conversion][c_timestamp][truncation]") {
  SKIP_OLD_DRIVER("BD#42", "old driver does not report 01S07 for fractional seconds");
  Connection conn;

  auto ts =
      check_fractional_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '10:30:00.123456789'::TIME"), 1);

  CHECK(ts.hour == 10);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 0);
}

TEST_CASE("TIME NULL to SQL_C_TYPE_TIMESTAMP", "[time][conversion][c_timestamp][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::TIME");

  check_null_via_get_data(stmt, 1, SQL_C_TYPE_TIMESTAMP);
}
