#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetData: HY010 during SQL_NEED_DATA",
                 "[odbc-api][getdata][retrieving_results][error]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &ind);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}
