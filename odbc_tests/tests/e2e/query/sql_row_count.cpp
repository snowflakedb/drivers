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

TEST_CASE("SQLRowCount returns HY009 when called with null pointer.", "[query]") {
  // Given Snowflake client is logged in and a statement has been executed
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // When SQLRowCount is called with a null pointer for RowCountPtr
  ret = SQLRowCount(stmt.getHandle(), nullptr);

  // Then SQLRowCount should return SQL_ERROR with SQLSTATE HY009 (Invalid use of null pointer)
  OLD_DRIVER_ONLY("BD#27") { REQUIRE(ret == SQL_SUCCESS); }
  NEW_DRIVER_ONLY("BD#27") {
    REQUIRE(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "HY009");
  }
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

TEST_CASE("SQLRowCount returns correct count for MERGE with DELETE clause.", "[query]") {
  // Doc: MERGE can combine INSERT, UPDATE, and DELETE operations.
  //      SQLRowCount should return the total rows affected across all clauses.

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // And a target table with data exists
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"CREATE TEMPORARY TABLE merge_del_target (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret =
      SQLExecDirect(stmt.getHandle(),
                    (SQLCHAR*)"INSERT INTO merge_del_target VALUES (1, 'keep'), (2, 'remove'), (3, 'update')", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And a source table with actions exists
  ret = SQLExecDirect(
      stmt.getHandle(),
      (SQLCHAR*)"CREATE TEMPORARY TABLE merge_del_source (id INT, value VARCHAR(50), action VARCHAR(10))", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(
      stmt.getHandle(),
      (SQLCHAR*)"INSERT INTO merge_del_source VALUES (2, 'x', 'delete'), (3, 'new3', 'update'), (4, 'new4', 'insert')",
      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // When a MERGE with INSERT, UPDATE, and DELETE clauses is executed
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"MERGE INTO merge_del_target t USING merge_del_source s ON t.id = s.id "
                                "WHEN MATCHED AND s.action = 'delete' THEN DELETE "
                                "WHEN MATCHED AND s.action = 'update' THEN UPDATE SET t.value = s.value "
                                "WHEN NOT MATCHED THEN INSERT (id, value) VALUES (s.id, s.value)",
                      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 3 (1 deleted + 1 updated + 1 inserted)
  REQUIRE(rows_affected == 3);
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

TEST_CASE("SQLRowCount returns correct count for UPDATE with JOIN.", "[query]") {
  // Doc: Validates calculate_rows_affected() for multi-joined updates.

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // And a target table and a source table exist
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                (SQLCHAR*)"CREATE TEMPORARY TABLE upd_target (id INT, status VARCHAR(20))", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret =
      SQLExecDirect(stmt.getHandle(),
                    (SQLCHAR*)"INSERT INTO upd_target VALUES (1, 'pending'), (2, 'pending'), (3, 'shipped')", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TEMPORARY TABLE upd_source (id INT, new_status VARCHAR(20))",
                      SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO upd_source VALUES (1, 'shipped'), (2, 'shipped')",
                      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // When an UPDATE with JOIN affecting 2 rows is executed
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"UPDATE upd_target t SET t.status = s.new_status "
                                "FROM upd_source s WHERE t.id = s.id",
                      SQL_NTS);
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

TEST_CASE("SQLRowCount returns correct count for INSERT from multi-table JOIN.", "[query]") {
  // Doc: Validates calculate_rows_affected() for multi-table inserts
  //      where the source involves a JOIN across multiple tables.

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto random_schema = Schema::use_random_schema(conn);

  // And two source tables and a destination table exist
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                (SQLCHAR*)"CREATE TEMPORARY TABLE ins_src_a (id INT, value VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO ins_src_a VALUES (1, 'a'), (2, 'b')", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TEMPORARY TABLE ins_src_b (id INT, label VARCHAR(50))",
                      SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"INSERT INTO ins_src_b VALUES (1, 'x'), (2, 'y'), (3, 'z')", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret =
      SQLExecDirect(stmt.getHandle(),
                    (SQLCHAR*)"CREATE TEMPORARY TABLE ins_dst (id INT, value VARCHAR(50), label VARCHAR(50))", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // When INSERT from a multi-table JOIN subquery is executed
  ret = SQLExecDirect(
      stmt.getHandle(),
      (SQLCHAR*)"INSERT INTO ins_dst SELECT a.id, a.value, b.label FROM ins_src_a a JOIN ins_src_b b ON a.id = b.id",
      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLRowCount is called
  SQLLEN rows_affected = 0;
  ret = SQLRowCount(stmt.getHandle(), &rows_affected);
  CHECK_ODBC(ret, stmt);

  // Then the number of rows affected should be 2 (only 2 rows match the JOIN)
  REQUIRE(rows_affected == 2);
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
