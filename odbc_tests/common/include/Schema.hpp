#ifndef SCHEMA_HPP
#define SCHEMA_HPP

#include <sql.h>
#include <sqlext.h>

#include <cstdlib>
#include <random>
#include <stdexcept>
#include <string>

#include "Connection.hpp"
#include "odbc_cast.hpp"
#include "test_setup.hpp"

// Static utility class that manages the test schema for the current process.
//
// Two modes of operation:
//
// 1. Shared schema (ODBC_TEST_SCHEMA env var set by runner script):
//    The runner pre-creates a single schema for all test processes.
//    use_temp_session_schema() just issues USE SCHEMA — no CREATE.
//    The runner's trap/finally drops the schema on exit.
//
// 2. Fallback (no env var — IDE, direct ctest, individual binary):
//    Generates a random schema name, issues CREATE SCHEMA IF NOT EXISTS.
//    The Catch2 SchemaCleanupListener drops it in testRunEnded.
//
// Schemas are NOT dropped via std::atexit because the ODBC driver's Rust
// runtime tears down TLS before C++ atexit handlers run, making any ODBC
// call from atexit unsafe (causes abort via Rust panic on macOS ARM64).
class Schema {
 public:
  Schema() = delete;
  Schema(const Schema&) = delete;
  Schema& operator=(const Schema&) = delete;
  Schema(Schema&&) = delete;
  Schema& operator=(Schema&&) = delete;

  static void use_temp_session_schema(Connection& conn) {
    used_ = true;
    if (!is_external_schema()) {
      conn.execute("CREATE SCHEMA IF NOT EXISTS " + session_schema_name());
    }
    conn.execute("USE SCHEMA " + session_schema_name());
  }

  static void use_temp_session_schema(SQLHDBC dbc) {
    used_ = true;
    if (!is_external_schema()) {
      execute_on_dbc(dbc, "CREATE SCHEMA IF NOT EXISTS " + session_schema_name());
    }
    execute_on_dbc(dbc, "USE SCHEMA " + session_schema_name());
  }

  static const std::string& name() { return session_schema_name(); }

  static bool is_external_schema() {
    static const bool external = (std::getenv("ODBC_TEST_SCHEMA") != nullptr);
    return external;
  }

  static bool was_used() { return used_; }

 private:
  static inline bool used_ = false;

  static const std::string& session_schema_name() {
    static std::string schema_name = [] {
      const char* env = std::getenv("ODBC_TEST_SCHEMA");
      return env ? std::string(env) : generate_random_name();
    }();
    return schema_name;
  }

  static std::string generate_random_name() {
    std::random_device rd;
    std::mt19937_64 gen(rd());
    return "TEMP_TEST_SCHEMA_" + std::to_string(gen());
  }

  static void execute_on_dbc(const SQLHDBC dbc, const std::string& sql) {
    SQLHSTMT stmt = SQL_NULL_HSTMT;
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
    if (!SQL_SUCCEEDED(ret)) {
      throw std::runtime_error("Schema: SQLAllocHandle failed for: " + sql);
    }
    ret = SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);
    if (!SQL_SUCCEEDED(ret)) {
      SQLFreeHandle(SQL_HANDLE_STMT, stmt);
      throw std::runtime_error("Schema: SQLExecDirect failed for: " + sql);
    }
    SQLFreeStmt(stmt, SQL_CLOSE);
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  }
};

#endif  // SCHEMA_HPP
