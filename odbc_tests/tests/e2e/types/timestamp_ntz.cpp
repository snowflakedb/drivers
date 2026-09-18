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
#include "timestamp_e2e.hpp"

TEST_CASE("should cast timestamp_ntz values to appropriate type", "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ" is executed
  const auto stmt = conn.execute_fetch("SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ");

  // Then All values should be returned as appropriate type
  require_sql_timestamp(stmt, 1);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);

  // And Values should not have timezone info
  INFO("SQL_TIMESTAMP_STRUCT cannot carry timezone");
}

TEST_CASE("should select timestamp_ntz values", "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  struct Case {
    const char* name;
    const char* query;
  };
  const Case cases[] = {
      {"basic", "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, '2024-06-20 14:45:30'::TIMESTAMP_NTZ"},
      {"epoch", "SELECT '1970-01-01 00:00:00'::TIMESTAMP_NTZ"},
      {"microseconds", "SELECT '2024-01-15 10:30:00.123456'::TIMESTAMP_NTZ"},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);

    // When Query "SELECT <query_values>" is executed
    const auto stmt = conn.execute_fetch(test_case.query);

    // Then Result should contain timestamps <expected_values>
    if (std::string(test_case.name) == "basic") {
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 2), 2024, 6, 20, 14, 45, 30);
    } else if (std::string(test_case.name) == "epoch") {
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1970, 1, 1, 0, 0, 0);
    } else {
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0, 123456000);
    }

    // And Values should not have timezone info
    INFO("SQL_TIMESTAMP_STRUCT cannot carry timezone");
  }
}

TEST_CASE("should select sub-second timestamp_ntz values before epoch", "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Sub-second timestamp_ntz values before the epoch are selected
  {
    const auto stmt = conn.execute_fetch("SELECT '1969-12-31 23:59:59.999999999'::TIMESTAMP_NTZ");
    // Then Result should contain the expected sub-second values before the epoch
    require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1969, 12, 31, 23, 59, 59, 999999999);
  }
  {
    const auto stmt = conn.execute_fetch("SELECT '1969-12-31 23:59:58.5'::TIMESTAMP_NTZ(3)");
    require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1969, 12, 31, 23, 59, 58, 500000000);
  }
}

TEST_CASE("should handle NULL values for timestamp_ntz", "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ" is executed
  const auto stmt = conn.execute_fetch("SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ");

  // Then Result should contain [2024-01-15 10:30:00, NULL]
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);
  REQUIRE(get_data_optional<SQL_C_TYPE_TIMESTAMP>(stmt, 2) == std::nullopt);
}

TEST_CASE("should download large result set with multiple chunks for timestamp_ntz", "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00'::TIMESTAMP_NTZ)
  // as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                sqlchar("SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
                                        "'2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts "
                                        "FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts"),
                                SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
  require_sequential_from_2024(stmt, 50000);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select values from table for timestamp_ntz", "[timestamp_ntz]") {
  // Given Snowflake client is logged in

  struct Case {
    const char* name;
    const char* insert_sql;
  };
  const Case cases[] = {
      {"basic", "INSERT INTO ts_ntz_table VALUES ('2024-01-15 10:30:00'), ('2024-06-20 14:45:30')"},
      {"epoch", "INSERT INTO ts_ntz_table VALUES ('1970-01-01 00:00:00'), ('2024-01-15 10:30:00')"},
      {"microseconds", "INSERT INTO ts_ntz_table VALUES ('2024-01-15 10:30:00'), ('2024-01-15 10:30:00.123456')"},
      {"null", "INSERT INTO ts_ntz_table VALUES (NULL), ('2024-01-15 10:30:00')"},
  };

  for (const auto& test_case : cases) {
    INFO(test_case.name);
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE ts_ntz_table (col TIMESTAMP_NTZ)");

    // And Table with TIMESTAMP_NTZ column exists with values <insert_values>
    conn.execute(test_case.insert_sql);

    // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    const auto stmt = conn.createStatement();
    SQLRETURN ret =
        SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM ts_ntz_table ORDER BY col NULLS LAST"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    // Then Result should contain timestamps <expected_values>
    if (std::string(test_case.name) == "basic") {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 6, 20, 14, 45, 30);
    } else if (std::string(test_case.name) == "epoch") {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1970, 1, 1, 0, 0, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);
    } else if (std::string(test_case.name) == "microseconds") {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0, 123456000);
    } else {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
      REQUIRE(get_data_optional<SQL_C_TYPE_TIMESTAMP>(stmt, 1) == std::nullopt);
    }

    // And Values should not have timezone info
    INFO("SQL_TIMESTAMP_STRUCT cannot carry timezone");
    ret = SQLFetch(stmt.getHandle());
    REQUIRE(ret == SQL_NO_DATA);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should download large result set with multiple chunks from table for timestamp_ntz",
                 "[timestamp_ntz]") {
  // Given Snowflake client is logged in

  // And Table with TIMESTAMP_NTZ column exists with 50000 sequential timestamp values
  conn.execute("CREATE TEMPORARY TABLE ts_ntz_large (col TIMESTAMP_NTZ)");
  conn.execute(
      "INSERT INTO ts_ntz_large "
      "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00'::TIMESTAMP_NTZ) "
      "FROM TABLE(GENERATOR(ROWCOUNT => 50000))");

  // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM ts_ntz_large ORDER BY col NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
  require_sequential_from_2024(stmt, 50000);
}

TEST_CASE("should select timestamp_ntz using parameter binding", "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When Query "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ" is executed with bound timestamp values
  const auto stmt = conn.createStatement();
  SQL_TIMESTAMP_STRUCT val1 = make_ts(2024, 1, 15, 10, 30, 0);
  SQL_TIMESTAMP_STRUCT val2 = make_ts(2024, 6, 20, 14, 45, 30);
  SQLLEN ind1 = sizeof(val1);
  SQLLEN ind2 = sizeof(val2);
  bind_ts(stmt, 1, val1, ind1);
  bind_ts(stmt, 2, val2, ind2);

  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [2024-01-15 10:30:00, 2024-06-20 14:45:30]
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 2), 2024, 6, 20, 14, 45, 30);

  // And Values should not have timezone info
  INFO("SQL_TIMESTAMP_STRUCT cannot carry timezone");
}

TEST_CASE("should return NULL when selecting timestamp_ntz using parameter binding with NULL value",
          "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound NULL value
  const auto stmt = conn.createStatement();
  SQLLEN ind = SQL_NULL_DATA;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29,
                                   9, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::TIMESTAMP_NTZ"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [NULL]
  REQUIRE(get_data_optional<SQL_C_TYPE_TIMESTAMP>(stmt, 1) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should insert timestamp_ntz using parameter binding", "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // And Table with TIMESTAMP_NTZ column exists
  conn.execute("CREATE TEMPORARY TABLE ts_ntz_bind (col TIMESTAMP_NTZ)");

  // When Timestamp values are bulk-inserted using multirow binding
  constexpr SQLULEN num_rows = 3;
  SQL_TIMESTAMP_STRUCT values[num_rows] = {make_ts(2024, 6, 20, 14, 45, 30), make_ts(2024, 1, 15, 10, 30, 0),
                                           make_ts(1970, 1, 1, 0, 0, 0)};
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
  ret = SQLExecDirect(insert_stmt.getHandle(), sqlchar("INSERT INTO ts_ntz_bind VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, insert_stmt);
  REQUIRE(params_processed == num_rows);

  // And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
  const auto stmt = conn.createStatement();
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM ts_ntz_bind ORDER BY col NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then SELECT should return the inserted values in ascending order
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 1970, 1, 1, 0, 0, 0);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 6, 20, 14, 45, 30);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("should return naive datetime for type_name alias when session mapping is TIMESTAMP_NTZ", "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  const char* aliases[] = {"TIMESTAMP", "DATETIME"};
  for (const char* type_name : aliases) {
    INFO(type_name);

    // And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_NTZ
    conn.execute("ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'");

    // When Query "SELECT '2024-01-15 10:30:00'::<type_name>" is executed
    const auto expr = std::string("'2024-01-15 10:30:00'::") + type_name;
    const auto stmt = conn.execute_fetch(std::string("SELECT ") + expr + ", SYSTEM$TYPEOF(" + expr + ")");

    // Then All values should be returned as appropriate type
    require_sql_timestamp(stmt, 1);
    require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);

    // And Values should not have timezone info
    require_no_timezone_info(get_data<SQL_C_CHAR>(stmt, 2));
  }
}

TEST_CASE("should return aware datetime for TIMESTAMP alias when session mapping is TIMESTAMP_LTZ", "[timestamp_ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_LTZ
  conn.execute("ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_LTZ'");
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP" is executed
  const auto stmt =
      conn.execute_fetch("SELECT '2024-01-15 10:30:00'::TIMESTAMP, SYSTEM$TYPEOF('2024-01-15 10:30:00'::TIMESTAMP)");

  // Then All values should be returned as appropriate type
  require_sql_timestamp(stmt, 1);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 15, 10, 30, 0);

  // And Values should have timezone info
  require_timezone_info(get_data<SQL_C_CHAR>(stmt, 2));
}
