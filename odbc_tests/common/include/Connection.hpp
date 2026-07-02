#ifndef CONNECTION_HPP
#define CONNECTION_HPP

#include <sql.h>
#include <sqlext.h>

#include <string>
#include <string_view>

#include "HandleWrapper.hpp"
#include "WideString.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

class Connection {
 public:
  ConnectionHandleWrapper& handleWrapper() { return dbc; }

  static EnvironmentHandleWrapper initEnv() {
    EnvironmentHandleWrapper env;
    SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    REQUIRE_ODBC(ret, env);
    return env;
  }

  static ConnectionHandleWrapper initDbc(EnvironmentHandleWrapper& env, const std::string& connection_string) {
    ConnectionHandleWrapper dbc = env.createConnectionHandle();
    SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                     SQL_DRIVER_NOPROMPT);
    REQUIRE_ODBC(ret, dbc);
    return dbc;
  }
  // Constructor that initializes the connection string
  explicit Connection(std::string connection_string)
      : connection_string(std::move(connection_string)),
        env{initEnv()},
        dbc{initDbc(this->env, this->connection_string)} {}

  Connection() : Connection(get_connection_string()) {}
  ~Connection() noexcept {
    // Disconnect best-effort; the handle wrapper frees
    // the handle next regardless of the SQLDisconnect return code.
    SQLDisconnect(dbc.getHandle());
  }

  StatementHandleWrapper createStatement() { return dbc.createStatementHandle(); }

  StatementHandleWrapper execute(const std::string& query) {
    auto stmt = createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)query.c_str(), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    return stmt;
  }

  // Best-effort variant of execute() for destructor / teardown cleanup paths.
  // REQUIRE_ODBC throws Catch::TestFailureException on failure; from a
  // destructor running during stack unwinding that becomes std::terminate.
  // Swallows any exception (including the REQUIRE in HandleWrapper's ctor on
  // SQLAllocHandle failure) and any non-success SQLRETURN.
  void try_execute(const std::string& query) noexcept {
    try {
      auto stmt = createStatement();
      SQLExecDirect(stmt.getHandle(), (SQLCHAR*)query.c_str(), SQL_NTS);
    } catch (...) {
    }
  }

  // Submit a SQL statement through the wide entry point. The query is
  // given as Unicode code points (`U"..."` literal or `std::u32string`)
  // and transcoded to the DM-side `SQLWCHAR` encoding (UTF-16 under
  // unixODBC, UTF-32 under iODBC) inside the call. Going through the
  // wide entry point avoids iODBC's narrow→wide auto-conversion, which
  // transcodes via Latin-1 and would mangle non-ASCII bytes.
  StatementHandleWrapper executew(std::u32string_view query) {
    auto stmt = createStatement();
    auto wide = sf::wide::encode_wide(query);
    SQLRETURN ret = SQLExecDirectW(stmt.getHandle(), wide.data(), static_cast<SQLINTEGER>(wide.size() - 1));
    REQUIRE_ODBC(ret, stmt);
    return stmt;
  }

  StatementHandleWrapper executew_fetch(std::u32string_view query) {
    auto stmt = executew(query);
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    return stmt;
  }

  StatementHandleWrapper execute_fetch(const std::string& query) {
    auto stmt = execute(query);
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    return stmt;
  }

 private:
  std::string connection_string;
  EnvironmentHandleWrapper env;
  ConnectionHandleWrapper dbc;
};

#endif  // CONNECTION_HPP
