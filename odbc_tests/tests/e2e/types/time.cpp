#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <optional>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

SQL_TIME_STRUCT make_time(SQLUSMALLINT hour, SQLUSMALLINT minute, SQLUSMALLINT second) {
  return SQL_TIME_STRUCT{hour, minute, second};
}

void require_time(const SQL_TIME_STRUCT& got, SQLUSMALLINT hour, SQLUSMALLINT minute, SQLUSMALLINT second) {
  REQUIRE(got.hour == hour);
  REQUIRE(got.minute == minute);
  REQUIRE(got.second == second);
}

void require_time_ts(const SQL_TIMESTAMP_STRUCT& got, SQLUSMALLINT hour, SQLUSMALLINT minute, SQLUSMALLINT second,
                     SQLUINTEGER fraction) {
  REQUIRE(got.hour == hour);
  REQUIRE(got.minute == minute);
  REQUIRE(got.second == second);
  REQUIRE(got.fraction == fraction);
}

void require_sql_type(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT expected) {
  SQLSMALLINT data_type = 0;
  SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, nullptr, nullptr, nullptr);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(data_type == expected);
}

void millis_to_hmsf(int millis, SQLUSMALLINT& hour, SQLUSMALLINT& minute, SQLUSMALLINT& second, SQLUINTEGER& fraction) {
  hour = static_cast<SQLUSMALLINT>(millis / 3600000);
  int rem = millis % 3600000;
  minute = static_cast<SQLUSMALLINT>(rem / 60000);
  rem %= 60000;
  second = static_cast<SQLUSMALLINT>(rem / 1000);
  fraction = static_cast<SQLUINTEGER>((rem % 1000) * 1000000);
}

void require_sequential_times(const StatementHandleWrapper& stmt, int expected_rows) {
  int row_count = 0;
  while (true) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);
    SQLUSMALLINT hour = 0;
    SQLUSMALLINT minute = 0;
    SQLUSMALLINT second = 0;
    SQLUINTEGER fraction = 0;
    millis_to_hmsf(row_count, hour, minute, second, fraction);
    INFO("row=" << row_count);
    require_time_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), hour, minute, second, fraction);
    row_count++;
  }
  REQUIRE(row_count == expected_rows);
}

void bind_time(const StatementHandleWrapper& stmt, SQLUSMALLINT param, SQL_TIME_STRUCT& value, SQLLEN& indicator) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_TYPE_TIME, 0, 0,
                                   &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

TEST_CASE("should cast time values to appropriate type", "[time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME" is executed
  const auto stmt = conn.execute_fetch("SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME");

  // Then All values should be returned as appropriate type
  require_sql_type(stmt, 1, SQL_TYPE_TIME);
  require_sql_type(stmt, 2, SQL_TYPE_TIME);
  require_sql_type(stmt, 3, SQL_TYPE_TIME);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 10, 30, 0);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 2), 0, 0, 0);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 3), 23, 59, 59);
}

TEST_CASE("should select time values", "[time]") {
  // Given Snowflake client is logged in
  Connection conn;

  struct Case {
    const char* name;
    const char* query;
  };
  const Case cases[] = {
      {"basic", "SELECT '10:30:00'::TIME, '14:45:30'::TIME, '23:59:59'::TIME"},
      {"midnight", "SELECT '00:00:00'::TIME"},
      {"microseconds", "SELECT '10:30:00.123456'::TIME"},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);

    // When Query "SELECT <query_values>" is executed
    const auto stmt = conn.execute_fetch(test_case.query);

    // Then Result should contain times <expected_values>
    if (std::string(test_case.name) == "basic") {
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 10, 30, 0);
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 2), 14, 45, 30);
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 3), 23, 59, 59);
    } else if (std::string(test_case.name) == "midnight") {
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 0, 0, 0);
    } else {
      require_time_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 10, 30, 0, 123456000);
    }
  }
}

TEST_CASE("should handle time precision scale", "[time]") {
  // Given Snowflake client is logged in
  Connection conn;

  struct Case {
    const char* name;
    const char* query;
    SQLUINTEGER fraction;
  };
  const Case cases[] = {
      {"0", "SELECT '10:30:00.123456789'::TIME(0)", 0},
      {"3", "SELECT '10:30:00.123456789'::TIME(3)", 123000000},
      {"6", "SELECT '10:30:00.123456789'::TIME(6)", 123456000},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);

    // When Query "SELECT '10:30:00.123456789'::TIME(<scale>)" is executed
    const auto stmt = conn.execute_fetch(test_case.query);

    // Then Result should contain [<expected>]
    if (test_case.fraction == 0) {
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 10, 30, 0);
    } else {
      require_time_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 10, 30, 0, test_case.fraction);
    }
  }
}

TEST_CASE("should preserve nanosecond precision for time", "[time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '10:30:00.123456789'::TIME" is executed
  const auto stmt = conn.execute_fetch("SELECT '10:30:00.123456789'::TIME");

  // Then Result should contain [10:30:00.123456789]
  require_time_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 10, 30, 0, 123456789);
}

TEST_CASE("should handle NULL values for time", "[time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME" is executed
  const auto stmt = conn.execute_fetch("SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME");

  // Then Result should contain [10:30:00, NULL, 23:59:59]
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 10, 30, 0);
  REQUIRE(get_data_optional<SQL_C_TYPE_TIME>(stmt, 2) == std::nullopt);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 3), 23, 59, 59);
}

TEST_CASE("should download large result set with multiple chunks for time", "[time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) as t FROM
  // TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY t" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                sqlchar("SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, "
                                        "'00:00:00'::TIME) as t FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY t"),
                                SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 100000 sequentially increasing time values from 00:00:00
  require_sequential_times(stmt, 100000);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select values from table for time", "[time]") {
  // Given Snowflake client is logged in

  struct Case {
    const char* name;
    const char* insert_sql;
  };
  const Case cases[] = {
      {"basic", "INSERT INTO time_table VALUES ('10:30:00'::TIME), ('14:45:30'::TIME), ('23:59:59'::TIME)"},
      {"midnight", "INSERT INTO time_table VALUES ('00:00:00'::TIME), ('12:00:00'::TIME), ('23:59:59'::TIME)"},
      {"microseconds", "INSERT INTO time_table VALUES ('10:30:00'::TIME), ('10:30:00.123456'::TIME)"},
      {"null", "INSERT INTO time_table VALUES (NULL), ('10:30:00'::TIME)"},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE time_table (col TIME)");

    // And Table with TIME column exists with values <insert_values>
    conn.execute(test_case.insert_sql);

    // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    const auto stmt = conn.createStatement();
    SQLRETURN ret =
        SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM time_table ORDER BY col NULLS LAST"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    // Then Result should contain times <expected_values>
    if (std::string(test_case.name) == "basic") {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 10, 30, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 14, 45, 30);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 23, 59, 59);
    } else if (std::string(test_case.name) == "midnight") {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 0, 0, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 12, 0, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 23, 59, 59);
    } else if (std::string(test_case.name) == "microseconds") {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_time_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 10, 30, 0, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_time_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 10, 30, 0, 123456000);
    } else {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 10, 30, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      REQUIRE(get_data_optional<SQL_C_TYPE_TIME>(stmt, 1) == std::nullopt);
    }
    ret = SQLFetch(stmt.getHandle());
    REQUIRE(ret == SQL_NO_DATA);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should download large result set with multiple chunks from table for time",
                 "[time]") {
  // Given Snowflake client is logged in

  // And Table with TIME column exists with 100000 sequential time values starting from 00:00:00
  conn.execute("CREATE TEMPORARY TABLE time_large_table (col TIME)");
  conn.execute(
      "INSERT INTO time_large_table "
      "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) "
      "FROM TABLE(GENERATOR(ROWCOUNT => 100000))");

  // When Query "SELECT * FROM <table> ORDER BY col" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM time_large_table ORDER BY col"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 100000 sequentially increasing time values from 00:00:00
  require_sequential_times(stmt, 100000);
}

TEST_CASE("should select time using parameter binding", "[time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::TIME, ?::TIME, ?::TIME" is executed with bound time values [10:30:00, 14:45:30, 23:59:59]
  const auto stmt = conn.createStatement();
  SQL_TIME_STRUCT val1 = make_time(10, 30, 0);
  SQL_TIME_STRUCT val2 = make_time(14, 45, 30);
  SQL_TIME_STRUCT val3 = make_time(23, 59, 59);
  SQLLEN ind1 = sizeof(val1);
  SQLLEN ind2 = sizeof(val2);
  SQLLEN ind3 = sizeof(val3);
  bind_time(stmt, 1, val1, ind1);
  bind_time(stmt, 2, val2, ind2);
  bind_time(stmt, 3, val3, ind3);

  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::TIME, ?::TIME, ?::TIME"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain times [10:30:00, 14:45:30, 23:59:59]
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 10, 30, 0);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 2), 14, 45, 30);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 3), 23, 59, 59);
}

TEST_CASE("should select null time using parameter binding", "[time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::TIME" is executed with bound NULL value
  const auto stmt = conn.createStatement();
  SQLLEN ind = SQL_NULL_DATA;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_TYPE_TIME, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::TIME"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [NULL]
  REQUIRE(get_data_optional<SQL_C_TYPE_TIME>(stmt, 1) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should insert time using parameter binding", "[time]") {
  // Given Snowflake client is logged in

  // And Table with TIME column exists
  conn.execute("CREATE TEMPORARY TABLE time_bind_table (col TIME)");

  // When Time values [00:00:00, 10:30:00, 14:45:30, 23:59:59] are inserted using binding
  SQL_TIME_STRUCT values[] = {make_time(0, 0, 0), make_time(10, 30, 0), make_time(14, 45, 30), make_time(23, 59, 59)};
  const auto insert_stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(insert_stmt.getHandle(), sqlchar("INSERT INTO time_bind_table VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, insert_stmt);
  SQL_TIME_STRUCT val = {};
  SQLLEN ind = sizeof(val);
  bind_time(insert_stmt, 1, val, ind);
  for (const auto& value : values) {
    val = value;
    ret = SQLExecute(insert_stmt.getHandle());
    REQUIRE_ODBC(ret, insert_stmt);
  }

  // And Query "SELECT * FROM <table> ORDER BY col" is executed
  const auto stmt = conn.createStatement();
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM time_bind_table ORDER BY col"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain times [00:00:00, 10:30:00, 14:45:30, 23:59:59]
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 0, 0, 0);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 10, 30, 0);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 14, 45, 30);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_time(get_data<SQL_C_TYPE_TIME>(stmt, 1), 23, 59, 59);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should insert time with fractional seconds using parameter binding", "[time]") {
  // Given Snowflake client is logged in

  // And Table with TIME column exists
  conn.execute("CREATE TEMPORARY TABLE time_frac_table (col TIME)");

  // When Time values [10:30:00.123456, 14:45:30.654321] are bulk-inserted using multirow binding
  constexpr SQLULEN num_rows = 2;
  char values[num_rows][17] = {"10:30:00.123456", "14:45:30.654321"};
  SQLLEN indicators[num_rows] = {SQL_NTS, SQL_NTS};
  SQLUSMALLINT param_status[num_rows] = {};
  SQLULEN params_processed = 0;

  const auto insert_stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status, 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLBindParameter(insert_stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_TYPE_TIME, 16, 6, values,
                         sizeof(values[0]), indicators);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLExecDirect(insert_stmt.getHandle(), sqlchar("INSERT INTO time_frac_table VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, insert_stmt);
  REQUIRE(params_processed == num_rows);

  // And Query "SELECT * FROM <table> ORDER BY col" is executed
  const auto stmt = conn.createStatement();
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM time_frac_table ORDER BY col"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain times [10:30:00.123456, 14:45:30.654321]
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_time_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 10, 30, 0, 123456000);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_time_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 14, 45, 30, 654321000);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}
