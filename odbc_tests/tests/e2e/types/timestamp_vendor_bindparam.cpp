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
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
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
  // With session TZ == UTC, the bound wall-clock and the displayed
  // wall-clock collapse to the same instant: the driver emits a bare
  // `"YYYY-MM-DD HH:MM:SS.fff"` string with `type=TEXT`, the server
  // interprets it in the session TZ (UTC), and stores the same UTC
  // instant. The non-UTC session test below pins the session-TZ
  // dependence explicitly.

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

TEST_CASE("SQL_SF_TIMESTAMP_LTZ wall-clock is interpreted in the session timezone on bind",
          "[timestamp_ltz][bind_fetch][vendor_codes][session_tz]") {
  // The load-bearing claim of TIMESTAMP_LTZ on the bind side is that the
  // driver emits a **bare** wall-clock string (`"YYYY-MM-DD HH:MM:SS.FFFFFFFFF"`)
  // tagged as `type=TEXT` on the wire, matching the legacy 3.16.0 driver's
  // JSON-bind path in `Snowflake-odbc/Source/DataEngine/SFQueryExecutor.cpp`.
  // The Snowflake server then interprets that wall-clock literal in the
  // **session** timezone when coercing into TIMESTAMP_LTZ:
  //
  //     stored_utc = bound_wall_clock - session_tz_offset
  //
  // (The process-local-offset format used by `BindUploader.cpp` is a
  // *separate* CSV-staging path; JSON binds always go through the bare
  // SFQueryExecutor TEXT path and pick up session-TZ semantics from the
  // server, not from `localtime` on the client.)
  //
  // This test pins both halves of the contract:
  //
  //   1. The stored UTC instant equals the bound wall-clock interpreted in
  //      the **session** TZ -- i.e. shifted by `-session_offset`.
  //   2. Re-displayed in the same session TZ on fetch, the wall-clock
  //      round-trips unchanged (with the session offset visible in the
  //      `TZHTZM` slot).

  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'Asia/Kolkata'");
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_ltz_wallclock (id INT, ts TIMESTAMP_LTZ)");
  auto stmt = conn.createStatement();

  // When A SQL_TIMESTAMP_STRUCT wall-clock is bound with the vendor LTZ code
  // in a session whose TZ is Asia/Kolkata (+05:30)
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

  // Then The stored UTC instant equals the bound wall-clock interpreted in
  // the session TZ -- 14:30 Asia/Kolkata = 09:00 UTC (shift of -05:30).
  auto utc_stmt = conn.execute_fetch(
      "SELECT TO_VARCHAR(CONVERT_TIMEZONE('UTC', ts), 'YYYY-MM-DD HH24:MI:SS.FF9') "
      "FROM ts_ltz_wallclock WHERE id = 1");
  CHECK(get_data<SQL_C_CHAR>(utc_stmt, 1) == "2024-03-15 09:00:45.123456789");

  // And Re-displayed in the session TZ (Asia/Kolkata, +05:30), the
  // wall-clock round-trips back to the bound value with the session
  // offset visible in the `TZHTZM` slot.
  auto wc_stmt = conn.execute_fetch(
      "SELECT TO_VARCHAR(ts, 'YYYY-MM-DD HH24:MI:SS.FF9 TZHTZM') FROM ts_ltz_wallclock WHERE id = 1");
  CHECK(get_data<SQL_C_CHAR>(wc_stmt, 1) == "2024-03-15 14:30:45.123456789 +0530");
}

TEST_CASE("SQL_SF_TIMESTAMP_TZ binds SQL_C_TYPE_TIMESTAMP into a TIMESTAMP_TZ column as UTC",
          "[timestamp_tz][bind_fetch][vendor_codes]") {
  // SQL_TIMESTAMP_STRUCT has no offset field, so binding it to a TIMESTAMP_TZ
  // column treats the wall-clock as UTC (offset = 0 on the wire). This matches
  // the legacy Python connector's behavior for naive `datetime` values bound
  // to TIMESTAMP_TZ. Applications that need to preserve a non-UTC offset must
  // bind via SQL_C_CHAR / SQL_C_WCHAR with a `+/-HH:MM` suffix instead.
  //
  // The legacy 3.16.0 ODBC driver REJECTS this binding with SQLSTATE HY000 /
  // NativeError 40620 ("Logic error during conversion") rather than accepting
  // the naive value. The new driver implements the spec/Python-connector
  // semantics; documenting the divergence in BehaviorDifferences.yaml under
  // BD#51 and skipping on the reference driver here.
  SKIP_OLD_DRIVER("BD#51",
                  "Legacy driver returns 40620 for SQL_C_TYPE_TIMESTAMP -> SQL_SF_TIMESTAMP_TZ; new driver "
                  "accepts and binds as UTC (offset=0) per Python connector parity");

  // Given Snowflake client is logged in and a temporary table with a TIMESTAMP_TZ column
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
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
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_SF_TIMESTAMP_TZ, 35, 9, &ts_in,
                         sizeof(ts_in), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched UTC wall-clock matches the inserted naive value (session
  // TIMEZONE = UTC, so SQL_C_TYPE_TIMESTAMP read returns the same components).
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_tz_vendor WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 3);
  CHECK(ts_out.day == 15);
  CHECK(ts_out.hour == 14);
  CHECK(ts_out.minute == 30);
  CHECK(ts_out.second == 45);
}

TEST_CASE("SQL_SF_TIMESTAMP_TZ binds SQL_C_CHAR with offset suffix and round-trips the offset",
          "[timestamp_tz][bind_fetch][vendor_codes]") {
  // Spec-correct path for binding a TIMESTAMP_TZ value with a non-UTC offset:
  // SQL_C_CHAR with a `+/-HH:MM` suffix. The driver parses the offset, emits
  // the legacy `<epoch_ns> <offset_minutes_plus_1440>` two-token wire format,
  // and the server stores the original instant alongside the offset.
  //
  // The legacy 3.16.0 ODBC driver REJECTS this binding with SQLSTATE HY000 /
  // NativeError 40620 ("Logic error during conversion") — it does not parse
  // the `+/-HH:MM` suffix from SQL_C_CHAR for vendor TZ. The new driver
  // implements offset parsing per the universal driver design. Documented
  // under BD#51; skip on the reference driver.
  SKIP_OLD_DRIVER("BD#51",
                  "Legacy driver returns 40620 for SQL_C_CHAR with `+/-HH:MM` -> SQL_SF_TIMESTAMP_TZ; new "
                  "driver parses and preserves the offset on wire");

  // Given Snowflake client is logged in and a temporary table with a TIMESTAMP_TZ column
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_tz_char_bind (id INT, ts TIMESTAMP_TZ)");
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_tz_char_bind VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // When A SQL_C_CHAR ISO-8601 string with a `+05:30` suffix is bound with the vendor TZ ParameterType
  std::string ts_in = "2024-03-15 14:30:45 +05:30";
  SQLLEN ts_ind = static_cast<SQLLEN>(ts_in.size());
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_SF_TIMESTAMP_TZ, 35, 0,
                         const_cast<char*>(ts_in.data()), static_cast<SQLLEN>(ts_in.size()), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched UTC wall-clock matches the offset-applied instant: the
  // `+05:30` suffix moved 14:30:45 back to 09:00:45 UTC before storage.
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_tz_char_bind WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 3);
  CHECK(ts_out.day == 15);
  CHECK(ts_out.hour == 9);
  CHECK(ts_out.minute == 0);
  CHECK(ts_out.second == 45);

  // And The server-side rendering of the TZ value preserves *both* the
  // wall-clock and the original `+05:30` offset on the wire. The check
  // above only proves the UTC instant is right, which a buggy
  // implementation that always emitted `offset=0` (and let the server
  // derive UTC from session-TZ shenanigans) would also satisfy. Pinning
  // the formatted readback here closes that gap end-to-end -- a
  // regression that drops the offset on the wire reverts to
  // `2024-03-15 09:00:45 +00:00` and fails this assertion. See PR #1005
  // review on `timestamp_vendor_bindparam.cpp:223`.
  auto str_stmt = conn.execute_fetch(
      "SELECT TO_VARCHAR(ts, 'YYYY-MM-DD HH24:MI:SS TZH:TZM') FROM ts_tz_char_bind WHERE id = 1");
  CHECK(get_data<SQL_C_CHAR>(str_stmt, 1) == "2024-03-15 09:00:45 +05:30");
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
  SKIP_OLD_DRIVER("BD#50",
                  "Old driver leaks the raw vendor code (2000/2001/2002) through SQLDescribeParam and "
                  "SQLGetDescField(IPD, SQL_DESC_TYPE); new driver normalises to SQL_TYPE_TIMESTAMP (93) "
                  "per MS ODBC spec.");

  // Given Snowflake client is logged in and a prepared two-parameter INSERT
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

  // When parameter #2 is bound in turn with each Snowflake vendor TIMESTAMP code
  // Then SQLDescribeParam reports the standard SQL_TYPE_TIMESTAMP (93) -- never
  // the raw vendor code -- proving the vendor opt-in is input-only.
  //
  // We also cross-check via `SQLGetDescField(IPD, SQL_DESC_TYPE)` because
  // `SQLDescribeParam` goes through the unixODBC Driver Manager (which can
  // post-process the driver's response via `__map_type(MAP_SQL_D2DM)`) while
  // `SQLGetDescField` on a descriptor handle obtained from `SQL_ATTR_IMP_PARAM_DESC`
  // is forwarded to the driver verbatim. If the two routes disagree, the
  // discrepancy pinpoints DM remapping; if they agree, the value is what the
  // driver actually stored.
  SQLHDESC ipd = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_PARAM_DESC, &ipd, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  REQUIRE(ipd != nullptr);

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

    SQLSMALLINT ipd_type = 0;
    ret = SQLGetDescField(ipd, 2, SQL_DESC_TYPE, &ipd_type, 0, nullptr);
    REQUIRE_ODBC(ret, stmt);

    INFO("SQLDescribeParam returned data_type = " << data_type
                                                  << "; SQLGetDescField(IPD, SQL_DESC_TYPE) returned = " << ipd_type);
    CHECK(data_type == SQL_TYPE_TIMESTAMP);  // 93, not the 2000/2001/2002 we bound with
    CHECK(ipd_type == SQL_TYPE_TIMESTAMP);   // verifies the driver actually stored 93 in the IPD
  }
}
