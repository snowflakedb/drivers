#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetStmtAttr: HY010 during SQL_NEED_DATA",
                 "[odbc-api][setstmtattr][driver_attributes][error]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_MAX_LENGTH, reinterpret_cast<SQLPOINTER>(1024), SQL_IS_UINTEGER);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetStmtAttr: SQL_ATTR_QUERY_TIMEOUT set and get",
                 "[odbc-api][setstmtattr][driver_attributes][query_timeout]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_QUERY_TIMEOUT,
                                 reinterpret_cast<SQLPOINTER>(30), SQL_IS_UINTEGER);
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_QUERY_TIMEOUT, &value, SQL_IS_UINTEGER, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 30);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetStmtAttr: SQL_ATTR_QUERY_TIMEOUT zero disables",
                 "[odbc-api][setstmtattr][driver_attributes][query_timeout]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_QUERY_TIMEOUT,
                                 reinterpret_cast<SQLPOINTER>(60), SQL_IS_UINTEGER);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_QUERY_TIMEOUT,
                       reinterpret_cast<SQLPOINTER>(0), SQL_IS_UINTEGER);
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN value = 99;
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_QUERY_TIMEOUT, &value, SQL_IS_UINTEGER, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 0);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetStmtAttr: SQL_ATTR_QUERY_TIMEOUT default is zero",
                 "[odbc-api][getstmtattr][driver_attributes][query_timeout]") {
  SQLULEN value = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_QUERY_TIMEOUT, &value, SQL_IS_UINTEGER, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 0);
}
