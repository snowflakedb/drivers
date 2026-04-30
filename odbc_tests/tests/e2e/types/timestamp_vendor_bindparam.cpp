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
  ts_in.fraction = 0;
  SQLLEN ts_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_SF_TIMESTAMP_NTZ, 29, 9,
                         &ts_in, sizeof(ts_in), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched wall-clock matches the inserted value
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_ntz_vendor WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 3);
  CHECK(ts_out.day == 15);
  CHECK(ts_out.hour == 14);
  CHECK(ts_out.minute == 30);
  CHECK(ts_out.second == 45);
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
  ts_in.fraction = 0;
  SQLLEN ts_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_SF_TIMESTAMP_LTZ, 29, 9,
                         &ts_in, sizeof(ts_in), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched value reflects the same UTC instant (session TZ = UTC, so
  // the wall-clock is preserved end-to-end). The key point is that the bind
  // succeeded against an LTZ column, exercising the LTZ logical type route.
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_ltz_vendor WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 3);
  CHECK(ts_out.day == 15);
  CHECK(ts_out.hour == 14);
  CHECK(ts_out.minute == 30);
  CHECK(ts_out.second == 45);
}

TEST_CASE("SQL_SF_TIMESTAMP_TZ binds SQL_C_TYPE_TIMESTAMP into a TIMESTAMP_TZ column as UTC",
          "[timestamp_tz][bind_fetch][vendor_codes]") {
  // SQL_TIMESTAMP_STRUCT has no offset field, so binding it to a TIMESTAMP_TZ
  // column treats the wall-clock as UTC (offset = 0 on the wire). This matches
  // the legacy Python connector's behavior for naive `datetime` values bound
  // to TIMESTAMP_TZ. Applications that need to preserve a non-UTC offset must
  // bind via SQL_C_CHAR / SQL_C_WCHAR with a `+/-HH:MM` suffix instead.

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
}
