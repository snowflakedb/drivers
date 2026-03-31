#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <fstream>
#include <iostream>
#include <sstream>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

TEST_CASE("Test SELECT 1", "[odbc]") {
  std::string connection_string = get_connection_string();
  EnvironmentHandleWrapper env;

  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);

  // Get driver path from environment variable
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  ret = SQLDriverConnect(dbc.getHandle(), NULL, sqlchar(connection_string.c_str()), SQL_NTS, NULL, 0, NULL,
                         SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);
  {
    StatementHandleWrapper stmt = dbc.createStatementHandle();
    ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    SQLSMALLINT num_cols;
    ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
    REQUIRE_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);

    SQLINTEGER result = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), NULL);
    REQUIRE_ODBC(ret, stmt);
  }
  ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(ret, dbc);
}
