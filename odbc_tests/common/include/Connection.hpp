#include <sql.h>
#include <sqlext.h>

#include <string>

#include "HandleWrapper.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

class Connection {
 public:
  static EnvironmentHandleWrapper initEnv() {
    EnvironmentHandleWrapper env;
    SQLRETURN ret =
        SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_ODBC(ret, env);
    return env;
  }

  static ConnectionHandleWrapper initDbc(EnvironmentHandleWrapper& env,
                                         const std::string& connection_string) {
    ConnectionHandleWrapper dbc = env.createConnectionHandle();
    SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(),
                                     SQL_NTS, NULL, 0, NULL, SQL_DRIVER_NOPROMPT);
    CHECK_ODBC(ret, dbc);
    return dbc;
  }
  // Constructor that initializes the connection string
  explicit Connection(std::string connection_string)
      : connection_string(std::move(connection_string)),
        env{initEnv()},
        dbc{initDbc(this->env, this->connection_string)} {}

  Connection() : Connection(get_connection_string()) {}
  ~Connection() {
    SQLRETURN ret = SQLDisconnect(dbc.getHandle());
    CHECK_ODBC(ret, dbc);
  }

  StatementHandleWrapper createStatement() { return dbc.createStatementHandle(); }

  StatementHandleWrapper execute(const std::string& query) {
    auto stmt = createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)query.c_str(), SQL_NTS);
    CHECK_ODBC(ret, stmt);
    return stmt;
  }

  StatementHandleWrapper execute_fetch(const std::string& query) {
    auto stmt = execute(query);
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    return stmt;
  }

 private:
  std::string connection_string;
  EnvironmentHandleWrapper env;
  ConnectionHandleWrapper dbc;
};
