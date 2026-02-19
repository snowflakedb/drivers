#include "query_helpers.hpp"

#include <sql.h>
#include <sqlext.h>

#include <stdexcept>

#include "odbc_cast.hpp"

std::string get_current_database(SQLHDBC dbc) {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
  if (!SQL_SUCCEEDED(ret)) {
    throw std::runtime_error("get_current_database: SQLAllocHandle(SQL_HANDLE_STMT) failed");
  }

  ret = SQLExecDirect(stmt, sqlchar("SELECT CURRENT_DATABASE()"), SQL_NTS);
  if (!SQL_SUCCEEDED(ret)) {
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    throw std::runtime_error("get_current_database: SQLExecDirect failed");
  }

  ret = SQLFetch(stmt);
  if (!SQL_SUCCEEDED(ret)) {
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    throw std::runtime_error("get_current_database: SQLFetch failed");
  }

  char db[256] = {};
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt, 1, SQL_C_CHAR, db, sizeof(db), &indicator);
  if (!SQL_SUCCEEDED(ret) || indicator == SQL_NULL_DATA) {
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    throw std::runtime_error("get_current_database: SQLGetData failed or returned NULL");
  }

  SQLFreeStmt(stmt, SQL_CLOSE);
  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  return std::string(db);
}
