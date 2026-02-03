#ifndef ODBC_TESTS_MACROS_HPP
#define ODBC_TESTS_MACROS_HPP

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

inline std::string return_code_to_string(SQLRETURN ret) {
  switch (ret) {
    case SQL_SUCCESS:
      return "SQL_SUCCESS";
    case SQL_SUCCESS_WITH_INFO:
      return "SQL_SUCCESS_WITH_INFO";
    case SQL_ERROR:
      return "SQL_ERROR";
    case SQL_INVALID_HANDLE:
      return "SQL_INVALID_HANDLE";
    default:
      return "UNKNOWN_RETURN_CODE(" + std::to_string(ret) + ")";
  }
}

#define CHECK_ODBC(ret, handle) CHECK_ODBC_ERROR(ret, handle.getHandle(), handle.getType())

#define CHECK_ODBC_ERROR(ret, handle, handleType)                                                                   \
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {                                                         \
    if (ret == SQL_INVALID_HANDLE) {                                                                                \
      FAIL("ODBC Error Status:" << return_code_to_string(ret) << " (SQL_INVALID_HANDLE). "                          \
                                << "HandleType=" << handleType << " Handle=" << handle);                            \
    }                                                                                                               \
    SQLINTEGER nativeError = 0;                                                                                     \
    SQLCHAR state[1024] = {0};                                                                                      \
    SQLCHAR message[1024] = {0};                                                                                    \
    SQLRETURN diag_ret = SQLGetDiagRec(handleType, handle, 1, state, &nativeError, message, sizeof(message), NULL); \
    if (diag_ret == SQL_SUCCESS || diag_ret == SQL_SUCCESS_WITH_INFO) {                                             \
      FAIL("ODBC Error Status:" << ret << " Error: " << message << " State: " << state                              \
                                << " NativeError: " << nativeError);                                                \
    } else {                                                                                                        \
      FAIL("ODBC Error Status:" << ret << " (no diagnostics; SQLGetDiagRec ret=" << diag_ret                        \
                                << "). HandleType=" << handleType << " Handle=" << handle);                         \
    }                                                                                                               \
  }

#endif  // ODBC_TESTS_MACROS_HPP
