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

TEST_CASE("TIMESTAMP_NTZ round-trip via SQL_C_TYPE_TIMESTAMP bind and fetch",
          "[timestamp_ntz][bind_fetch][round_trip]") {
  // Given Snowflake client is logged in and a temporary table with a TIMESTAMP_NTZ column exists
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'");
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_ntz_rt (id INT, ts TIMESTAMP_NTZ)");
  auto stmt = conn.createStatement();

  // When A SQL_TIMESTAMP_STRUCT value is inserted via SQLBindParameter and then fetched back
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_ntz_rt VALUES (?, ?)"), SQL_NTS);
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
  ts_in.fraction = 123456789;
  SQLLEN ts_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29, 9, &ts_in,
                         sizeof(ts_in), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched SQL_TIMESTAMP_STRUCT matches the inserted value
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_ntz_rt WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 3);
  CHECK(ts_out.day == 15);
  CHECK(ts_out.hour == 14);
  CHECK(ts_out.minute == 30);
  CHECK(ts_out.second == 45);
  CHECK(ts_out.fraction == 123456789);
}

TEST_CASE("TIMESTAMP_NTZ round-trip via SQL_C_CHAR string bind", "[timestamp_ntz][bind_fetch][round_trip]") {
  // Given Snowflake client is logged in and a temporary table exists
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'");
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_ntz_str (id INT, ts TIMESTAMP_NTZ)");
  auto stmt = conn.createStatement();

  // When A timestamp string is inserted via SQL_C_CHAR and then fetched back as SQL_C_TYPE_TIMESTAMP
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_ntz_str VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  char ts_str[] = "2024-06-15 23:59:59.500000000";
  SQLLEN ts_ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_TYPE_TIMESTAMP, 29, 9, ts_str,
                         sizeof(ts_str), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched struct matches the inserted string value
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_ntz_str WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 6);
  CHECK(ts_out.day == 15);
  CHECK(ts_out.hour == 23);
  CHECK(ts_out.minute == 59);
  CHECK(ts_out.second == 59);
  CHECK(ts_out.fraction == 500000000);
}

TEST_CASE("TIMESTAMP_NTZ round-trip NULL via bind parameter", "[timestamp_ntz][bind_fetch][null]") {
  // Given Snowflake client is logged in and a temporary table exists
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'");
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_ntz_null (id INT, ts TIMESTAMP_NTZ)");
  auto stmt = conn.createStatement();

  // When A NULL timestamp is inserted via SQL_NULL_DATA indicator
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_ntz_null VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  SQLLEN ts_ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29, 9, nullptr,
                         0, &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched value should be NULL
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_ntz_null WHERE id = 1");
  check_null_via_get_data(select_stmt, 1, SQL_C_TYPE_TIMESTAMP);
}

TEST_CASE("TIMESTAMP_NTZ round-trip multiple rows with re-execution", "[timestamp_ntz][bind_fetch][round_trip]") {
  // Given Snowflake client is logged in and a temporary table exists
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'");
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_ntz_multi (id INT, ts TIMESTAMP_NTZ)");
  auto stmt = conn.createStatement();

  // When Multiple rows are inserted via repeated execution with changed bound values
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_ntz_multi VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 0;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  SQL_TIMESTAMP_STRUCT ts = {};
  SQLLEN ts_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29, 9, &ts,
                         sizeof(ts), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ts.year = 2000;
  ts.month = 1;
  ts.day = 1;
  ts.hour = 0;
  ts.minute = 0;
  ts.second = 0;
  ts.fraction = 0;
  id = 1;
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  REQUIRE_ODBC(ret, stmt);

  ts.year = 1999;
  ts.month = 12;
  ts.day = 31;
  ts.hour = 23;
  ts.minute = 59;
  ts.second = 59;
  ts.fraction = 999000000;
  id = 2;
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Both rows should be retrievable with correct values
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_ntz_multi ORDER BY id");
  auto ts1 = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts1.year == 2000);
  CHECK(ts1.month == 1);
  CHECK(ts1.day == 1);
  CHECK(ts1.hour == 0);

  ret = SQLFetch(select_stmt.getHandle());
  REQUIRE_ODBC(ret, select_stmt);
  auto ts2 = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts2.year == 1999);
  CHECK(ts2.month == 12);
  CHECK(ts2.day == 31);
  CHECK(ts2.hour == 23);
  CHECK(ts2.minute == 59);
  CHECK(ts2.second == 59);
  CHECK(ts2.fraction == 999000000);
}

TEST_CASE("TIMESTAMP_LTZ round-trip via SQL_C_TYPE_TIMESTAMP bind and fetch",
          "[timestamp_ltz][bind_fetch][round_trip]") {
  // Given Snowflake client is logged in and a temporary table with a TIMESTAMP_LTZ column exists
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_ltz_rt (id INT, ts TIMESTAMP_LTZ)");
  auto stmt = conn.createStatement();

  // When A SQL_TIMESTAMP_STRUCT value is inserted via SQLBindParameter and then fetched back
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_ltz_rt VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id, 0, &id_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  SQL_TIMESTAMP_STRUCT ts_in = {};
  ts_in.year = 2024;
  ts_in.month = 7;
  ts_in.day = 4;
  ts_in.hour = 18;
  ts_in.minute = 15;
  ts_in.second = 30;
  ts_in.fraction = 500000000;
  SQLLEN ts_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29, 9, &ts_in,
                         sizeof(ts_in), &ts_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched SQL_TIMESTAMP_STRUCT matches the inserted value
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_ltz_rt WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 7);
  CHECK(ts_out.day == 4);
  CHECK(ts_out.hour == 18);
  CHECK(ts_out.minute == 15);
  CHECK(ts_out.second == 30);
  CHECK(ts_out.fraction == 500000000);
}

TEST_CASE("TIMESTAMP_TZ round-trip via SQL_C_TYPE_TIMESTAMP bind and fetch", "[timestamp_tz][bind_fetch][round_trip]") {
  // Given Snowflake client is logged in and a temporary table with a TIMESTAMP_TZ column exists
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE ts_tz_rt (id INT, ts TIMESTAMP_TZ)");

  // When A timestamp with an explicit timezone offset is inserted and then fetched back
  conn.execute("INSERT INTO ts_tz_rt VALUES (1, '2024-03-15 14:30:45.123456789 +05:30'::TIMESTAMP_TZ)");

  // Then The fetched SQL_TIMESTAMP_STRUCT contains the UTC-converted value
  auto select_stmt = conn.execute_fetch("SELECT ts FROM ts_tz_rt WHERE id = 1");
  auto ts_out = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(select_stmt, 1);
  CHECK(ts_out.year == 2024);
  CHECK(ts_out.month == 3);
  CHECK(ts_out.day == 15);
  CHECK(ts_out.hour == 9);
  CHECK(ts_out.minute == 0);
  CHECK(ts_out.second == 45);
  CHECK(ts_out.fraction == 123456789);
}
