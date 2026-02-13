#ifndef TEST_MACROS_HPP
#define TEST_MACROS_HPP

#include <sql.h>
#include <sqlext.h>

#include <stdexcept>
#include <string>

#include <catch2/catch_test_macros.hpp>

// Query the current database name using a temporary statement on the given connection.
// Allocates and frees its own statement handle so the caller's statement state is not mutated.
inline std::string get_current_database(SQLHDBC dbc) {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
  if (!SQL_SUCCEEDED(ret)) {
    throw std::runtime_error("get_current_database: SQLAllocHandle(SQL_HANDLE_STMT) failed");
  }

  ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT CURRENT_DATABASE()")), SQL_NTS);
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

// Helper macro to check for expected ODBC error with specific SQLSTATE
#define REQUIRE_EXPECTED_ERROR(ret, expectedState, handle, handleType)                \
  do {                                                                                \
    REQUIRE(ret == SQL_ERROR);                                                        \
    const auto __odbc_test_diag_records__ = get_diag_rec(handleType, handle);         \
    REQUIRE(!__odbc_test_diag_records__.empty());                                     \
    INFO("SQLSTATE: " << __odbc_test_diag_records__[0].sqlState                       \
                      << ", Message: " << __odbc_test_diag_records__[0].messageText); \
    REQUIRE(__odbc_test_diag_records__[0].sqlState == expectedState);                 \
  } while (0)

// Helper macro to check for expected ODBC warning (SQL_SUCCESS_WITH_INFO) with specific SQLSTATE
#define REQUIRE_EXPECTED_WARNING(ret, expectedState, handle, handleType)              \
  do {                                                                                \
    REQUIRE(ret == SQL_SUCCESS_WITH_INFO);                                            \
    const auto __odbc_test_diag_records__ = get_diag_rec(handleType, handle);         \
    REQUIRE(!__odbc_test_diag_records__.empty());                                     \
    INFO("SQLSTATE: " << __odbc_test_diag_records__[0].sqlState                       \
                      << ", Message: " << __odbc_test_diag_records__[0].messageText); \
    REQUIRE(__odbc_test_diag_records__[0].sqlState == expectedState);                 \
  } while (0)

#endif
