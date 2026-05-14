#ifndef OVERFLOW_HELPERS_HPP
#define OVERFLOW_HELPERS_HPP

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include "HandleWrapper.hpp"
#include "get_diag_rec.hpp"

/// Diagnostic data captured when a query errors. Carries the full first
/// `SQLGetDiagRec` record (sqlstate / native error / message text) plus the
/// name of the API call that surfaced the error -- useful for pinning down
/// what the legacy driver actually returns without re-running locally.
struct OverflowResult {
  SQLRETURN ret;
  /// Which ODBC entry point produced the first non-success return code:
  /// "SQLExecDirect", "SQLFetch", "SQLGetData", or empty if every step
  /// returned success.
  std::string which_step;
  std::string sqlstate;
  SQLINTEGER native_error;
  std::string message;
  /// What SQLGetData wrote into the buffer if every step succeeded.
  std::string rendered;
};

namespace overflow_helpers_detail {
inline OverflowResult error_at(const char* step, SQLRETURN ret, StatementHandleWrapper& stmt) {
  auto records = get_diag_rec(stmt);
  if (records.empty()) {
    return {ret, step, "", 0, "", ""};
  }
  return {ret, step, records[0].sqlState, records[0].nativeError, records[0].messageText, ""};
}
}  // namespace overflow_helpers_detail

/// Walk SQLExecDirect -> SQLFetch -> SQLGetData(SQL_C_CHAR) and stop at the
/// first non-success step, returning the full first diagnostic record so
/// the test's INFO line shows exactly what the driver said.
inline OverflowResult run_overflow_query(StatementHandleWrapper& stmt, const char* sql) {
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
    return overflow_helpers_detail::error_at("SQLExecDirect", ret, stmt);
  }

  ret = SQLFetch(stmt.getHandle());
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
    return overflow_helpers_detail::error_at("SQLFetch", ret, stmt);
  }

  char buffer[64] = {};
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
    return overflow_helpers_detail::error_at("SQLGetData", ret, stmt);
  }
  return {ret, "", "", 0, "", (indicator > 0) ? std::string(buffer, static_cast<size_t>(indicator)) : std::string{}};
}

#endif  // OVERFLOW_HELPERS_HPP
