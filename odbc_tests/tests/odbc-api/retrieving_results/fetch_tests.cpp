#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFetch: HY010 during SQL_NEED_DATA",
                 "[odbc-api][fetch][retrieving_results][error]") {
  // Given a prepared statement with a SQL_DATA_AT_EXEC parameter whose execution has
  // entered the SQL_NEED_DATA state (waiting for SQLPutData)
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  // When SQLFetch is called while the statement is in the SQL_NEED_DATA state
  ret = SQLFetch(stmt_handle());
  // Then DM surfaces HY010
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  // And the statement is cancelled to release any pending state
  SQLCancel(stmt_handle());
}

// SQL_ROWSET_SIZE (attr 9, drives SQLExtendedFetch) and SQL_ATTR_ROW_ARRAY_SIZE
// (attr 27, drives SQLFetch/SQLFetchScroll) are required by the ODBC spec to stay
// separate. SQLExtendedFetch must fetch SQL_ROWSET_SIZE rows regardless of the ARD
// array size, and must not write the rowset size back into the ARD.
TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLExtendedFetch: SQL_ROWSET_SIZE does not clobber SQL_ATTR_ROW_ARRAY_SIZE across cursor close",
                 "[odbc-api][fetch][retrieving_results]") {
  // Given SQL_ATTR_ROW_ARRAY_SIZE = 10 (SQLFetch) and SQL_ROWSET_SIZE = 2 (SQLExtendedFetch)
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ROW_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(10), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 15)) ORDER BY id"),
                      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLExtendedFetch runs (rowset = SQL_ROWSET_SIZE = 2)
  SQLULEN row_count = 0;
  SQLUSMALLINT row_status[2] = {SQL_ROW_NOROW, SQL_ROW_NOROW};
  ret = SQLExtendedFetch(stmt_handle(), SQL_FETCH_NEXT, 0, &row_count, row_status);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(row_count == 2);

  // And the cursor is closed (clears used_extended_fetch but must not touch the ARD array size)
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_ATTR_ROW_ARRAY_SIZE still reads 10 — it was not clobbered by the rowset size
  SQLULEN row_array_size = 0;
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_ROW_ARRAY_SIZE, &row_array_size, SQL_IS_UINTEGER, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(row_array_size == 10);

  // And SQL_ROWSET_SIZE still reads 2
  SQLULEN rowset_size = 0;
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, &rowset_size, SQL_IS_UINTEGER, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowset_size == 2);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLExtendedFetch: rowset is driven by SQL_ROWSET_SIZE independently of SQL_ATTR_ROW_ARRAY_SIZE",
                 "[odbc-api][fetch][retrieving_results]") {
  constexpr int kBufSize = 8;
  SQLBIGINT result[kBufSize] = {0};
  SQLLEN indicator[kBufSize] = {0};
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SBIGINT, result, 0, indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Case 1: ROWSET_SIZE = 5 while ROW_ARRAY_SIZE = 1 -> SQLExtendedFetch returns 5 rows.
  {
    ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ROW_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(1), 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, reinterpret_cast<SQLPOINTER>(5), 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 15)) ORDER BY id"),
                        SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);

    SQLULEN row_count = 0;
    ret = SQLExtendedFetch(stmt_handle(), SQL_FETCH_NEXT, 0, &row_count, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(row_count == 5);
    for (int i = 0; i < 5; i++) {
      REQUIRE(result[i] == i);
    }

    ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
    REQUIRE(ret == SQL_SUCCESS);
  }

  // Case 2 (reverse): ROWSET_SIZE = 1 while ROW_ARRAY_SIZE = 5 -> SQLExtendedFetch returns 1 row.
  {
    ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ROW_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(5), 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, reinterpret_cast<SQLPOINTER>(1), 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 15)) ORDER BY id"),
                        SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);

    SQLULEN row_count = 0;
    ret = SQLExtendedFetch(stmt_handle(), SQL_FETCH_NEXT, 0, &row_count, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(row_count == 1);
    REQUIRE(result[0] == 0);

    ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
    REQUIRE(ret == SQL_SUCCESS);
  }
}
