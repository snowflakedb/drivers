#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLParamData - Data-at-Execution Flow
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLParamData: Drives data-at-execution flow for single parameter",
                 "[odbc-api][paramdata][submitting_request]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER valuePtr = nullptr;
  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);
  REQUIRE(valuePtr == reinterpret_cast<SQLPOINTER>(1));

  ret = SQLPutData(stmt_handle(), const_cast<char*>("hello"), 5);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(std::string(reinterpret_cast<char*>(buf)) == "hello");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLParamData: Drives data-at-execution flow for multiple parameters",
                 "[odbc-api][paramdata][submitting_request]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS v1, ? AS v2"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae1 = SQL_DATA_AT_EXEC, dae2 = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(100), 0, &dae1);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  ret = SQLBindParameter(stmt_handle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(200), 0, &dae2);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  // Note: Reference driver returns parameters in binding order (ODBC spec does not guarantee this).
  SQLPOINTER valuePtr = nullptr;
  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);
  REQUIRE(valuePtr == reinterpret_cast<SQLPOINTER>(100));
  ret = SQLPutData(stmt_handle(), const_cast<char*>("first"), 5);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);
  REQUIRE(valuePtr == reinterpret_cast<SQLPOINTER>(200));
  ret = SQLPutData(stmt_handle(), const_cast<char*>("second"), 6);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLCHAR b1[64] = {}, b2[64] = {};
  SQLLEN i1 = 0, i2 = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, b1, sizeof(b1), &i1);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  ret = SQLBindCol(stmt_handle(), 2, SQL_C_CHAR, b2, sizeof(b2), &i2);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(std::string(reinterpret_cast<char*>(b1)) == "first");
  REQUIRE(std::string(reinterpret_cast<char*>(b2)) == "second");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLParamData: Drives data-at-execution flow initiated by SQLExecDirect",
                 "[odbc-api][paramdata][submitting_request]") {
  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  SQLRETURN ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                                   reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER valuePtr = nullptr;
  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);
  REQUIRE(valuePtr == reinterpret_cast<SQLPOINTER>(1));

  ret = SQLPutData(stmt_handle(), const_cast<char*>("direct"), 6);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(std::string(reinterpret_cast<char*>(buf)) == "direct");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLParamData: Succeeds with NULL ValuePtrPtr",
                 "[odbc-api][paramdata][submitting_request]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLParamData(stmt_handle(), nullptr);
  REQUIRE(ret == SQL_NEED_DATA);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtSessionSchemaFixture,
                 "SQLParamData: zero-row DAE DML leaves DmlExecuted state and SQLRowCount returns 0",
                 "[odbc-api][paramdata][submitting_request]") {
  // iODBC's DM rejects the second SQLParamData (the one that drives the
  //   DAE handshake to completion) with SQL_INVALID_HANDLE instead of
  //   threading SQL_NEED_DATA / SQL_NO_DATA through, so the full DAE
  //   protocol cannot be exercised here.
  SKIP_IODBC("iODBC DM does not thread the DAE SQL_NEED_DATA -> SQL_NO_DATA sequence through SQLParamData");
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TEMPORARY TABLE dae_zero_dml_t(c1 INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLPrepare(stmt_handle(), sqlchar("UPDATE dae_zero_dml_t SET c1 = ? WHERE 1 = 0"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER valuePtr = nullptr;
  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);
  REQUIRE(valuePtr == reinterpret_cast<SQLPOINTER>(1));

  SQLINTEGER val = 42;
  ret = SQLPutData(stmt_handle(), &val, static_cast<SQLLEN>(sizeof(val)));
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NO_DATA);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 0);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLParamData: SQLCancel aborts data-at-execution and restores prepared state",
                 "[odbc-api][paramdata][submitting_request]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER valuePtr = nullptr;
  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 42;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &val, 0, &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
}

// ============================================================================
// SQLParamData - Error Cases
// ============================================================================

TEST_CASE("SQLParamData: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][paramdata][submitting_request][error]") {
  SQLPOINTER vp = nullptr;
  const SQLRETURN ret = SQLParamData(SQL_NULL_HSTMT, &vp);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLParamData: HY010 without prior SQL_NEED_DATA",
                 "[odbc-api][paramdata][submitting_request][error]") {
  SQLPOINTER vp = nullptr;
  const SQLRETURN ret = SQLParamData(stmt_handle(), &vp);
  NON_IODBC {
    // And the DM forwards the call; the driver rejects it
    //   with HY010 (function sequence error)
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
  }
  IODBC_ONLY {
    // And the DM rejects the call before it reaches the driver
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsError());
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLParamData: HY010 when called consecutively without SQLPutData",
                 "[odbc-api][paramdata][submitting_request][error]") {
  // iODBC's DM rejects the very first SQLParamData call after SQL_NEED_DATA
  //   with SQL_INVALID_HANDLE, so the SQL_NEED_DATA -> consecutive call
  //   transition cannot be exercised
  SKIP_IODBC("iODBC DM rejects SQLParamData after SQL_NEED_DATA with SQL_INVALID_HANDLE");
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER vp = nullptr;
  ret = SQLParamData(stmt_handle(), &vp);
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLParamData(stmt_handle(), &vp);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}
