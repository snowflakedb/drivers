#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"

// =============================================================================
// SELECT QUERIES
// =============================================================================

TEST_CASE("should execute simple SELECT returning single value", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 1 AS value" is executed
  auto stmt = conn.execute_fetch("SELECT 1 AS value");

  // Then the result should contain value 1
  auto value = get_data<SQL_C_LONG>(stmt, 1);
  CHECK(value == 1);
}

TEST_CASE("should execute SELECT returning multiple columns", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3" is executed
  auto stmt = conn.execute_fetch("SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3");

  // Then the result should contain:
  SQLSMALLINT num_cols;
  SQLRETURN ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  REQUIRE_ODBC(ret, stmt);
  CHECK(num_cols == 3);

  auto col1 = get_data<SQL_C_CHAR>(stmt, 1);
  auto col2 = get_data<SQL_C_CHAR>(stmt, 2);
  auto col3 = get_data<SQL_C_CHAR>(stmt, 3);
  CHECK(col1 == "1");
  CHECK(col2 == "hello");
  CHECK(col3 == "3.14");
}

TEST_CASE("should execute SELECT returning multiple rows", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id" is executed
  auto stmt = conn.execute("SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id");

  // Then there are 5 numbered sequentially rows returned
  int row_count = 0;
  while (true) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);

    auto value = get_data<SQL_C_LONG>(stmt, 1);
    CHECK(value == row_count);
    row_count++;
  }
  CHECK(row_count == 5);
}

TEST_CASE("should execute SELECT returning empty result set", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 1 WHERE 1=0" is executed
  auto stmt = conn.execute("SELECT 1 WHERE 1=0");

  // Then the result set should be empty
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should execute SELECT returning NULL values", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL AS col1, 42 AS col2, NULL AS col3" is executed
  auto stmt = conn.execute_fetch("SELECT NULL AS col1, 42 AS col2, NULL AS col3");

  // Then the result should contain NULL for col1 and col3 and 42 for col2
  auto col1 = get_data_optional<SQL_C_CHAR>(stmt, 1);
  auto col2 = get_data<SQL_C_LONG>(stmt, 2);
  auto col3 = get_data_optional<SQL_C_CHAR>(stmt, 3);
  CHECK(!col1.has_value());
  CHECK(col2 == 42);
  CHECK(!col3.has_value());
}

// =============================================================================
// DDL STATEMENTS
// =============================================================================

TEST_CASE("should execute CREATE and DROP TABLE statements", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When CREATE TABLE statement is executed
  conn.execute("CREATE TABLE basic_exec_test (id INT, name VARCHAR(100))");

  // Then the table should be created successfully
  auto verify_stmt = conn.execute_fetch("SELECT COUNT(*) FROM basic_exec_test");
  CHECK(get_data<SQL_C_LONG>(verify_stmt, 1) == 0);

  // And DROP TABLE statement should complete successfully
  conn.execute("DROP TABLE basic_exec_test");
}

// =============================================================================
// DML STATEMENTS
// =============================================================================

TEST_CASE("should execute INSERT and retrieve inserted data", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table is created
  conn.execute("CREATE TEMPORARY TABLE insert_test (id INT, value VARCHAR(100))");

  // When INSERT statement is executed to add rows
  conn.execute("INSERT INTO insert_test VALUES (1, 'first'), (2, 'second'), (3, 'third')");

  // And Query "SELECT id, value FROM {table} ORDER BY id" is executed
  auto stmt = conn.execute("SELECT id, value FROM insert_test ORDER BY id");

  // Then the inserted data should be correctly returned
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 1);
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "first");

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 2);
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "second");

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 3);
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "third");

  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

// =============================================================================
// ERROR HANDLING
// =============================================================================

TEST_CASE("should return error for invalid SQL syntax", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Invalid SQL "SELCT INVALID SYNTAX" is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELCT INVALID SYNTAX", SQL_NTS);

  // Then An error should be returned
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "42000");
}

// =============================================================================
// SEQUENTIAL EXECUTION
// =============================================================================

TEST_CASE("should execute multiple queries sequentially on same connection", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Multiple queries are executed sequentially
  auto stmt1 = conn.execute_fetch("SELECT 1 AS value");
  // Then each query should return correct results
  CHECK(get_data<SQL_C_LONG>(stmt1, 1) == 1);

  auto stmt2 = conn.execute_fetch("SELECT 'hello' AS greeting");
  CHECK(get_data<SQL_C_CHAR>(stmt2, 1) == "hello");

  auto stmt3 = conn.execute_fetch("SELECT 100 AS number");
  CHECK(get_data<SQL_C_LONG>(stmt3, 1) == 100);
}
