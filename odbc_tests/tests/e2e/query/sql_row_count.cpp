#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("SQLRowCount returns HY010 when called without executing statement.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLRowCount is called without executing any statement first
  SQLLEN rows_affected = 0;
  SQLRETURN ret = SQLRowCount(stmt.getHandle(), &rows_affected);

  // Then SQLRowCount should return SQL_ERROR with SQLSTATE HY010 (Function sequence error)
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY010");
}

TEST_CASE("SQLRowCount returns data about number of rows affected.") {
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

TEST_CASE("SQLRowCount returns 0 for DDL statements.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // When SQLExecDirect is called to execute a DDL statement
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TABLE test_table (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called to get the number of rows affected
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be -1
  REQUIRE(rows_affected == -1);
}

TEST_CASE("SQLRowCount returns -1 for ALTER TABLE DDL statement.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // When an ALTER TABLE DDL statement is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TABLE alter_test (id INT)", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"ALTER TABLE alter_test ADD COLUMN value VARCHAR(50)", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be -1
  REQUIRE(rows_affected == -1);
}

TEST_CASE("SQLRowCount returns -1 for DROP TABLE DDL statement.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // When a DROP TABLE DDL statement is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TABLE drop_test (id INT)", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"DROP TABLE drop_test", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be -1
  REQUIRE(rows_affected == -1);
}

// =============================================================================
// MERGE statement
// =============================================================================

TEST_CASE("SQLRowCount returns correct count for MERGE statement.", "[query]") {
  // Doc: "SQLRowCount returns the number of rows affected by an UPDATE, INSERT,
  //       or DELETE statement."  MERGE combines these operations.
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlrowcount-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // When a MERGE statement affecting rows is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                (SQLCHAR*)"CREATE TEMPORARY TABLE merge_target (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO merge_target VALUES (1, 'old1'), (2, 'old2')", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TEMPORARY TABLE merge_source (id INT, value VARCHAR(50))",
                      SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO merge_source VALUES (2, 'new2'), (3, 'new3')", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"MERGE INTO merge_target t USING merge_source s ON t.id = s.id "
                                "WHEN MATCHED THEN UPDATE SET t.value = s.value "
                                "WHEN NOT MATCHED THEN INSERT (id, value) VALUES (s.id, s.value)",
                      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);

  // Then the number of rows affected should be 2 (1 updated + 1 inserted)
  REQUIRE(rows_affected == 2);
}

TEST_CASE("SQLRowCount returns correct count for UPDATE statement.", "[query]") {
  // Doc: "SQLRowCount returns the number of rows affected by an UPDATE, INSERT,
  //       or DELETE statement."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlrowcount-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // And a table with data exists
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                (SQLCHAR*)"CREATE TEMPORARY TABLE update_test (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"INSERT INTO update_test VALUES (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd')", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // When an UPDATE statement affecting 2 rows is executed
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"UPDATE update_test SET value = 'updated' WHERE id <= 2", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 2
  REQUIRE(rows_affected == 2);
}

TEST_CASE("SQLRowCount returns correct count for DELETE statement.", "[query]") {
  // Doc: "SQLRowCount returns the number of rows affected by an UPDATE, INSERT,
  //       or DELETE statement."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlrowcount-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // And a table with data exists
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                (SQLCHAR*)"CREATE TEMPORARY TABLE delete_test (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret =
      SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO delete_test VALUES (1, 'a'), (2, 'b'), (3, 'c')", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // When a DELETE statement affecting 2 rows is executed
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"DELETE FROM delete_test WHERE id >= 2", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 2
  REQUIRE(rows_affected == 2);
}

TEST_CASE("SQLRowCount returns total count for DELETE all rows.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // When a DELETE without WHERE clause is executed on a table with 4 rows
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TEMPORARY TABLE delete_all (id INT)", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO delete_all VALUES (1), (2), (3), (4)", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"DELETE FROM delete_all", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 4
  REQUIRE(rows_affected == 4);
}

TEST_CASE("SQLRowCount returns total count for UPDATE all rows.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // When an UPDATE without WHERE clause is executed on a table with 3 rows
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                (SQLCHAR*)"CREATE TEMPORARY TABLE update_all (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret =
      SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO update_all VALUES (1, 'a'), (2, 'b'), (3, 'c')", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"UPDATE update_all SET value = 'updated'", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 3
  REQUIRE(rows_affected == 3);
}

TEST_CASE("SQLRowCount returns correct count for INSERT INTO SELECT.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // When INSERT INTO ... SELECT copies 3 rows from a source table
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                (SQLCHAR*)"CREATE TEMPORARY TABLE src_table (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO src_table VALUES (1, 'a'), (2, 'b'), (3, 'c')", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TEMPORARY TABLE dst_table (id INT, value VARCHAR(50))",
                      SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO dst_table SELECT * FROM src_table", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 3
  REQUIRE(rows_affected == 3);
}

// =============================================================================
// Cursor / Statement State
// =============================================================================

TEST_CASE("SQLRowCount returns HY010 after SQLFreeStmt SQL_CLOSE.", "[query]") {
  // Doc: "The cached row count value is valid until the statement handle is set
  //       back to the prepared or allocated state, the statement is reexecuted,
  //       or SQLCloseCursor is called."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlrowcount-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query is executed and then SQLFreeStmt(SQL_CLOSE) resets the statement
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);

  // Then SQLRowCount should return SQL_ERROR with SQLSTATE HY010 (Function sequence error)
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY010");
}

TEST_CASE("SQLRowCount returns HY010 after SQLPrepare without execute.", "[query]") {
  // Doc: Calling SQLRowCount before SQLExecute/SQLExecDirect should return
  //      HY010 (Function sequence error).

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a statement is prepared but not executed
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);

  // Then SQLRowCount should return SQL_ERROR with SQLSTATE HY010 (Function sequence error)
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY010");
}

// =============================================================================
// Prepared statement execution
// =============================================================================

TEST_CASE("SQLRowCount works with SQLPrepare and SQLExecute flow.", "[query]") {
  // Doc: "SQLRowCount returns the number of rows affected by an UPDATE, INSERT,
  //       or DELETE statement" — applies to both SQLExecDirect and SQLExecute.
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlrowcount-function#related-functions

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // When a statement is prepared and executed via SQLPrepare and SQLExecute
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TEMPORARY TABLE prep_test (id INT)", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO prep_test VALUES (1), (2), (3)", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 3
  REQUIRE(rows_affected == 3);
}

// =============================================================================
// Mixed DML on same statement
// =============================================================================

TEST_CASE("SQLRowCount updates correctly across different DML types on same statement.", "[query]") {
  // Doc: "When SQLExecute, SQLExecDirect, SQLBulkOperations, SQLSetPos, or
  //       SQLMoreResults is called, the SQL_DIAG_ROW_COUNT field … is set to
  //       the row count, and the row count is cached."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlrowcount-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // When INSERT, UPDATE, and DELETE are executed sequentially on the same statement
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                (SQLCHAR*)"CREATE TEMPORARY TABLE mixed_dml (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO mixed_dml VALUES (1, 'a'), (2, 'b'), (3, 'c')", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);
  REQUIRE(rows_affected == 3);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"UPDATE mixed_dml SET value = 'updated' WHERE id <= 2", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);
  REQUIRE(rows_affected == 2);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"DELETE FROM mixed_dml WHERE id = 3", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then SQLRowCount should reflect the count from each operation
  REQUIRE(rows_affected == 1);
}

// =============================================================================
// Fetching State
// =============================================================================

TEST_CASE("SQLRowCount returns cached count after SQLFetch has started.", "[query]") {
  // Doc: "SQLRowCount returns the cached row count value. The cached row count
  //       value is valid until the statement handle is set back to the prepared
  //       or allocated state, the statement is reexecuted, or SQLCloseCursor
  //       is called."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlrowcount-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT query returning 5 rows is executed
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And some rows are fetched
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLRowCount should still return the row count
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);
  CHECK(rows_affected == 5);
}

// =============================================================================
// Re-execution
// =============================================================================

TEST_CASE("SQLRowCount updates after re-execution with different INSERT.", "[query]") {
  // Doc: "When SQLExecute, SQLExecDirect, SQLBulkOperations, SQLSetPos, or
  //       SQLMoreResults is called, the SQL_DIAG_ROW_COUNT field of the diagnostic
  //       data structure is set to the row count, and the row count is cached in
  //       an implementation-dependent way."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlrowcount-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // And a table exists
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TEMPORARY TABLE reexec_test (id INT)", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // When an INSERT of 3 rows is executed
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO reexec_test VALUES (1), (2), (3)", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);
  REQUIRE(rows_affected == 3);

  // And a second INSERT of 1 row is executed on the same statement
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO reexec_test VALUES (4)", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then SQLRowCount should return the updated count
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);
  REQUIRE(rows_affected == 1);
}
