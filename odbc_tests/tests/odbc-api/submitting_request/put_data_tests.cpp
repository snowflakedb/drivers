#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLPutData - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: Sends data in a single call",
                 "[odbc-api][putdata][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER valuePtr = nullptr;
  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLPutData(stmt_handle(), const_cast<char*>("hello"), 5);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(reinterpret_cast<char*>(buf)) == "hello");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: Sends data in multiple chunks which are concatenated",
                 "[odbc-api][putdata][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 200, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER valuePtr = nullptr;
  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLPutData(stmt_handle(), const_cast<char*>("AAA"), 3);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLPutData(stmt_handle(), const_cast<char*>("BBB"), 3);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLParamData(stmt_handle(), &valuePtr);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(reinterpret_cast<char*>(buf)) == "AAABBB");
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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPutData(stmt_handle(), const_cast<char*>("x"), 1);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPutData: HY009 for null DataPtr with SQL_NTS",
                 "[odbc-api][putdata][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER vp = nullptr;
  ret = SQLParamData(stmt_handle(), &vp);
  REQUIRE(ret == SQL_NEED_DATA);

  SQLINTEGER val = 42;
  ret = SQLPutData(stmt_handle(), &val, sizeof(val));
  REQUIRE(ret == SQL_SUCCESS);

  // Second PutData for non-char/binary type triggers HY019
  ret = SQLPutData(stmt_handle(), &val, sizeof(val));
  REQUIRE_EXPECTED_ERROR(ret, "HY019", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}
