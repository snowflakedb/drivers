#include <optional>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "macros.hpp"
#include "odbc_cast.hpp"

TEST_CASE("should cast boolean values to appropriate type", "[boolean]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN");

  // Then All values should be returned as appropriate type
  auto col1 = get_data<SQL_C_BIT>(stmt, 1);
  auto col2 = get_data<SQL_C_BIT>(stmt, 2);
  auto col3 = get_data<SQL_C_BIT>(stmt, 3);

  // And Values should match [TRUE, FALSE, TRUE]
  REQUIRE(col1 == 1);
  REQUIRE(col2 == 0);
  REQUIRE(col3 == 1);
}

TEST_CASE("should select boolean literals", "[boolean]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then Result should contain [TRUE, FALSE]
  REQUIRE(get_data<SQL_C_BIT>(stmt, 1) == 1);
  REQUIRE(get_data<SQL_C_BIT>(stmt, 2) == 0);
}

TEST_CASE("should handle NULL values from literals", "[boolean]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN");

  // Then Result should contain [FALSE, NULL, TRUE, NULL]
  REQUIRE(get_data_optional<SQL_C_BIT>(stmt, 1) == std::optional<SQLCHAR>(0));
  REQUIRE(get_data_optional<SQL_C_BIT>(stmt, 2) == std::nullopt);
  REQUIRE(get_data_optional<SQL_C_BIT>(stmt, 3) == std::optional<SQLCHAR>(1));
  REQUIRE(get_data_optional<SQL_C_BIT>(stmt, 4) == std::nullopt);
}

TEST_CASE("should download large result set with multiple chunks from GENERATOR", "[boolean]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT (id % 2 = 0)::BOOLEAN FROM <generator>" is executed
  const auto stmt = conn.createStatement();
  const auto sql = "SELECT (seq8() % 2 = 0)::BOOLEAN AS col FROM TABLE(GENERATOR(ROWCOUNT => 1000000))";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 500000 TRUE and 500000 FALSE values
  int true_count = 0;
  int false_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLCHAR value = 0;
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BIT, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);
    REQUIRE((value == 0 || value == 1));

    if (value == 1) {
      true_count++;
    } else {
      false_count++;
    }
  }

  REQUIRE(true_count == 500000);
  REQUIRE(false_count == 500000);
}

TEST_CASE("should select boolean values from table", "[boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (BOOLEAN, BOOLEAN, BOOLEAN) exists
  conn.execute("CREATE TABLE boolean_table (c1 BOOLEAN, c2 BOOLEAN, c3 BOOLEAN)");

  // And Row (TRUE, FALSE, TRUE) is inserted
  conn.execute("INSERT INTO boolean_table VALUES (TRUE, FALSE, TRUE)");

  // When Query "SELECT * FROM <table>" is executed
  const auto stmt = conn.execute_fetch("SELECT * FROM boolean_table");

  // Then Result should contain [TRUE, FALSE, TRUE]
  REQUIRE(get_data<SQL_C_BIT>(stmt, 1) == 1);
  REQUIRE(get_data<SQL_C_BIT>(stmt, 2) == 0);
  REQUIRE(get_data<SQL_C_BIT>(stmt, 3) == 1);
}

TEST_CASE("should handle NULL values from table", "[boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with BOOLEAN column exists
  conn.execute("CREATE TABLE boolean_null_table (col BOOLEAN)");

  // And Rows [NULL, TRUE, FALSE] are inserted
  conn.execute("INSERT INTO boolean_null_table VALUES (NULL), (TRUE), (FALSE)");

  // When Query "SELECT * FROM <table>" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM boolean_null_table"), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain [NULL, TRUE, FALSE] in any order
  int true_count = 0;
  int false_count = 0;
  int null_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    if (auto result = get_data_optional<SQL_C_BIT>(stmt, 1); !result.has_value()) {
      null_count++;
    } else if (result.value() == 1) {
      true_count++;
    } else {
      false_count++;
    }
  }

  REQUIRE(true_count == 1);
  REQUIRE(false_count == 1);
  REQUIRE(null_count == 1);
}

TEST_CASE("should download large result set with multiple chunks from table", "[boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with BOOLEAN column exists with 500000 TRUE and 500000 FALSE values
  conn.execute("CREATE TABLE boolean_large_table (col BOOLEAN)");
  conn.execute(
      "INSERT INTO boolean_large_table "
      "SELECT (seq8() % 2 = 0)::BOOLEAN FROM TABLE(GENERATOR(ROWCOUNT => 1000000))");

  // When Query "SELECT col FROM <table>" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM boolean_large_table"), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 500000 TRUE and 500000 FALSE values
  int true_count = 0;
  int false_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLCHAR value = 0;
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BIT, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);
    REQUIRE((value == 0 || value == 1));

    if (value == 1) {
      true_count++;
    } else {
      false_count++;
    }
  }

  REQUIRE(true_count == 500000);
  REQUIRE(false_count == 500000);
}

TEST_CASE("should select boolean using parameter binding", "[boolean]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN" is executed with bound boolean values [TRUE, FALSE, TRUE]
  {
    const auto stmt = conn.createStatement();
    SQLCHAR val1 = 1;
    SQLCHAR val2 = 0;
    SQLCHAR val3 = 1;
    SQLLEN ind1 = 0, ind2 = 0, ind3 = 0;

    SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 1, 0, &val1, 0, &ind1);
    CHECK_ODBC(ret, stmt);
    ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 1, 0, &val2, 0, &ind2);
    CHECK_ODBC(ret, stmt);
    ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 1, 0, &val3, 0, &ind3);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN"), SQL_NTS);
    CHECK_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    // Then Result should contain [TRUE, FALSE, TRUE]
    REQUIRE(get_data<SQL_C_BIT>(stmt, 1) == 1);
    REQUIRE(get_data<SQL_C_BIT>(stmt, 2) == 0);
    REQUIRE(get_data<SQL_C_BIT>(stmt, 3) == 1);
  }

  // When Query "SELECT ?::BOOLEAN" is executed with bound NULL value
  {
    const auto stmt = conn.createStatement();
    SQLCHAR val = 0;
    SQLLEN ind = SQL_NULL_DATA;

    SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 1, 0, &val, 0, &ind);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::BOOLEAN"), SQL_NTS);
    CHECK_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    // Then Result should contain [NULL]
    REQUIRE(get_data_optional<SQL_C_BIT>(stmt, 1) == std::nullopt);
  }
}

TEST_CASE("should insert boolean using parameter binding", "[boolean]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with BOOLEAN column exists
  conn.execute("CREATE TABLE boolean_bind_table (col BOOLEAN)");

  // When Boolean values [TRUE, FALSE, NULL] are bulk-inserted using multirow binding
  constexpr SQLULEN num_rows = 3;
  SQLCHAR values[num_rows] = {1, 0, 0};
  SQLLEN indicators[num_rows] = {0, 0, SQL_NULL_DATA};
  SQLUSMALLINT param_status[num_rows] = {};
  SQLULEN params_processed = 0;

  const auto insert_stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  CHECK_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  CHECK_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status, 0);
  CHECK_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  CHECK_ODBC(ret, insert_stmt);

  ret = SQLBindParameter(insert_stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 1, 0, values, 0, indicators);
  CHECK_ODBC(ret, insert_stmt);

  ret = SQLExecDirect(insert_stmt.getHandle(), sqlchar("INSERT INTO boolean_bind_table VALUES (?)"), SQL_NTS);
  CHECK_ODBC(ret, insert_stmt);
  REQUIRE(params_processed == num_rows);

  // Then SELECT should return the same values in any order
  const auto stmt = conn.createStatement();
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM boolean_bind_table"), SQL_NTS);
  CHECK_ODBC(ret, stmt);

  int true_count = 0;
  int false_count = 0;
  int null_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    if (auto result = get_data_optional<SQL_C_BIT>(stmt, 1); !result.has_value()) {
      null_count++;
    } else if (result.value() == 1) {
      true_count++;
    } else {
      false_count++;
    }
  }

  REQUIRE(true_count == 1);
  REQUIRE(false_count == 1);
  REQUIRE(null_count == 1);
}
