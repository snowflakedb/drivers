#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLNumResultCols: HY010 during SQL_NEED_DATA",
                 "[odbc-api][numresultcols][retrieving_results][error]") {
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

  // When SQLNumResultCols is called while the statement is in the SQL_NEED_DATA state
  SQLSMALLINT col_count = 0;
  ret = SQLNumResultCols(stmt_handle(), &col_count);
  // Both DMs surface HY010: unixODBC gates at the DM layer; iODBC forwards and the driver short-circuits with the same
  // error.
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  // And the statement is cancelled to release any pending state
  SQLCancel(stmt_handle());
}
