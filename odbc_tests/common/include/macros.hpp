#ifndef ODBC_TESTS_MACROS_HPP
#define ODBC_TESTS_MACROS_HPP

#include "odbc_matchers.hpp"

// Deprecated – use REQUIRE_ODBC(ret, handle) from odbc_matchers.hpp.
#ifdef __GNUC__
#define CHECK_ODBC(ret, handle) \
  _Pragma("GCC warning \"CHECK_ODBC is deprecated, use REQUIRE_ODBC from odbc_matchers.hpp\"") REQUIRE_ODBC(ret, handle)
#define CHECK_ODBC_ERROR(ret, handle, handleType)                                   \
  _Pragma("GCC warning \"CHECK_ODBC_ERROR is deprecated – see odbc_matchers.hpp\"") \
      REQUIRE_THAT(OdbcResult(ret, handleType, handle), OdbcMatchers::Succeeded())
#elif defined(_MSC_VER)
#define CHECK_ODBC(ret, handle) \
  __pragma(message("CHECK_ODBC is deprecated, use REQUIRE_ODBC from odbc_matchers.hpp")) REQUIRE_ODBC(ret, handle)
#define CHECK_ODBC_ERROR(ret, handle, handleType)                             \
  __pragma(message("CHECK_ODBC_ERROR is deprecated – see odbc_matchers.hpp")) \
      REQUIRE_THAT(OdbcResult(ret, handleType, handle), OdbcMatchers::Succeeded())
#else
#define CHECK_ODBC(ret, handle) REQUIRE_ODBC(ret, handle)
#define CHECK_ODBC_ERROR(ret, handle, handleType) \
  REQUIRE_THAT(OdbcResult(ret, handleType, handle), OdbcMatchers::Succeeded())
#endif

#endif  // ODBC_TESTS_MACROS_HPP
