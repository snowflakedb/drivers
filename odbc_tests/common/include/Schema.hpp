#ifndef SCHEMA_HPP
#define SCHEMA_HPP

#include <sql.h>
#include <sqlext.h>

#include <iostream>
#include <mutex>
#include <random>
#include <stdexcept>
#include <string>

#include "Connection.hpp"
#include "odbc_cast.hpp"
#include "test_setup.hpp"

// Static utility class that manages a single per-process test schema.
// First call creates the schema on Snowflake and registers atexit cleanup.
// Subsequent calls just switch the connection to the existing schema.
class Schema {
 public:
  Schema() = delete;
  Schema(const Schema&) = delete;
  Schema& operator=(const Schema&) = delete;
  Schema(Schema&&) = delete;
  Schema& operator=(Schema&&) = delete;

  static void use_temp_session_schema(Connection& conn) {
    if (first_call()) {
      conn.execute("CREATE SCHEMA IF NOT EXISTS " + session_schema_name());
    }
    conn.execute("USE SCHEMA " + session_schema_name());
  }

  static void use_temp_session_schema(SQLHDBC dbc) {
    if (first_call()) {
      execute_on_dbc(dbc, "CREATE SCHEMA IF NOT EXISTS " + session_schema_name());
    }
    execute_on_dbc(dbc, "USE SCHEMA " + session_schema_name());
  }

  static const std::string& name() { return session_schema_name(); }

 private:
  static const std::string& session_schema_name() {
    static std::string schema_name = generate_random_name();
    return schema_name;
  }

  static std::string generate_random_name() {
    std::random_device rd;
    std::mt19937_64 gen(rd());
    return "TEMP_TEST_SCHEMA_" + std::to_string(gen());
  }

  // Returns true on the very first call; false on all subsequent calls.
  // Caches the connection string and registers atexit cleanup exactly once.
  // Thread-safe via std::call_once.
  static bool first_call() {
    static std::once_flag flag;
    bool is_first = false;
    std::call_once(flag, [&] {
      cached_connection_string() = get_connection_string();
      // Touch session_schema_name() before registering the atexit handler so
      // its function-local static is initialized (and therefore destroyed)
      // after cleanup_session_schema runs at process exit.
      (void)session_schema_name();
      std::atexit(cleanup_session_schema);
      is_first = true;
    });
    return is_first;
  }

  static std::string& cached_connection_string() {
    static std::string cs;
    return cs;
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

  // Best-effort cleanup at process exit using raw ODBC (no Catch2 macros).
  static void cleanup_session_schema() {
    const auto& cs = cached_connection_string();
    if (cs.empty()) return;

    SQLHENV env = SQL_NULL_HENV;
    SQLHDBC dbc = SQL_NULL_HDBC;
    if (!SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env))) return;
    SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
    if (!SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc))) {
      SQLFreeHandle(SQL_HANDLE_ENV, env);
      return;
    }
    if (SQL_SUCCEEDED(
            SQLDriverConnect(dbc, nullptr, sqlchar(cs.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT))) {
      SQLHSTMT stmt = SQL_NULL_HSTMT;
      if (SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt))) {
        const std::string sql = "DROP SCHEMA IF EXISTS " + session_schema_name() + " CASCADE";
        SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
      }
      SQLDisconnect(dbc);
    }
    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
  }
};

#endif  // SCHEMA_HPP
