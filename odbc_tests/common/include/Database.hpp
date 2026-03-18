#ifndef DATABASE_HPP
#define DATABASE_HPP

#include <sql.h>
#include <sqlext.h>

#include <random>
#include <stdexcept>
#include <string>

#include "odbc_cast.hpp"

// Process-scoped shared database for catalog tests.
//
// Creates a small, isolated database on the first call to use() and
// switches every subsequent connection to it via USE DATABASE.  Because the
// database contains only the objects each test creates (in random schemas),
// information_schema queries are fast.
//
// The database is NOT dropped automatically -- each test creates its own
// connection, so there is no single connection that outlives them all.
// Snowflake account-level cleanup (or a CI post-step) should remove stale
// CATALOG_TEST_DB_* databases.
class CatalogTestDatabase {
 public:
  CatalogTestDatabase() = delete;

  // Ensures the shared catalog database exists and is the active database
  // on the given connection.  The first call creates the database; subsequent
  // calls only execute USE DATABASE.
  static void use(SQLHDBC dbc) {
    if (name_.empty()) {
      name_ = generate_name();
      execute(dbc, "CREATE DATABASE IF NOT EXISTS " + name_);
    }
    execute(dbc, "USE DATABASE " + name_);
  }

  static const std::string& name() { return name_; }

 private:
  static std::string generate_name() {
    std::random_device rd;
    std::mt19937_64 gen(rd());
    return "CATALOG_TEST_DB_" + std::to_string(gen());
  }

  static void execute(SQLHDBC dbc, const std::string& sql) {
    SQLHSTMT stmt = SQL_NULL_HSTMT;
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
    if (!SQL_SUCCEEDED(ret)) {
      throw std::runtime_error("CatalogTestDatabase: SQLAllocHandle failed for: " + sql);
    }
    ret = SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);
    SQLFreeStmt(stmt, SQL_CLOSE);
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    if (!SQL_SUCCEEDED(ret)) {
      throw std::runtime_error("CatalogTestDatabase: SQLExecDirect failed for: " + sql);
    }
  }

  static inline std::string name_;
};

#endif  // DATABASE_HPP
