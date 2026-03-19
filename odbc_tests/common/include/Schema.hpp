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
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_return_code.hpp"

class Schema {
 public:
  Schema(Connection& conn, const std::string& schema_name)
      : execute_fn([&conn](const std::string& sql) { conn.execute(sql); }), schema_name(schema_name) {
    SQLHANDLE dbc_handle = conn.handleWrapper().getHandle();
    create_schema_with_diagnostics(dbc_handle);
    use_schema_with_retry(dbc_handle);
  }

  Schema(const SQLHDBC dbc, const std::string& schema_name)
      : execute_fn(make_dbc_executor(dbc)), schema_name(schema_name) {
    create_schema_with_diagnostics(dbc);
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

  void create_schema_with_diagnostics(SQLHANDLE dbc) {
    const std::string sql = "CREATE SCHEMA IF NOT EXISTS " + schema_name;

    SQLHSTMT stmt = SQL_NULL_HSTMT;
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
    if (!SQL_SUCCEEDED(ret)) {
      WARN("[Schema] CREATE: SQLAllocHandle failed for " << schema_name);
      execute_fn(sql);
      return;
    }

    ret = SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);

    auto diags = get_diag_rec(SQL_HANDLE_STMT, stmt);
    SQLLEN row_count = -1;
    SQLRowCount(stmt, &row_count);

    WARN("[Schema] CREATE " << schema_name << ": ret=" << return_code_to_string(ret) << " row_count=" << row_count
                            << " diag_count=" << diags.size());

    for (size_t i = 0; i < diags.size(); ++i) {
      WARN("[Schema]   diag[" << i << "] SQLSTATE=" << diags[i].sqlState << " NativeError=" << diags[i].nativeError
                              << " " << diags[i].messageText);
    }

    if (SQL_SUCCEEDED(ret) && row_count != 0) {
      SQLSMALLINT col_count = 0;
      SQLNumResultCols(stmt, &col_count);
      while (SQLFetch(stmt) == SQL_SUCCESS) {
        std::string row;
        for (SQLSMALLINT col = 1; col <= col_count; ++col) {
          char buf[1024] = {};
          SQLLEN indicator = 0;
          SQLGetData(stmt, col, SQL_C_CHAR, buf, sizeof(buf), &indicator);
          if (!row.empty()) row += ", ";
          row += (indicator == SQL_NULL_DATA) ? "NULL" : std::string(buf);
        }
        WARN("[Schema]   row: " << row);
      }
    }

    SQLFreeStmt(stmt, SQL_CLOSE);
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);

    if (!SQL_SUCCEEDED(ret)) {
      execute_fn(sql);
    }
  }

  // Retry USE SCHEMA with exponential backoff: 250ms, 500ms, 1000ms (~2s total),
  // but only for SQLSTATE 42000 ("Object does not exist") to avoid masking real errors.
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

      if (SQL_SUCCEEDED(ret)) {
        SQLFreeStmt(stmt, SQL_CLOSE);
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        return;
      }

      std::string state = get_sqlstate(SQL_HANDLE_STMT, stmt);
      SQLFreeStmt(stmt, SQL_CLOSE);
      SQLFreeHandle(SQL_HANDLE_STMT, stmt);

      if (state != "42000") {
        break;
      }

      WARN("[Schema] USE SCHEMA " << schema_name << " failed (SQLSTATE 42000), retrying in " << delay_ms << "ms");
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
