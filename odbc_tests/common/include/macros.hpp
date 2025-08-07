#ifndef ODBC_TESTS_MACROS_HPP
#define ODBC_TESTS_MACROS_HPP

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#define CHECK_ODBC(ret, handle) CHECK_ODBC_ERROR(ret, handle.getHandle(), handle.getType())

#define CHECK_ODBC_ERROR(ret, handle, handleType)                                              \
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {                                    \
    SQLINTEGER nativeError;                                                                    \
    SQLCHAR state[1024];                                                                       \
    SQLCHAR message[1024];                                                                     \
    SQLGetDiagRec(handleType, handle, 1, state, &nativeError, message, sizeof(message), NULL); \
    FAIL("ODBC Error Status:" << ret << " Error: " << message << " State: " << state);         \
    REQUIRE(false);                                                                            \
  }

#endif  // ODBC_TESTS_MACROS_HPP
