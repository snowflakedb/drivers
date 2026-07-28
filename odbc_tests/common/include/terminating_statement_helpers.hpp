#ifndef TERMINATING_STATEMENT_HELPERS_HPP
#define TERMINATING_STATEMENT_HELPERS_HPP

#include <sql.h>
#include <sqlext.h>

#include <optional>
#include <random>
#include <string>

#ifdef _WIN32
#include <process.h>
#define TERMINATING_STMT_GET_PID() _getpid()
#else
#include <unistd.h>
#define TERMINATING_STMT_GET_PID() getpid()
#endif

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "odbc_cast.hpp"

// Shared helpers for tests that need a second connection on an existing
// environment handle: the terminating_statement transaction suites
// (end_tran_tests.cpp, transact_tests.cpp) and the descriptor swap suite
// (swap_desc_tests.cpp).

// Unique permanent-table name for multi-connection tests that must be visible
// across DBCs in the same session schema. Generated once per test case so
// concurrent CI jobs against the same account cannot collide mid-test.
inline std::string generate_unique_table_name(const std::string& prefix) {
  std::random_device rd;
  std::mt19937_64 gen(rd());
  return prefix + "_" + std::to_string(TERMINATING_STMT_GET_PID()) + "_" + std::to_string(gen());
}

// A second connected connection (plus a statement) allocated on an existing
// environment handle, used to exercise environment-scoped transaction calls
// that span more than one connection. RAII-cleans up via the handle wrappers.
class ExtraConnectedDbc {
 public:
  ExtraConnectedDbc(SQLHENV env, const std::string& dsn) : dbc_wrapper_(env, SQL_HANDLE_DBC) {
    const SQLRETURN ret = SQLConnect(dbc_wrapper_.getHandle(), sqlchar(dsn.c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
    REQUIRE(ret == SQL_SUCCESS);
    stmt_wrapper_.emplace(dbc_wrapper_.getHandle(), SQL_HANDLE_STMT);
  }

  ~ExtraConnectedDbc() {
    stmt_wrapper_.reset();
    if (dbc_wrapper_.getHandle() != SQL_NULL_HDBC) {
      SQLDisconnect(dbc_wrapper_.getHandle());
    }
  }

  [[nodiscard]] SQLHDBC dbc() const { return dbc_wrapper_.getHandle(); }
  [[nodiscard]] SQLHSTMT stmt() const { return stmt_wrapper_->getHandle(); }

  // Does not assert success. Both drivers refuse SQLDisconnect with 25000 while a manual-commit
  // transaction is still open (SNOW-3831236), so callers that need the connection to be genuinely
  // closed afterwards must end the transaction (SQLEndTran commit/rollback) first. The return code
  // is logged so an unexpected failure is observable.
  void disconnect() {
    if (dbc_wrapper_.getHandle() != SQL_NULL_HDBC) {
      const SQLRETURN ret = SQLDisconnect(dbc_wrapper_.getHandle());
      INFO("SQLDisconnect returned " << ret);
    }
  }

 private:
  ConnectionHandleWrapper dbc_wrapper_;
  std::optional<StatementHandleWrapper> stmt_wrapper_;
};

#endif  // TERMINATING_STATEMENT_HELPERS_HPP
