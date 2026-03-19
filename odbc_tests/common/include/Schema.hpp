#ifndef SCHEMA_HPP
#define SCHEMA_HPP

#include <sql.h>
#include <sqlext.h>

#include <functional>
#include <random>
#include <stdexcept>
#include <string>

#include "Connection.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_return_code.hpp"

class Schema {
 public:
  Schema(Connection& conn, const std::string& schema_name)
      : execute_fn([&conn](const std::string& sql) { conn.execute(sql); }), schema_name(schema_name) {
    create_schema_with_diagnostics(conn.handleWrapper().getHandle());
    execute_fn("USE SCHEMA " + schema_name);
  }

  Schema(const SQLHDBC dbc, const std::string& schema_name)
      : execute_fn(make_dbc_executor(dbc)), schema_name(schema_name) {
    create_schema_with_diagnostics(dbc);
    execute_fn("USE SCHEMA " + schema_name);
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

    if (!SQL_SUCCEEDED(ret)) {
      auto diags = get_diag_rec(SQL_HANDLE_STMT, stmt);
      SQLLEN row_count = -1;
      SQLRowCount(stmt, &row_count);

      WARN("[Schema] CREATE " << schema_name << " FAILED: ret=" << return_code_to_string(ret)
                              << " row_count=" << row_count << " diag_count=" << diags.size());

      for (size_t i = 0; i < diags.size(); ++i) {
        WARN("[Schema]   diag[" << i << "] SQLSTATE=" << diags[i].sqlState << " NativeError=" << diags[i].nativeError
                                << " " << diags[i].messageText);
      }

      SQLFreeStmt(stmt, SQL_CLOSE);
      SQLFreeHandle(SQL_HANDLE_STMT, stmt);
      execute_fn(sql);
      return;
    }

    SQLFreeStmt(stmt, SQL_CLOSE);
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
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
