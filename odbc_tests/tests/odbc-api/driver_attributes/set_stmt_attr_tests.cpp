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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetStmtAttr: SQL_ROWSET_SIZE = 0 coerces to 1",
                 "[odbc-api][setstmtattr][driver_attributes]") {
  // SQL_ROWSET_SIZE = 0 is invalid. Handling is driver-manager dependent: unixODBC
  // validates the value and rejects it with SQL_ERROR before the call reaches the
  // driver, whereas iODBC (and the Windows DM) forward it so the driver decides.
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, reinterpret_cast<SQLPOINTER>(0), 0);

  if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
    // The DM forwarded the value; assert the driver-level handling (no 01S02 posted).
    SQLULEN rowset_size = 0;
    ret = SQLGetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, &rowset_size, SQL_IS_UINTEGER, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    // The new driver clamps the invalid 0 to 1 (matching SQL_ATTR_ROW_ARRAY_SIZE); the
    // reference driver stores 0 unchanged.
    NEW_DRIVER_ONLY("BD#103") { REQUIRE(rowset_size == 1); }
    OLD_DRIVER_ONLY("BD#103") { REQUIRE(rowset_size == 0); }
  } else {
    // The DM (unixODBC) validated and rejected the invalid value before the driver.
    REQUIRE(ret == SQL_ERROR);
  }
}
