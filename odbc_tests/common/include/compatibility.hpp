#ifndef COMPATIBILITY_HPP
#define COMPATIBILITY_HPP

#include <cstdlib>

#include <catch2/catch_test_macros.hpp>

// Cross-platform process ID
#ifdef _WIN32
#include <process.h>
#define GET_PROCESS_ID() _getpid()
#else
#include <unistd.h>
#define GET_PROCESS_ID() getpid()
#endif

enum class DRIVER_TYPE {
  NEW = 0,
  OLD = 1,
};

extern DRIVER_TYPE get_driver_type();

#define NEW_DRIVER_ONLY(x) if (get_driver_type() == DRIVER_TYPE::NEW)

#define OLD_DRIVER_ONLY(x) if (get_driver_type() == DRIVER_TYPE::OLD)

#define SKIP_OLD_DRIVER(bd, message)                            \
  if (get_driver_type() == DRIVER_TYPE::OLD) {                  \
    SKIP("Skipping for old driver: " << bd << ": " << message); \
  }

#define SKIP_NEW_DRIVER(bd, message)                            \
  if (get_driver_type() == DRIVER_TYPE::NEW) {                  \
    SKIP("Skipping for new driver: " << bd << ": " << message); \
  }

#define SKIP_NEW_DRIVER_NOT_IMPLEMENTED()        \
  do {                                           \
    if (get_driver_type() == DRIVER_TYPE::NEW) { \
      SKIP("Not implemented for new driver");    \
    }                                            \
  } while (0)

// On Windows the ODBC driver interprets UTF-8 wire bytes as Windows-1252 and
// re-encodes them to UTF-8 (double-encoding).  SQL_C_BINARY therefore returns
// different byte sequences than on Unix/Linux where raw UTF-8 is preserved.
// Use WINDOWS_ONLY / UNIX_ONLY to gate platform-specific assertions.
#ifdef _WIN32
#define WINDOWS_ONLY
#define UNIX_ONLY if (false)
#else
#define WINDOWS_ONLY if (false)
#define UNIX_ONLY
#endif

#define REQUIRE_VPN(message)                              \
  do {                                                    \
    if (std::getenv("JENKINS_URL") == nullptr) {          \
      SKIP("Requires VPN (run on Jenkins): " << message); \
    }                                                     \
  } while (0)

#endif  // COMPATIBILITY_HPP
