#ifndef QUERY_HELPERS_HPP
#define QUERY_HELPERS_HPP

#include <sql.h>
#include <sqlext.h>

#include <optional>
#include <string>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "sf_odbc.h"

// Query the current database name using a temporary statement on the given connection.
// Allocates and frees its own statement handle so the caller's statement state is not mutated.
// Throws std::runtime_error if any ODBC call fails.
std::string get_current_database(SQLHDBC dbc);

// Returns the query ID of the last statement executed on `stmt` via
// SQL_SF_STMT_ATTR_LAST_QUERY_ID. Returns an empty string if no statement has
// been executed yet. Fails the test via REQUIRE_ODBC on any ODBC error.
inline std::string get_last_query_id(StatementHandleWrapper& stmt) {
  char buf[40] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, buf, sizeof(buf), &len);
  REQUIRE_ODBC(ret, stmt);
  return std::string(buf);
}

// Returns the number of files currently in the @SYSTEM$BIND temporary stage,
// or std::nullopt if the stage does not yet exist in this session (LIST returns
// an error). This distinguishes "stage absent" (expected before the first stage
// bind) from a real ODBC error: a real error triggers REQUIRE_ODBC inside the
// fetch loop, while a missing stage returns nullopt from the initial ExecDirect.
inline std::optional<int> list_system_bind_file_count(Connection& conn) {
  auto stmt = conn.createStatement();
  SQLCHAR sql[] = "LIST @SYSTEM$BIND";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sql, SQL_NTS);
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
    return std::nullopt;
  }
  int count = 0;
  SQLRETURN fetch_ret;
  while ((fetch_ret = SQLFetch(stmt.getHandle())) == SQL_SUCCESS) {
    count++;
  }
  REQUIRE(fetch_ret == SQL_NO_DATA);
  return count;
}

#endif  // QUERY_HELPERS_HPP
