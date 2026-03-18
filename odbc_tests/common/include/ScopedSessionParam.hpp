#ifndef SCOPEDSESSIONPARAM_HPP
#define SCOPEDSESSIONPARAM_HPP

#include <sql.h>
#include <sqlext.h>

#include <string>

#include "odbc_cast.hpp"

// RAII wrapper that sets a Snowflake session parameter on construction and
// unsets it (restoring the server default) on destruction.  Allocates a
// temporary statement handle internally so it never interferes with the
// test's own statement handle.
class ScopedSessionParam {
 public:
  ScopedSessionParam(SQLHDBC dbc, const std::string& param, const std::string& value)
      : dbc_(dbc), param_(param), active_(execute("ALTER SESSION SET " + param_ + "=" + value)) {}

  ~ScopedSessionParam() {
    if (active_) {
      execute("ALTER SESSION UNSET " + param_);
    }
  }

  [[nodiscard]] bool is_active() const { return active_; }

  static ScopedSessionParam use_connection_ctx(SQLHDBC dbc) {
    return ScopedSessionParam(dbc, "CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX", "true");
  }

  ScopedSessionParam(const ScopedSessionParam&) = delete;
  ScopedSessionParam& operator=(const ScopedSessionParam&) = delete;

  ScopedSessionParam(ScopedSessionParam&& other) noexcept
      : dbc_(other.dbc_), param_(std::move(other.param_)), active_(other.active_) {
    other.dbc_ = SQL_NULL_HDBC;
    other.param_.clear();
    other.active_ = false;
  }

  ScopedSessionParam& operator=(ScopedSessionParam&& other) noexcept {
    if (this != &other) {
      if (active_) {
        execute("ALTER SESSION UNSET " + param_);
      }
      dbc_ = other.dbc_;
      param_ = std::move(other.param_);
      active_ = other.active_;
      other.dbc_ = SQL_NULL_HDBC;
      other.param_.clear();
      other.active_ = false;
    }
    return *this;
  }

 private:
  bool execute(const std::string& sql) {
    if (dbc_ == SQL_NULL_HDBC) return false;
    SQLHSTMT stmt = SQL_NULL_HSTMT;
    if (!SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_STMT, dbc_, &stmt))) return false;
    const SQLRETURN ret = SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);
    SQLFreeStmt(stmt, SQL_CLOSE);
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    return SQL_SUCCEEDED(ret);
  }

  SQLHDBC dbc_;
  std::string param_;
  bool active_;
};

#endif  // SCOPEDSESSIONPARAM_HPP
