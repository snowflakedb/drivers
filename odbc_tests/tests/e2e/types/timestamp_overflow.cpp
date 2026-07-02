#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "overflow_helpers.hpp"

// SQL TIMESTAMP values are bounded to year 0001..9999. Snowflake's server
// will happily compute and ship out-of-range timestamps when DATEADD/etc.
// pushes the result past those bounds (verified manually: an offset of
// +8000 years lands at year 10001). The driver must reject these with
// SQL_ERROR -- the same contract the legacy ODBC driver enforces.
//
// Note: only year-overflow (>9999) is exercised here. Year-underflow
// (negative years from large negative offsets) is not a regression
// against the legacy driver -- legacy itself renders e.g.
// "-6000-09-08 12:00:00" without an error, so testing that contract
// would fail on legacy too.
namespace {
constexpr const char* kNtzOverflowQuery =
    "WITH t(c_int, c_ts) AS (SELECT 8000, '2001-09-08 12:00:00'::TIMESTAMP_NTZ) "
    "SELECT DATEADD(YEAR, c_int, c_ts) FROM t";
constexpr const char* kLtzOverflowQuery =
    "WITH t(c_int, c_ts) AS (SELECT 8000, '2001-09-08 12:00:00'::TIMESTAMP_LTZ) "
    "SELECT DATEADD(YEAR, c_int, c_ts) FROM t";
constexpr const char* kTzOverflowQuery =
    "WITH t(c_int, c_ts) AS (SELECT 8000, '2001-09-08 12:00:00 +00:00'::TIMESTAMP_TZ) "
    "SELECT DATEADD(YEAR, c_int, c_ts) FROM t";
}  // namespace

TEST_CASE("TIMESTAMP_NTZ year overflow surfaces a datetime overflow error", "[timestamp_ntz][overflow][22007]") {
  // Given a connection
  Connection conn;
  auto stmt = conn.createStatement();

  // When DATEADD pushes the timestamp past year 9999
  auto result = run_overflow_query(stmt, kNtzOverflowQuery);

  // Then the driver reports SQL_ERROR with SQLSTATE 22007 (invalid datetime format)
  INFO("which_step=\"" << result.which_step << "\" sqlstate=\"" << result.sqlstate
                       << "\" native_error=" << result.native_error << " msg=\"" << result.message << "\" rendered=\""
                       << result.rendered << "\"");
  REQUIRE(result.ret == SQL_ERROR);
  CHECK(result.sqlstate == "22007");
}

TEST_CASE("TIMESTAMP_LTZ year overflow surfaces a datetime overflow error", "[timestamp_ltz][overflow][22007]") {
  // Given a connection
  Connection conn;
  auto stmt = conn.createStatement();

  // When DATEADD pushes the timestamp past year 9999
  auto result = run_overflow_query(stmt, kLtzOverflowQuery);

  // Then the driver reports SQL_ERROR with SQLSTATE 22007 (invalid datetime format)
  INFO("which_step=\"" << result.which_step << "\" sqlstate=\"" << result.sqlstate
                       << "\" native_error=" << result.native_error << " msg=\"" << result.message << "\" rendered=\""
                       << result.rendered << "\"");
  REQUIRE(result.ret == SQL_ERROR);
  CHECK(result.sqlstate == "22007");
}

TEST_CASE("TIMESTAMP_TZ year overflow surfaces a datetime overflow error", "[timestamp_tz][overflow][22007]") {
  // Given a connection
  Connection conn;
  auto stmt = conn.createStatement();

  // When DATEADD pushes the timestamp past year 9999
  auto result = run_overflow_query(stmt, kTzOverflowQuery);

  // Then the driver reports SQL_ERROR with SQLSTATE 22007 (invalid datetime format)
  INFO("which_step=\"" << result.which_step << "\" sqlstate=\"" << result.sqlstate
                       << "\" native_error=" << result.native_error << " msg=\"" << result.message << "\" rendered=\""
                       << result.rendered << "\"");
  REQUIRE(result.ret == SQL_ERROR);
  CHECK(result.sqlstate == "22007");
}
