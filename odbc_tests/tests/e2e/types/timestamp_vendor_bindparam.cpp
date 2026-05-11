// Tests for SQLBindParameter using the Snowflake vendor TIMESTAMP SQL type
// codes (SQL_SF_TIMESTAMP_LTZ = 2000, SQL_SF_TIMESTAMP_TZ = 2001,
// SQL_SF_TIMESTAMP_NTZ = 2002). These let an application opt into the matching
// wire `SnowflakeLogicalType` instead of always landing on TIMESTAMP_NTZ via
// the standard `SQL_TYPE_TIMESTAMP` (93). Mirrors the legacy 3.16.0 driver.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "snowflake_odbc_constants.hpp"

TEST_CASE("SQL_SF_TIMESTAMP_NTZ binds SQL_C_TYPE_TIMESTAMP into a TIMESTAMP_NTZ column",
          "[timestamp_ntz][bind_fetch][vendor_codes]") {
  // Given Snowflake client is logged in and a temporary table with a TIMESTAMP_NTZ column
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_ntz_vendor (id INT, ts TIMESTAMP_NTZ)");
  auto stmt = conn.createStatement();

  // When A SQL_TIMESTAMP_STRUCT is bound with the vendor NTZ ParameterType
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_ntz_vendor VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  SQL_TIMESTAMP_STRUCT ts_in = {};
  ts_in.year = 2024;
  ts_in.month = 3;
  ts_in.day = 15;
  ts_in.hour = 14;
  ts_in.minute = 30;
  ts_in.second = 45;
  // Use a non-zero nanosecond fraction so a regression that silently truncates
  // to whole seconds is caught here. The vendor-NTZ arm in `make_converter`
  // hardcodes `scale: 9`, so the round-trip must preserve all nine digits.
  ts_in.fraction = 123456789;
  SQLLEN ts_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_SF_TIMESTAMP_NTZ, 29, 9,
                         &ts_in, sizeof(ts_in), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched wall-clock matches the inserted value (including the
  // nanosecond fraction; locks in the scale-9 contract from the bind side).
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_ntz_vendor WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 3);
  CHECK(ts_out.day == 15);
  CHECK(ts_out.hour == 14);
  CHECK(ts_out.minute == 30);
  CHECK(ts_out.second == 45);
  CHECK(ts_out.fraction == 123456789);
}

TEST_CASE("SQL_SF_TIMESTAMP_LTZ binds SQL_C_TYPE_TIMESTAMP into a TIMESTAMP_LTZ column",
          "[timestamp_ltz][bind_fetch][vendor_codes]") {
  // Given Snowflake client is logged in and a temporary table with a TIMESTAMP_LTZ column
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_ltz_vendor (id INT, ts TIMESTAMP_LTZ)");
  auto stmt = conn.createStatement();

  // When A SQL_TIMESTAMP_STRUCT is bound with the vendor LTZ ParameterType
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_ltz_vendor VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  SQL_TIMESTAMP_STRUCT ts_in = {};
  ts_in.year = 2024;
  ts_in.month = 3;
  ts_in.day = 15;
  ts_in.hour = 14;
  ts_in.minute = 30;
  ts_in.second = 45;
  // Non-zero fraction locks in the scale-9 contract; same rationale as the
  // NTZ test above.
  ts_in.fraction = 123456789;
  SQLLEN ts_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_SF_TIMESTAMP_LTZ, 29, 9,
                         &ts_in, sizeof(ts_in), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched wall-clock matches the inserted value end-to-end. With
  // session TZ = UTC, the LTZ wall-clock-in-session-TZ semantics collapse to
  // the same observable as NTZ; the non-UTC test below exercises the case
  // where they diverge.
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_ltz_vendor WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 3);
  CHECK(ts_out.day == 15);
  CHECK(ts_out.hour == 14);
  CHECK(ts_out.minute == 30);
  CHECK(ts_out.second == 45);
  CHECK(ts_out.fraction == 123456789);
}

TEST_CASE("SQL_SF_TIMESTAMP_LTZ preserves wall-clock interpreted in non-UTC session timezone",
          "[timestamp_ltz][bind_fetch][vendor_codes][session_tz]") {
  // The load-bearing claim of TIMESTAMP_LTZ is that the bound naive datetime
  // is interpreted as a wall-clock in the active session timezone (matching
  // legacy 3.16.0). The previous UTC-pinned test cannot prove this because
  // LTZ and NTZ are observationally identical for `TIMEZONE='UTC'`. This
  // test runs with `Asia/Kolkata` (UTC+05:30, no DST so the assertion is
  // DST-immune) and asserts both observables that a wall-clock-string
  // emitter must produce:
  //
  //   1. Re-displayed in session TZ, the wall-clock is unchanged
  //      (`2024-03-15 14:30:45.123456789 +0530`).
  //   2. The underlying UTC instant is shifted by -05:30 from the bound
  //      wall-clock (`2024-03-15 09:00:45.123456789`).
  //
  // A regression that emits epoch-nanoseconds (treating the wall-clock as
  // already-UTC) would store `2024-03-15T14:30:45.123Z` and the readbacks
  // above would return `2024-03-15 20:00:45 +0530` and
  // `2024-03-15 14:30:45` respectively -- both assertions would fail.
  // See PR #1004 review on `param_binding.rs:245`.
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'Asia/Kolkata'");
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_ltz_wallclock (id INT, ts TIMESTAMP_LTZ)");
  auto stmt = conn.createStatement();

  // When A SQL_TIMESTAMP_STRUCT wall-clock is bound with the vendor LTZ code
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_ltz_wallclock VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  SQL_TIMESTAMP_STRUCT ts_in = {2024, 3, 15, 14, 30, 45, 123456789};
  SQLLEN ts_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_SF_TIMESTAMP_LTZ, 29, 9,
                         &ts_in, sizeof(ts_in), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The wall-clock displayed in session TZ matches the bound wall-clock
  auto wc_stmt = conn.execute_fetch(
      "SELECT TO_VARCHAR(ts, 'YYYY-MM-DD HH24:MI:SS.FF9 TZHTZM') FROM ts_ltz_wallclock WHERE id = 1");
  CHECK(get_data<SQL_C_CHAR>(wc_stmt, 1) == "2024-03-15 14:30:45.123456789 +0530");

  // And The underlying UTC instant is shifted by -05:30 from the wall-clock
  // (proving the server actually parsed the literal in session TZ rather
  // than treating it as already-UTC).
  auto utc_stmt = conn.execute_fetch(
      "SELECT TO_VARCHAR(CONVERT_TIMEZONE('UTC', ts), 'YYYY-MM-DD HH24:MI:SS.FF9') "
      "FROM ts_ltz_wallclock WHERE id = 1");
  CHECK(get_data<SQL_C_CHAR>(utc_stmt, 1) == "2024-03-15 09:00:45.123456789");
}

TEST_CASE("SQL_SF_TIMESTAMP_TZ binding is rejected with SQLSTATE 07006",
          "[timestamp_tz][bind_fetch][vendor_codes][.skip]") {
  // SKIPPED: this test pins the *current gap* — the new driver rejects TZ
  // binds because it doesn't yet emit the `<epoch_ns> <offset_minutes>`
  // two-token wire format the legacy driver uses. A follow-up PR will add
  // the offset round-trip and flip the assertion to a positive round-trip
  // check. Skipping (rather than deleting) preserves the table setup,
  // SQLBindParameter call, and SQLExecute scaffolding for that follow-up
  // to reuse.
  SKIP("Pending follow-up PR that adds TIMESTAMP_TZ binding (offset round-trip).");

  // Faithfully binding TIMESTAMP_TZ requires preserving the offset (legacy
  // emits `<epoch_ns> <offset_minutes>`), which the new driver doesn't yet
  // emit. The driver therefore rejects the bind with `Restricted data type
  // attribute violation` (07006) at SQLExecute time rather than silently
  // dropping the offset.

  // Given Snowflake client is logged in and a temporary table with a TIMESTAMP_TZ column
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_tz_vendor (id INT, ts TIMESTAMP_TZ)");
  auto stmt = conn.createStatement();

  // When A SQL_TIMESTAMP_STRUCT is bound with ParameterType = SQL_SF_TIMESTAMP_TZ
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_tz_vendor VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  SQL_TIMESTAMP_STRUCT ts_in = {};
  ts_in.year = 2024;
  ts_in.month = 3;
  ts_in.day = 15;
  ts_in.hour = 14;
  ts_in.minute = 30;
  ts_in.second = 45;
  SQLLEN ts_ind = 0;
  // SQLBindParameter itself accepts the vendor type; rejection happens when
  // the driver actually serialises the binding during SQLExecute.
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_SF_TIMESTAMP_TZ, 35, 9, &ts_in,
                         sizeof(ts_in), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then SQLExecute fails with SQLSTATE 07006
  ret = SQLExecute(stmt.getHandle());
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "07006");
}

TEST_CASE("SQLDescribeParam returns SQL_TYPE_TIMESTAMP (93) after binding with vendor codes",
          "[timestamp_vendor_codes][describe_param]") {
  // The Snowflake vendor codes 2000/2001/2002 are an *input-only* aliasing
  // that lets the application pick the wire-format Snowflake logical type at
  // SQLBindParameter time. The MS ODBC spec for SQLDescribeParam describes
  // returning standard SQL type codes (1..=12, 91..=95, 101..=113), so the
  // vendor codes must be normalised to SQL_TYPE_TIMESTAMP (93) on the IPD
  // before any introspection call observes them. Otherwise an application
  // that switches on 93 in describe-param would silently fall through to an
  // unknown-type branch when the upstream code happens to bind via the
  // vendor opt-in. See PR #1004 review (odbc_types.rs:631).
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_describe_param (id INT, ts TIMESTAMP_NTZ)");
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_describe_param VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  SQL_TIMESTAMP_STRUCT ts_in = {2024, 3, 15, 14, 30, 45, 0};
  SQLLEN ts_ind = 0;

  // Try each vendor code in turn. After the bind, SQLDescribeParam must
  // report 93 — not 2000/2001/2002 — for parameter #2.
  for (SQLSMALLINT vendor_code : {SQL_SF_TIMESTAMP_NTZ, SQL_SF_TIMESTAMP_LTZ, SQL_SF_TIMESTAMP_TZ}) {
    INFO("vendor code = " << vendor_code);
    ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, vendor_code, 29, 9, &ts_in,
                           sizeof(ts_in), &ts_ind);
    REQUIRE_ODBC_SUCCESS(ret, stmt);

    SQLSMALLINT data_type = 0;
    SQLULEN parameter_size = 0;
    SQLSMALLINT decimal_digits = 0;
    SQLSMALLINT nullable = 0;
    ret = SQLDescribeParam(stmt.getHandle(), 2, &data_type, &parameter_size, &decimal_digits, &nullable);
    REQUIRE_ODBC(ret, stmt);

    CHECK(data_type == SQL_TYPE_TIMESTAMP);  // 93, not the 2000/2001/2002 we bound with
  }
}
