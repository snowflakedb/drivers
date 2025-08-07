#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <fstream>
#include <iostream>
#include <sstream>

#include <catch2/catch_test_macros.hpp>

#include "macros.hpp"
#include "test_setup.hpp"

TEST_CASE("Test SELECT 1", "[odbc]") {
  EnvironmentHandleWrapper env;
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  StatementHandleWrapper stmt = dbc.createStatementHandle();

  SQLRETURN ret =
      SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC_ERROR(ret, env, SQL_HANDLE_ENV)

  // Get driver path from environment variable
  std::string connection_string = get_connection_string();
  ret = SQLDriverConnect(dbc, NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                         SQL_DRIVER_NOPROMPT);
  CHECK_ODBC_ERROR(ret, dbc, SQL_HANDLE_DBC);

  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  ret = SQLExecDirect(stmt, (SQLCHAR*)"SELECT 1", SQL_NTS);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  SQLSMALLINT num_cols;
  ret = SQLNumResultCols(stmt, &num_cols);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  ret = SQLFetch(stmt);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  SQLINTEGER result = 0;
  ret = SQLGetData(stmt, 1, SQL_C_LONG, &result, sizeof(result), NULL);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  SQLDisconnect(dbc);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}
