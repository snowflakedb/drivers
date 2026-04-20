#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLPutData - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: Sends data in a single call",
                 "[odbc-api][putdata][submitting_request]") {
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: Sends data in multiple chunks which are concatenated",
                 "[odbc-api][putdata][submitting_request]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 200, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER valuePtr = nullptr;
  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLPutData(stmt_handle(), const_cast<char*>("AAA"), 3);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  ret = SQLPutData(stmt_handle(), const_cast<char*>("BBB"), 3);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(std::string(reinterpret_cast<char*>(buf)) == "AAABBB");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: SQL_NULL_DATA sets parameter to NULL",
                 "[odbc-api][putdata][submitting_request]") {
  SKIP_OLD_DRIVER("SNOW-3240517", "Reference driver does not propagate SQL_NULL_DATA indicator for DAE params");
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

  ret = SQLPutData(stmt_handle(), nullptr, SQL_NULL_DATA);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(ind == SQL_NULL_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: SQL_NTS sends null-terminated string",
                 "[odbc-api][putdata][submitting_request]") {
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

  ret = SQLPutData(stmt_handle(), const_cast<char*>("nts_test"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(std::string(reinterpret_cast<char*>(buf)) == "nts_test");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: SQL_C_BINARY data sent in multiple chunks",
                 "[odbc-api][putdata][submitting_request]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARBINARY, 200, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER valuePtr = nullptr;
  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);

  unsigned char chunk1[] = {0xDE, 0xAD};
  unsigned char chunk2[] = {0xBE, 0xEF};
  ret = SQLPutData(stmt_handle(), chunk1, sizeof(chunk1));
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  ret = SQLPutData(stmt_handle(), chunk2, sizeof(chunk2));
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  unsigned char buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_BINARY, buf, sizeof(buf), &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(ind == 4);
  REQUIRE(buf[0] == 0xDE);
  REQUIRE(buf[1] == 0xAD);
  REQUIRE(buf[2] == 0xBE);
  REQUIRE(buf[3] == 0xEF);
}

// ============================================================================
// SQLPutData - Error Cases
// ============================================================================

TEST_CASE("SQLPutData: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][putdata][submitting_request][error]") {
  const SQLRETURN ret = SQLPutData(SQL_NULL_HSTMT, const_cast<char*>("x"), 1);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: HY010 without prior SQL_NEED_DATA",
                 "[odbc-api][putdata][submitting_request][error]") {
  SQLRETURN ret = SQLPutData(stmt_handle(), const_cast<char*>("x"), 1);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: HY009 for null DataPtr with SQL_NTS",
                 "[odbc-api][putdata][submitting_request][error]") {
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

  ret = SQLPutData(stmt_handle(), nullptr, SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: HY090 for negative StrLen_or_Ind",
                 "[odbc-api][putdata][submitting_request][error]") {
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

  ret = SQLPutData(stmt_handle(), const_cast<char*>("abc"), -99);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: HY019 for non-character data sent in pieces",
                 "[odbc-api][putdata][submitting_request][error]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER vp = nullptr;
  ret = SQLParamData(stmt_handle(), &vp);
  REQUIRE(ret == SQL_NEED_DATA);

  SQLINTEGER val = 42;
  ret = SQLPutData(stmt_handle(), &val, sizeof(val));
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Second PutData for non-char/binary type triggers HY019
  ret = SQLPutData(stmt_handle(), &val, sizeof(val));
  REQUIRE_EXPECTED_ERROR(ret, "HY019", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: HY020 for SQL_NULL_DATA after data chunk",
                 "[odbc-api][putdata][submitting_request][error]") {
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

  ret = SQLPutData(stmt_handle(), const_cast<char*>("abc"), 3);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Sending SQL_NULL_DATA after a data chunk triggers HY020.
  // The unixODBC Driver Manager may intercept this call in state S10 and
  // return HY011 before the driver sees it, so accept either SQLSTATE.
  char dummy = '\0';
  ret = SQLPutData(stmt_handle(), &dummy, SQL_NULL_DATA);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsError());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()),
               OdbcMatchers::HasSqlState("HY020") || OdbcMatchers::HasSqlState("HY011"));

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: HY020 for data after SQL_NULL_DATA",
                 "[odbc-api][putdata][submitting_request][error]") {
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

  ret = SQLPutData(stmt_handle(), nullptr, SQL_NULL_DATA);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Sending data after SQL_NULL_DATA triggers HY020
  ret = SQLPutData(stmt_handle(), const_cast<char*>("abc"), 3);
  REQUIRE_EXPECTED_ERROR(ret, "HY020", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: Error recovery preserves S9 state (retry after HY009)",
                 "[odbc-api][putdata][submitting_request][error]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
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

  // Trigger HY009 with null DataPtr and non-zero, non-NULL_DATA indicator
  ret = SQLPutData(stmt_handle(), nullptr, SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsError());

  // State S9 should be preserved — a valid PutData call should still work
  ret = SQLPutData(stmt_handle(), const_cast<char*>("recovered"), 9);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLParamData(stmt_handle(), &vp);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(std::string(reinterpret_cast<char*>(buf)) == "recovered");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: HY010 in state S8 (before SQLParamData)",
                 "[odbc-api][putdata][submitting_request][error]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  // S8: Execute returned NEED_DATA but SQLParamData has not been called yet.
  // The unixODBC DM may intercept this as HY010 itself, so accept either.
  ret = SQLPutData(stmt_handle(), const_cast<char*>("x"), 1);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsError());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()),
               OdbcMatchers::HasSqlState("HY010") || OdbcMatchers::HasSqlState("HY011"));

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: HY009 with null DataPtr and negative indicator",
                 "[odbc-api][putdata][submitting_request][error]") {
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

  // null DataPtr with an arbitrary negative indicator (not SQL_NULL_DATA or SQL_NTS)
  ret = SQLPutData(stmt_handle(), nullptr, -99);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}
