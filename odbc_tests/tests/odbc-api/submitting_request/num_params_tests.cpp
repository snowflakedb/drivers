#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLNumParams Tests
//
// Tested SQLSTATEs (from Microsoft ODBC spec):
//   SQL_INVALID_HANDLE — null statement handle
//   HY010             — function sequence error (before prepare, during NEED_DATA)
//
// Not tested (Driver Manager or infrastructure responsibility):
//   01000 (General warning)     — driver-specific, no reliable trigger
//   08S01 (Communication link)  — requires network fault injection
//   HY000 (General error)       — catch-all, not directly provokable
//   HY001 (Memory allocation)   — requires OOM simulation
//   HY008 (Operation canceled)  — requires async + cancel timing
//   HY013 (Memory management)   — requires low-memory conditions
//   HY117 (Suspended connection)— requires unknown transaction state
//   HYT01 (Connection timeout)  — requires timeout simulation
//   IM001 (Not supported)       — we support it, so N/A
//   IM017/IM018 (Async notify)  — notification mode not implemented
// ============================================================================

// ============================================================================
// SQLNumParams - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: Returns zero for statement with no parameters",
                 "[odbc-api][numparams][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 0);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: Returns one for single parameter marker",
                 "[odbc-api][numparams][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: Returns correct count for multiple parameter markers",
                 "[odbc-api][numparams][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?, ?, ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 3);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: Returns zero after SQLExecDirect with no parameters",
                 "[odbc-api][numparams][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 0);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: Succeeds with NULL ParameterCountPtr",
                 "[odbc-api][numparams][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLNumParams(stmt_handle(), nullptr);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLNumParams - Parser Edge Cases
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: Ignores ? inside single-quoted string literal",
                 "[odbc-api][numparams][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT '?', ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: Ignores ? inside line comment",
                 "[odbc-api][numparams][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? -- is this counted?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: Handles escaped quotes in literal",
                 "[odbc-api][numparams][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 'it''s a ?', ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);
}

// ============================================================================
// SQLNumParams - Re-prepare
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: Reflects new count after re-prepare",
                 "[odbc-api][numparams][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?, ?, ?, ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 4);
}

// ============================================================================
// SQLNumParams - Error Cases
// ============================================================================

TEST_CASE("SQLNumParams: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][numparams][submitting_request][error]") {
  SQLSMALLINT count = -1;
  const SQLRETURN ret = SQLNumParams(SQL_NULL_HSTMT, &count);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: HY010 when called before prepare",
                 "[odbc-api][numparams][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLSMALLINT count = -1;
  SQLRETURN ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumParams: HY010 during SQL_NEED_DATA",
                 "[odbc-api][numparams][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLSMALLINT count = -1;
  ret = SQLNumParams(stmt_handle(), &count);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}
