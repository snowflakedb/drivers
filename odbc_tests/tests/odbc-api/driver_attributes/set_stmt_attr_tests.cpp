#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetStmtAttr: SQL_ROWSET_SIZE = 0 returns SQL_ERROR",
                 "[odbc-api][setstmtattr][driver_attributes][error]") {
  // SQL_ROWSET_SIZE = 0 is invalid. unixODBC rejects it in the DM before the
  // driver; iODBC and the Windows DM forward it. 4.x returns
  // SQL_ERROR (HY024) either way. 3.x stores 0 when the DM forwards.
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, reinterpret_cast<SQLPOINTER>(0), 0);

  NEW_DRIVER_ONLY("BD#102") { REQUIRE_EXPECTED_ERROR(ret, "HY024", stmt_handle(), SQL_HANDLE_STMT); }
  OLD_DRIVER_ONLY("BD#102") {
    if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      SQLULEN rowset_size = 0;
      ret = SQLGetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, &rowset_size, SQL_IS_UINTEGER, nullptr);
      REQUIRE(ret == SQL_SUCCESS);
      REQUIRE(rowset_size == 0);
    } else {
      REQUIRE(ret == SQL_ERROR);
    }
  }
}
