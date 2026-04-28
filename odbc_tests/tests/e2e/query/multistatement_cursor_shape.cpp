#include <sql.h>
#include <sqlext.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "sf_odbc.h"

// =============================================================================
// Tests that assert cursor shape (SQLNumResultCols + SQLRowCount) for every
// result set produced by a multi-statement execution. ODBC-specific: the
// contract covers SQL_C API return codes, so there is no matching shared
// Gherkin feature; scenarios live as step comments inside each TEST_CASE.
// =============================================================================

namespace {
// Snapshot of what SQLNumResultCols + SQLRowCount report after some ODBC call.
struct CursorShape {
  SQLSMALLINT num_cols;
  SQLLEN row_count;
};

CursorShape captureShape(HSTMT stmt_handle) {
  CursorShape shape{};
  SQLRETURN ret = SQLNumResultCols(stmt_handle, &shape.num_cols);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLRowCount(stmt_handle, &shape.row_count);
  REQUIRE(ret == SQL_SUCCESS);
  return shape;
}
}  // namespace

TEST_CASE("should report correct cursor shape for each result set in a DDL + DML + DDL batch",
          "[query][multistatement]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Multistatement query with CREATE TABLE, INSERT, and DROP is executed
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(),
                      sqlchar("CREATE OR REPLACE TEMPORARY TABLE ms_shape_test(id INT);"
                              " INSERT INTO ms_shape_test VALUES (10),(20),(30);"
                              " DROP TABLE ms_shape_test"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the CREATE TABLE result set reports no cursor and unknown row count
  CursorShape create_shape = captureShape(stmt.getHandle());
  CHECK(create_shape.num_cols == 0);
  CHECK(create_shape.row_count == -1);

  // And fetching on the CREATE TABLE result set does not return a row
  ret = SQLFetch(stmt.getHandle());
  CHECK((ret == SQL_ERROR || ret == SQL_NO_DATA));

  // And the INSERT result set reports no cursor and a row count matching the inserted rows
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CursorShape insert_shape = captureShape(stmt.getHandle());
  CHECK(insert_shape.num_cols == 0);
  CHECK(insert_shape.row_count == 3);

  // And the DROP TABLE result set reports no cursor and unknown row count
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CursorShape drop_shape = captureShape(stmt.getHandle());
  CHECK(drop_shape.num_cols == 0);
  CHECK(drop_shape.row_count == -1);

  // And no further result sets are returned
  ret = SQLMoreResults(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should not open a cursor for any statement in a TCL-only batch",
          "[query][multistatement]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Multistatement query with BEGIN, ALTER SESSION, and COMMIT is executed
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(),
                      sqlchar("BEGIN;"
                              " ALTER SESSION SET TIMEZONE='UTC';"
                              " COMMIT"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  for (int stmt_idx = 0; stmt_idx < 3; ++stmt_idx) {
    INFO("stmt_idx=" << stmt_idx);

    // Then every result set reports no cursor and unknown row count
    CursorShape shape = captureShape(stmt.getHandle());
    CHECK(shape.num_cols == 0);
    CHECK(shape.row_count == -1);

    // And fetching on any result set does not return a row
    SQLRETURN fetch_ret = SQLFetch(stmt.getHandle());
    CHECK((fetch_ret == SQL_ERROR || fetch_ret == SQL_NO_DATA));

    if (stmt_idx < 2) {
      ret = SQLMoreResults(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
    }
  }

  // And no further result sets are returned
  ret = SQLMoreResults(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}
