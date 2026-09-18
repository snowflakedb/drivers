#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <optional>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

SQL_DATE_STRUCT make_date(SQLSMALLINT year, SQLUSMALLINT month, SQLUSMALLINT day) {
  return SQL_DATE_STRUCT{year, month, day};
}

void require_date(const SQL_DATE_STRUCT& got, SQLSMALLINT year, SQLUSMALLINT month, SQLUSMALLINT day) {
  REQUIRE(got.year == year);
  REQUIRE(got.month == month);
  REQUIRE(got.day == day);
}

void require_sql_type(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT expected) {
  SQLSMALLINT data_type = 0;
  SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, nullptr, nullptr, nullptr);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(data_type == expected);
}

bool is_leap_year(SQLSMALLINT year) {
  if (year % 400 == 0) {
    return true;
  }
  if (year % 100 == 0) {
    return false;
  }
  return year % 4 == 0;
}

SQLUSMALLINT days_in_month(SQLSMALLINT year, SQLUSMALLINT month) {
  static const SQLUSMALLINT kDays[] = {0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
  if (month == 2 && is_leap_year(year)) {
    return 29;
  }
  return kDays[month];
}

void increment_date(SQL_DATE_STRUCT& date) {
  date.day++;
  if (date.day > days_in_month(date.year, date.month)) {
    date.day = 1;
    date.month++;
    if (date.month > 12) {
      date.month = 1;
      date.year++;
    }
  }
}

void require_sequential_dates(const StatementHandleWrapper& stmt, int expected_rows, SQL_DATE_STRUCT start) {
  int row_count = 0;
  SQL_DATE_STRUCT expected = start;
  while (true) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);
    INFO("row=" << row_count);
    require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), expected.year, expected.month, expected.day);
    increment_date(expected);
    row_count++;
  }
  REQUIRE(row_count == expected_rows);
}

void bind_date(const StatementHandleWrapper& stmt, SQLUSMALLINT param, SQL_DATE_STRUCT& value, SQLLEN& indicator) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_TYPE_DATE, 0, 0,
                                   &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

TEST_CASE("should cast date values to appropriate type", "[date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
  const auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE");

  // Then All values should be returned as DATE type
  require_sql_type(stmt, 1, SQL_TYPE_DATE);
  require_sql_type(stmt, 2, SQL_TYPE_DATE);
  require_sql_type(stmt, 3, SQL_TYPE_DATE);

  // And No precision loss should occur
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 2024, 1, 15);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 2), 1970, 1, 1);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 3), 1999, 12, 31);
}

TEST_CASE("should select date literals", "[date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
  const auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE");

  // Then Result should contain dates [2024-01-15, 1970-01-01, 1999-12-31]
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 2024, 1, 15);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 2), 1970, 1, 1);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 3), 1999, 12, 31);
}

TEST_CASE("should select epoch and pre-epoch dates", "[date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE" is executed
  const auto stmt = conn.execute_fetch("SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE");

  // Then Result should contain dates [1970-01-01, 1969-12-31, 1900-01-01]
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 1970, 1, 1);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 2), 1969, 12, 31);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 3), 1900, 1, 1);
}

TEST_CASE("should select historical and boundary dates", "[date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE" is executed
  const auto stmt = conn.execute_fetch("SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE");

  // Then Result should contain dates [0001-01-01, 1582-10-15, 9999-12-31]
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 1, 1, 1);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 2), 1582, 10, 15);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 3), 9999, 12, 31);
}

TEST_CASE("should handle NULL values for date", "[date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
  const auto stmt = conn.execute_fetch("SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE");

  // Then Result should contain [NULL, 2024-01-15, NULL]
  REQUIRE(get_data_optional<SQL_C_TYPE_DATE>(stmt, 1) == std::nullopt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 2), 2024, 1, 15);
  REQUIRE(get_data_optional<SQL_C_TYPE_DATE>(stmt, 3) == std::nullopt);
}

TEST_CASE("should download large result set for date", "[date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) as d FROM
  // TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY d" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(),
                    sqlchar("SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, "
                            "'1970-01-01'::DATE) as d FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY d"),
                    SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
  require_sequential_dates(stmt, 100000, make_date(1970, 1, 1));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select dates from table", "[date]") {
  // Given Snowflake client is logged in

  // And Table with DATE column exists with values ['2024-01-15', '1970-01-01', '1999-12-31']
  conn.execute("CREATE TEMPORARY TABLE date_table (col DATE)");
  conn.execute("INSERT INTO date_table VALUES ('2024-01-15'::DATE), ('1970-01-01'::DATE), ('1999-12-31'::DATE)");

  // When Query "SELECT * FROM <table> ORDER BY col" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM date_table ORDER BY col"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 1970, 1, 1);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 1999, 12, 31);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 2024, 1, 15);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select dates with NULL from table", "[date]") {
  // Given Snowflake client is logged in

  // And Table with DATE column exists with values ['2024-01-15', NULL, '1999-12-31']
  conn.execute("CREATE TEMPORARY TABLE date_null_table (col DATE)");
  conn.execute("INSERT INTO date_null_table VALUES ('2024-01-15'::DATE), (NULL), ('1999-12-31'::DATE)");

  // When Query "SELECT * FROM <table> ORDER BY col" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM date_null_table ORDER BY col"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [1999-12-31, 2024-01-15, NULL]
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 1999, 12, 31);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 2024, 1, 15);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_data_optional<SQL_C_TYPE_DATE>(stmt, 1) == std::nullopt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select historical and boundary dates from table", "[date]") {
  // Given Snowflake client is logged in

  // And Table with DATE column exists with values ['0001-01-01', '0100-03-01', '1582-10-15', '9999-12-31']
  conn.execute("CREATE TEMPORARY TABLE date_hist_table (col DATE)");
  conn.execute(
      "INSERT INTO date_hist_table VALUES ('0001-01-01'::DATE), ('0100-03-01'::DATE), "
      "('1582-10-15'::DATE), ('9999-12-31'::DATE)");

  // When Query "SELECT * FROM <table> ORDER BY col" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM date_hist_table ORDER BY col"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain dates [0001-01-01, 0100-03-01, 1582-10-15, 9999-12-31]
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 1, 1, 1);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 100, 3, 1);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 1582, 10, 15);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 9999, 12, 31);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should download large result set for date from table", "[date]") {
  // Given Snowflake client is logged in

  // And Table with DATE column exists with 100000 sequential dates starting from 1970-01-01
  conn.execute("CREATE TEMPORARY TABLE date_large_table (col DATE)");
  conn.execute(
      "INSERT INTO date_large_table "
      "SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) "
      "FROM TABLE(GENERATOR(ROWCOUNT => 100000))");

  // When Query "SELECT * FROM <table> ORDER BY col" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM date_large_table ORDER BY col"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
  require_sequential_dates(stmt, 100000, make_date(1970, 1, 1));
}

TEST_CASE("should select date using parameter binding", "[date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::DATE, ?::DATE, ?::DATE" is executed with bound date values [2024-01-15, 1970-01-01,
  // 1999-12-31]
  const auto stmt = conn.createStatement();
  SQL_DATE_STRUCT val1 = make_date(2024, 1, 15);
  SQL_DATE_STRUCT val2 = make_date(1970, 1, 1);
  SQL_DATE_STRUCT val3 = make_date(1999, 12, 31);
  SQLLEN ind1 = sizeof(val1);
  SQLLEN ind2 = sizeof(val2);
  SQLLEN ind3 = sizeof(val3);
  bind_date(stmt, 1, val1, ind1);
  bind_date(stmt, 2, val2, ind2);
  bind_date(stmt, 3, val3, ind3);

  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::DATE, ?::DATE, ?::DATE"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [2024-01-15, 1970-01-01, 1999-12-31]
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 2024, 1, 15);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 2), 1970, 1, 1);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 3), 1999, 12, 31);
}

TEST_CASE("should select null date using parameter binding", "[date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::DATE" is executed with bound NULL value
  const auto stmt = conn.createStatement();
  SQLLEN ind = SQL_NULL_DATA;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_TYPE_DATE, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::DATE"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [NULL]
  REQUIRE(get_data_optional<SQL_C_TYPE_DATE>(stmt, 1) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should insert date using parameter binding", "[date]") {
  // Given Snowflake client is logged in

  // And Table with DATE column exists
  conn.execute("CREATE TEMPORARY TABLE date_bind_table (col DATE)");

  // When Date values [2024-01-15, 1970-01-01, 1999-12-31] are inserted using parameter binding
  SQL_DATE_STRUCT values[] = {make_date(2024, 1, 15), make_date(1970, 1, 1), make_date(1999, 12, 31)};
  const auto insert_stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(insert_stmt.getHandle(), sqlchar("INSERT INTO date_bind_table VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, insert_stmt);
  SQL_DATE_STRUCT val = {};
  SQLLEN ind = sizeof(val);
  bind_date(insert_stmt, 1, val, ind);
  for (const auto& value : values) {
    val = value;
    ret = SQLExecute(insert_stmt.getHandle());
    REQUIRE_ODBC(ret, insert_stmt);
  }

  // And Query "SELECT * FROM <table> ORDER BY col" is executed
  const auto stmt = conn.createStatement();
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM date_bind_table ORDER BY col"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 1970, 1, 1);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 1999, 12, 31);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_date(get_data<SQL_C_TYPE_DATE>(stmt, 1), 2024, 1, 15);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}
