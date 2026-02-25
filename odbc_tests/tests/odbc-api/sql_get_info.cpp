#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

TEST_CASE("SQLGetInfo SQL_GETDATA_EXTENSIONS", "[odbc][sqlgetinfo]") {
  EnvironmentHandleWrapper env;

  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC(ret, env);

  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  std::string connection_string = get_connection_string();
  ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                         SQL_DRIVER_NOPROMPT);
  CHECK_ODBC(ret, dbc);

  SQLUINTEGER getdata_extensions = 0;
  SQLSMALLINT string_length = 0;

  ret = SQLGetInfo(dbc.getHandle(), SQL_GETDATA_EXTENSIONS, &getdata_extensions, sizeof(getdata_extensions),
                   &string_length);
  CHECK_ODBC(ret, dbc);

  REQUIRE(ret == SQL_SUCCESS);

  // Expected extensions are supported
  CHECK((getdata_extensions & SQL_GD_ANY_COLUMN) != 0);
  CHECK((getdata_extensions & SQL_GD_ANY_ORDER) != 0);
  CHECK((getdata_extensions & SQL_GD_BOUND) != 0);

  // Expected extensions are NOT supported
  CHECK((getdata_extensions & SQL_GD_BLOCK) == 0);
  CHECK((getdata_extensions & SQL_GD_OUTPUT_PARAMS) == 0);

  SQLDisconnect(dbc.getHandle());
}
