#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <optional>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "timestamp_e2e.hpp"

namespace {

void bind_char(const StatementHandleWrapper& stmt, SQLUSMALLINT param, char* value, SQLLEN& indicator) {
  const auto len = static_cast<SQLLEN>(std::strlen(value));
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, len, 0, value,
                                   len + 1, &indicator);
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

TEST_CASE("should cast timestamp_tz values to appropriate type", "[timestamp_tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ" is executed
  const auto stmt = conn.execute_fetch("SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ");

  // Then All values should be returned as appropriate type
  require_sql_timestamp(stmt, 1);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0);

  // And Values should have timezone info
  INFO("SQL_TIMESTAMP_STRUCT cannot carry timezone");
}

TEST_CASE("should select timestamp_tz values", "[timestamp_tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  struct Case {
    const char* name;
    const char* query;
  };
  const Case cases[] = {
      {"basic", "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, '2024-06-20 14:45:30 -08:00'::TIMESTAMP_TZ"},
      {"epoch", "SELECT '1970-01-01 00:00:00 +00:00'::TIMESTAMP_TZ"},
      {"microseconds", "SELECT '2024-01-15 10:30:00.123456 +05:00'::TIMESTAMP_TZ"},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);

    // When Query "SELECT <query_values>" is executed
    const auto stmt = conn.execute_fetch(test_case.query);

    // Then Result should contain timestamps <expected_values>
    if (std::string(test_case.name) == "basic") {
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 2), 2024, 6, 20, 22, 45, 30);
    } else if (std::string(test_case.name) == "epoch") {
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1970, 1, 1, 0, 0, 0);
    } else {
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0, 123456000);
    }

    // And Values should have timezone info
    INFO("SQL_TIMESTAMP_STRUCT cannot carry timezone");
  }
}

TEST_CASE("should select sub-second timestamp_tz values before epoch", "[timestamp_tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Sub-second timestamp_tz values before the epoch are selected
  {
    const auto stmt = conn.execute_fetch("SELECT '1969-12-31 23:59:59.999999999 +00:00'::TIMESTAMP_TZ");
    // Then Result should contain the expected sub-second values before the epoch
    require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1969, 12, 31, 23, 59, 59, 999999999);
  }
  {
    const auto stmt = conn.execute_fetch("SELECT '1969-12-31 23:59:58.5 +00:00'::TIMESTAMP_TZ(3)");
    require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1969, 12, 31, 23, 59, 58, 500000000);
  }
}

TEST_CASE("should select edge date timestamp_tz values", "[timestamp_tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  struct Case {
    const char* name;
    const char* query;
    SQLSMALLINT year;
    SQLUSMALLINT month;
    SQLUSMALLINT day;
    SQLUSMALLINT hour;
    SQLUSMALLINT minute;
    SQLUSMALLINT second;
  };
  const Case cases[] = {
      {"year 9999", "SELECT '9999-12-31 23:59:59 +00:00'::TIMESTAMP_TZ", 9999, 12, 31, 23, 59, 59},
      {"year 1900", "SELECT '1900-01-01 00:00:00 +00:00'::TIMESTAMP_TZ", 1900, 1, 1, 0, 0, 0},
      {"pre-epoch", "SELECT '1960-06-15 12:00:00 +05:00'::TIMESTAMP_TZ", 1960, 6, 15, 7, 0, 0},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);

    // When Query "SELECT <query_values>" is executed
    const auto stmt = conn.execute_fetch(test_case.query);

    // Then Result should contain timestamps <expected_values>
    require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), test_case.year, test_case.month, test_case.day, test_case.hour,
               test_case.minute, test_case.second);

    // And Values should have timezone info
    INFO("SQL_TIMESTAMP_STRUCT cannot carry timezone");
  }
}

TEST_CASE("should handle NULL values for timestamp_tz", "[timestamp_tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ" is executed
  const auto stmt = conn.execute_fetch("SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ");

  // Then Result should contain [2024-01-15 10:30:00 +05:00, NULL]
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0);
  REQUIRE(get_data_optional<SQL_C_TYPE_TIMESTAMP>(stmt, 2) == std::nullopt);
}

TEST_CASE("should download large result set with multiple chunks for timestamp_tz", "[timestamp_tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00
  // +00:00'::TIMESTAMP_TZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                sqlchar("SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
                                        "'2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) as ts "
                                        "FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts"),
                                SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00
  require_sequential_from_2024(stmt, 50000);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select values from table for timestamp_tz", "[timestamp_tz]") {
  // Given Snowflake client is logged in

  struct Case {
    const char* name;
    const char* insert_sql;
  };
  const Case cases[] = {
      {"basic", "INSERT INTO ts_tz_table VALUES ('2024-01-15 10:30:00 +05:00'), ('2024-06-20 14:45:30 -08:00')"},
      {"epoch", "INSERT INTO ts_tz_table VALUES ('1970-01-01 00:00:00 +00:00'), ('2024-01-15 10:30:00 +05:00')"},
      {"microseconds",
       "INSERT INTO ts_tz_table VALUES ('2024-01-15 10:30:00 +05:00'), ('2024-01-15 10:30:00.123456 +05:00')"},
      {"null", "INSERT INTO ts_tz_table VALUES (NULL), ('2024-01-15 10:30:00 +05:00')"},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE ts_tz_table (col TIMESTAMP_TZ)");

    // And Table with TIMESTAMP_TZ column exists with values <insert_values>
    conn.execute(test_case.insert_sql);

    // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    const auto stmt = conn.createStatement();
    SQLRETURN ret =
        SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM ts_tz_table ORDER BY col NULLS LAST"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    // Then Result should contain timestamps <expected_values>
    if (std::string(test_case.name) == "basic") {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 6, 20, 22, 45, 30);
    } else if (std::string(test_case.name) == "epoch") {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1970, 1, 1, 0, 0, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0);
    } else if (std::string(test_case.name) == "microseconds") {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0, 123456000);
    } else {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      REQUIRE(get_data_optional<SQL_C_TYPE_TIMESTAMP>(stmt, 1) == std::nullopt);
    }

    // And Values should have timezone info
    INFO("SQL_TIMESTAMP_STRUCT cannot carry timezone");
    ret = SQLFetch(stmt.getHandle());
    REQUIRE(ret == SQL_NO_DATA);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should download large result set with multiple chunks from table for timestamp_tz",
                 "[timestamp_tz]") {
  // Given Snowflake client is logged in

  // And Table with TIMESTAMP_TZ column exists with 50000 sequential timestamp values
  conn.execute("CREATE TEMPORARY TABLE ts_tz_large (col TIMESTAMP_TZ)");
  conn.execute(
      "INSERT INTO ts_tz_large "
      "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
      "'2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) "
      "FROM TABLE(GENERATOR(ROWCOUNT => 50000))");

  // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM ts_tz_large ORDER BY col NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00
  require_sequential_from_2024(stmt, 50000);
}

TEST_CASE("should select timestamp_tz using parameter binding", "[timestamp_tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ" is executed with bound timestamp values
  const auto stmt = conn.createStatement();
  char val1[] = "2024-01-15 10:30:00 +05:00";
  char val2[] = "2024-06-20 14:45:30 -08:00";
  SQLLEN ind1 = SQL_NTS;
  SQLLEN ind2 = SQL_NTS;
  bind_char(stmt, 1, val1, ind1);
  bind_char(stmt, 2, val2, ind2);

  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain the bound timestamps
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 2), 2024, 6, 20, 22, 45, 30);

  // And Values should have timezone info
  INFO("SQL_TIMESTAMP_STRUCT cannot carry timezone");
}

TEST_CASE("should select null timestamp_tz using parameter binding", "[timestamp_tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::TIMESTAMP_TZ" is executed with bound NULL value
  const auto stmt = conn.createStatement();
  SQLLEN ind = SQL_NULL_DATA;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29,
                                   9, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::TIMESTAMP_TZ"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [NULL]
  REQUIRE(get_data_optional<SQL_C_TYPE_TIMESTAMP>(stmt, 1) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should insert timestamp_tz using parameter binding", "[timestamp_tz]") {
  // Given Snowflake client is logged in
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // And Table with TIMESTAMP_TZ column exists
  conn.execute("CREATE TEMPORARY TABLE ts_tz_bind (col TIMESTAMP_TZ)");

  // When Timestamp values are bulk-inserted using multirow binding
  constexpr SQLULEN num_rows = 3;
  SQL_TIMESTAMP_STRUCT values[num_rows] = {make_ts(2024, 6, 20, 22, 45, 30), make_ts(1970, 1, 1, 0, 0, 0),
                                           make_ts(2024, 1, 15, 5, 30, 0)};
  SQLLEN indicators[num_rows] = {sizeof(SQL_TIMESTAMP_STRUCT), sizeof(SQL_TIMESTAMP_STRUCT),
                                 sizeof(SQL_TIMESTAMP_STRUCT)};
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
  ret = SQLBindParameter(insert_stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29, 9,
                         values, sizeof(SQL_TIMESTAMP_STRUCT), indicators);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLExecDirect(insert_stmt.getHandle(), sqlchar("INSERT INTO ts_tz_bind VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, insert_stmt);
  REQUIRE(params_processed == num_rows);

  // And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
  const auto stmt = conn.createStatement();
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM ts_tz_bind ORDER BY col NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then SELECT should return the same values in any order
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1970, 1, 1, 0, 0, 0);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 5, 30, 0);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 6, 20, 22, 45, 30);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}
