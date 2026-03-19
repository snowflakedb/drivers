#ifndef SCHEMA_HPP
#define SCHEMA_HPP

#include <sql.h>
#include <sqlext.h>

#include <chrono>
#include <functional>
#include <random>
#include <stdexcept>
#include <string>
#include <thread>

#include "Connection.hpp"
#include "odbc_cast.hpp"

class Schema {
 public:
  Schema(Connection& conn, const std::string& schema_name)
      : execute_fn([&conn](const std::string& sql) { conn.execute(sql); }), schema_name(schema_name) {
    execute_fn("CREATE SCHEMA IF NOT EXISTS " + schema_name);
    use_schema_with_retry(conn.handleWrapper().getHandle());
  }

  Schema(const SQLHDBC dbc, const std::string& schema_name)
      : execute_fn(make_dbc_executor(dbc)), schema_name(schema_name) {
    execute_fn("CREATE SCHEMA IF NOT EXISTS " + schema_name);
    use_schema_with_retry(dbc);
  }

  static Schema use_random_schema(Connection& conn) { return Schema(conn, generate_random_name()); }

  static Schema use_random_schema(const SQLHDBC dbc) { return Schema(dbc, generate_random_name()); }

  const std::string& name() const { return schema_name; }

  ~Schema() {
    if (execute_fn) {
      execute_fn("DROP SCHEMA IF EXISTS " + schema_name + " CASCADE");
    }
  }

  Schema(const Schema&) = delete;
  Schema& operator=(const Schema&) = delete;

  Schema(Schema&& other) noexcept : execute_fn(std::move(other.execute_fn)), schema_name(std::move(other.schema_name)) {
    other.execute_fn = nullptr;
    other.schema_name.clear();
  }

  Schema& operator=(Schema&& other) noexcept {
    if (this != &other) {
      if (execute_fn) {
        execute_fn("DROP SCHEMA IF EXISTS " + schema_name + " CASCADE");
      }
      execute_fn = std::move(other.execute_fn);
      schema_name = std::move(other.schema_name);
      other.execute_fn = nullptr;
      other.schema_name.clear();
    }
    return *this;
  }

 private:
  static std::string generate_random_name() {
    std::random_device rd;
    std::mt19937_64 gen(rd());
    return "SCHEMA_" + std::to_string(gen());
  }

  // Retry USE SCHEMA with exponential backoff: 250ms, 500ms, 1000ms (~2s total).
  // On the final attempt, fall through to execute_fn so failures produce
  // the same diagnostics as the original non-retry code path.
  void use_schema_with_retry(SQLHANDLE dbc) {
    static constexpr int delays_ms[] = {250, 500, 1000};
    const std::string sql = "USE SCHEMA " + schema_name;

    for (int delay_ms : delays_ms) {
      SQLHSTMT stmt = SQL_NULL_HSTMT;
      SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
      if (!SQL_SUCCEEDED(ret)) {
        break;
      }

      ret = SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);
      SQLFreeStmt(stmt, SQL_CLOSE);
      SQLFreeHandle(SQL_HANDLE_STMT, stmt);

      if (SQL_SUCCEEDED(ret)) {
        return;
      }

      std::fprintf(stderr, "[Schema] USE SCHEMA %s failed, retrying in %dms\n", schema_name.c_str(), delay_ms);
      std::this_thread::sleep_for(std::chrono::milliseconds(delay_ms));
    }

    execute_fn(sql);
  }

  static std::function<void(const std::string&)> make_dbc_executor(SQLHDBC dbc) {
    return [dbc](const std::string& sql) {
      SQLHSTMT stmt = SQL_NULL_HSTMT;
      SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
      if (!SQL_SUCCEEDED(ret)) {
        throw std::runtime_error("Schema: SQLAllocHandle(SQL_HANDLE_STMT) failed for: " + sql);
      }
      ret = SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);
      if (!SQL_SUCCEEDED(ret)) {
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        throw std::runtime_error("Schema: SQLExecDirect failed for: " + sql);
      }
      SQLFreeStmt(stmt, SQL_CLOSE);
      SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    };
  }

  std::function<void(const std::string&)> execute_fn;
  std::string schema_name;
};

#endif  // SCHEMA_HPP
