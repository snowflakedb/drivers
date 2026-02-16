#ifndef TEST_MACROS_HPP
#define TEST_MACROS_HPP

#include <catch2/catch_test_macros.hpp>

#include "get_diag_rec.hpp"

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
