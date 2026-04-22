#ifndef SESSION_PARAMETER_OVERRIDE_HPP
#define SESSION_PARAMETER_OVERRIDE_HPP

#include <sql.h>

#include <string>

#include "odbc_cast.hpp"

// RAII guard that sets a Snowflake session parameter on construction and
// unsets it on destruction, guaranteeing cleanup even when the test exits
// early (e.g. via SKIP or an unexpected failure).
//
// The caller must supply a statement handle that remains valid for the
// lifetime of this object. The handle's cursor is closed after each
// ALTER SESSION so it stays usable for subsequent statements.
class SessionParameterOverride {
 public:
  SessionParameterOverride(SQLHSTMT stmt, const std::string& param_name, const std::string& param_value)
      : stmt_(stmt), param_name_(param_name) {
    const std::string sql = "ALTER SESSION SET " + param_name_ + " = " + param_value;
    SQLRETURN ret = SQLExecDirect(stmt_, sqlchar(sql.c_str()), SQL_NTS);
    if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
      active_ = false;
      return;
    }
    active_ = true;
    SQLFreeStmt(stmt_, SQL_CLOSE);
  }

  ~SessionParameterOverride() {
    if (!active_) {
      return;
    }
    const std::string sql = "ALTER SESSION UNSET " + param_name_;
    SQLExecDirect(stmt_, sqlchar(sql.c_str()), SQL_NTS);
    SQLFreeStmt(stmt_, SQL_CLOSE);
  }

  [[nodiscard]] bool is_active() const { return active_; }

  // Non-copyable, non-movable
  SessionParameterOverride(const SessionParameterOverride&) = delete;
  SessionParameterOverride& operator=(const SessionParameterOverride&) = delete;
  SessionParameterOverride(SessionParameterOverride&&) = delete;
  SessionParameterOverride& operator=(SessionParameterOverride&&) = delete;

 private:
  SQLHSTMT stmt_;
  std::string param_name_;
  bool active_ = false;
};

#endif  // SESSION_PARAMETER_OVERRIDE_HPP
