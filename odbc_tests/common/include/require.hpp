#include "HandleWrapper.hpp"

inline void require_connection_failed(const std::string& connection_string) {
  auto env = EnvironmentHandleWrapper();
  SQLRETURN ret =
      SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC(ret, env);

  auto dbc = env.createConnectionHandle();
  ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL,
                         0, NULL, SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);
}
