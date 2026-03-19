#ifndef COMPATIBILITY_HPP
#define COMPATIBILITY_HPP

#include <cstdlib>
#ifndef _WIN32
#include <locale>
#endif

#include <catch2/catch_test_macros.hpp>

// Cross-platform process ID
#ifdef _WIN32
#include <process.h>
#define GET_PROCESS_ID() _getpid()
#else
#include <unistd.h>

#include <cstring>
#define GET_PROCESS_ID() getpid()
#endif

enum class DRIVER_TYPE {
  NEW = 0,
  OLD = 1,
};

enum class PLATFORM {
  PLATFORM_WINDOWS = 0,
  PLATFORM_LINUX = 1,
  PLATFORM_MACOS = 2,
  PLATFORM_UNKNOWN = 3,
};

extern PLATFORM get_platform();

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

#ifdef FORCE_RUN_NOT_IMPLEMENTED
#define SKIP_NEW_DRIVER_NOT_IMPLEMENTED() ((void)0)
#else
#define SKIP_NEW_DRIVER_NOT_IMPLEMENTED()        \
  do {                                           \
    if (get_driver_type() == DRIVER_TYPE::NEW) { \
      SKIP("Not implemented for new driver");    \
    }                                            \
  } while (0)
#endif

// On Windows the ODBC driver interprets UTF-8 wire bytes as Windows-1252 and
// re-encodes them to UTF-8 (double-encoding).  SQL_C_BINARY therefore returns
// different byte sequences than on Unix/Linux where raw UTF-8 is preserved.
// Use WINDOWS_ONLY / UNIX_ONLY to gate platform-specific assertions.
#define WINDOWS_ONLY if (get_platform() == PLATFORM::PLATFORM_WINDOWS)
#define UNIX_ONLY if (get_platform() == PLATFORM::PLATFORM_LINUX || get_platform() == PLATFORM::PLATFORM_MACOS)

inline bool is_ascii_locale() {
#ifdef _WIN32
  return false;
#else
  setlocale(LC_CTYPE, "");
  const char* locale = setlocale(LC_CTYPE, nullptr);
  return locale != nullptr && (std::string(locale) == "C" || std::string(locale) == "POSIX");
#endif
}

#ifdef _WIN32
#define SKIP_WINDOWS_STRING_ENCODING() \
  SKIP("String encoding not yet supported on Windows (UTF-8 vs Windows-1252 issue)")
#else
#define SKIP_WINDOWS_STRING_ENCODING() ((void)0)
#endif

#define REQUIRE_VPN(message)                              \
  do {                                                    \
    if (std::getenv("JENKINS_URL") == nullptr) {          \
      SKIP("Requires VPN (run on Jenkins): " << message); \
    }                                                     \
  } while (0)

#endif  // COMPATIBILITY_HPP
