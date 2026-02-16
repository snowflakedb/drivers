#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("SQLRowCount returns data about number of rows affected.") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  // When SQLExecDirect is called to execute the query that returns 1 row
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  // And SQLRowCount is called to get the number of rows affected
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);
  // Then the number of rows affected should be 1
  REQUIRE(rows_affected == 1);
}

TEST_CASE("SQLRowCount returns correct count for INSERT statement.") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // Create a temporary table
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                (SQLCHAR*)"CREATE TEMPORARY TABLE test_table (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // When SQLExecDirect is called to execute an INSERT statement
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"INSERT INTO test_table VALUES (1, 'test'), (2, 'test2'), (3, 'test3')", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called to get the number of rows affected
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 3
  REQUIRE(rows_affected == 3);
}

TEST_CASE("SQLRowCount returns correct count for SELECT with many rows.") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called to execute a query that returns 10 rows
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 10)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called to get the number of rows affected
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 10
  REQUIRE(rows_affected == 10);
}
