#ifndef COMPATIBILITY_HPP
#define COMPATIBILITY_HPP

#include <cstdlib>
#include <string>
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

enum class ARCH {
  ARCH_X86_64 = 0,
  ARCH_AARCH64 = 1,
  ARCH_UNKNOWN = 2,
};

extern PLATFORM get_platform();

extern ARCH get_arch();

extern DRIVER_TYPE get_driver_type();

extern bool is_iodbc_test_suite();

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

#ifdef ENABLE_PROGRESS_REPORT
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

#define REQUIRE_DAILY_RUN_JENKINS_JOB(message)         \
  do {                                                 \
    if (std::getenv("JENKINS_DAILY_JOB") == nullptr) { \
      SKIP("Requires daily jenkins job: " << message); \
    }                                                  \
  } while (0)

// Gate for E2E tests that need the headless browser container
// (snowdrivers-test-external-browser-universal-driver: Chromium + the
// /externalbrowser/*.js automation scripts). Outside that container the
// scripts and the Chromium debug port don't exist, so the test is SKIPPED.
// Mirrors the Python `requires_browser` marker (SF_TEST_HEADLESS_BROWSER=true).
#define REQUIRE_BROWSER(message)                                                                \
  do {                                                                                          \
    const char* headless_browser_env = std::getenv("SF_TEST_HEADLESS_BROWSER");                 \
    if (headless_browser_env == nullptr || std::string(headless_browser_env) != "true") {       \
      SKIP("Requires headless browser container (SF_TEST_HEADLESS_BROWSER=true): " << message); \
    }                                                                                           \
  } while (0)

// Gate for requires_no_mfa tests (parameters_aws_local.json, SF_TEST_NO_MFA=true).
#define REQUIRE_NO_MFA(message)                                                      \
  do {                                                                               \
    const char* no_mfa_env = std::getenv("SF_TEST_NO_MFA");                          \
    if (no_mfa_env == nullptr || std::string(no_mfa_env) != "true") {                \
      SKIP("Requires parameters_aws_local.json (SF_TEST_NO_MFA=true): " << message); \
    }                                                                                \
  } while (0)

// ============================================================================
// Driver-manager-specific compatibility shims
// ============================================================================
//
// iODBC ships an older `<sqlext.h>` that doesn't define every macro from the
// ODBC 3.8 spec (notably `SQL_GD_OUTPUT_PARAMS`, added in ODBC 3.8 to advertise
// `SQLGetData` support against output parameters). unixODBC and Windows DM
// both define them. Pull in `<sqlext.h>` and fill in any missing macros with
// the canonical Microsoft-spec values so test code can reference them
// unconditionally.
#include <sqlext.h>

#ifndef SQL_GD_OUTPUT_PARAMS
#define SQL_GD_OUTPUT_PARAMS 0x00000010L
#endif

// `SQL_OV_ODBC3_80` is the value passed to `SQLSetEnvAttr` /
// `SQL_ATTR_ODBC_VERSION` to opt into ODBC 3.8 behaviors (asynchronous
// statement execution, `SQL_PARAM_DATA_AVAILABLE`, …). iODBC's `<sqlext.h>`
// stops at `SQL_OV_ODBC3` (3UL); the canonical 3.8 value (380UL) below
// matches Microsoft and unixODBC.
#ifndef SQL_OV_ODBC3_80
#define SQL_OV_ODBC3_80 380UL
#endif

// `SQL_API_SQLCANCELHANDLE` is the function ID reported by `SQLGetFunctions`
// for ODBC 3.8's `SQLCancelHandle`. Microsoft and unixODBC define it as 1022;
// iODBC ships an older `<sql.h>` that omits it.
#ifndef SQL_API_SQLCANCELHANDLE
#define SQL_API_SQLCANCELHANDLE 1022
#endif

// ODBC 3.8 `SQLGetInfo` info-type IDs and their associated bitmask values.
// Microsoft and unixODBC define them; iODBC's `<sqlext.h>` does not. Tests
// that probe `SQLGetInfo(SQL_ASYNC_DBC_FUNCTIONS)` etc. need these symbols at
// compile time even though the actual driver always returns the
// "not-capable" value.
#ifndef SQL_ASYNC_DBC_FUNCTIONS
#define SQL_ASYNC_DBC_FUNCTIONS 10023
#endif
#ifndef SQL_ASYNC_DBC_NOT_CAPABLE
#define SQL_ASYNC_DBC_NOT_CAPABLE 0x00000000L
#endif
#ifndef SQL_ASYNC_DBC_CAPABLE
#define SQL_ASYNC_DBC_CAPABLE 0x00000001L
#endif
#ifndef SQL_ASYNC_NOTIFICATION
#define SQL_ASYNC_NOTIFICATION 10025
#endif
#ifndef SQL_ASYNC_NOTIFICATION_NOT_CAPABLE
#define SQL_ASYNC_NOTIFICATION_NOT_CAPABLE 0x00000000L
#endif
#ifndef SQL_ASYNC_NOTIFICATION_CAPABLE
#define SQL_ASYNC_NOTIFICATION_CAPABLE 0x00000001L
#endif
#ifndef SQL_DRIVER_AWARE_POOLING_SUPPORTED
#define SQL_DRIVER_AWARE_POOLING_SUPPORTED 10024
#endif
#ifndef SQL_DRIVER_AWARE_POOLING_NOT_CAPABLE
#define SQL_DRIVER_AWARE_POOLING_NOT_CAPABLE 0x00000000L
#endif
#ifndef SQL_DRIVER_AWARE_POOLING_CAPABLE
#define SQL_DRIVER_AWARE_POOLING_CAPABLE 0x00000001L
#endif

// Connection-attribute / value pair for ODBC 3.8 asynchronous connection
// operations (`SQLConnect`, `SQLDriverConnect`, ...). Not exposed by iODBC.
#ifndef SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE
#define SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE 117
#endif
#ifndef SQL_ASYNC_DBC_ENABLE_ON
#define SQL_ASYNC_DBC_ENABLE_ON 1UL
#endif
#ifndef SQL_ASYNC_DBC_ENABLE_OFF
#define SQL_ASYNC_DBC_ENABLE_OFF 0UL
#endif

// `SQL_CVT_GUID` is the bitmask flag returned by `SQLGetInfo` for the
// `SQL_CONVERT_*` info types when the driver supports converting to GUID.
// Microsoft / unixODBC: `0x01000000L`. iODBC's `<sqlext.h>` omits it.
#ifndef SQL_CVT_GUID
#define SQL_CVT_GUID 0x01000000L
#endif

// iODBC's `<sqltypes.h>` always pulls in `<iodbcunix.h>`, which defines
// `_IODBCUNIX_H`. Use that purely as a header-presence check to fill in any
// ODBC-3.8 entry point that the iODBC headers don't ship so call sites stay
// compilable regardless of which DM's headers we built against.
#ifdef _IODBCUNIX_H
// `SQLCancelHandle` was added in ODBC 3.8 and is exposed by both the
// Microsoft DM and unixODBC, but iODBC never picked it up. Provide a stub
// that returns `SQL_INVALID_HANDLE` (matching the unixODBC behavior tests
// already assert) so iODBC-header builds link; tests that exercise this
// entry point should `SKIP_IODBC()` at runtime - calling the stub would
// still yield correct return-code semantics, but we'd rather signal
// "untested" than silently report a synthetic value.
static inline SQLRETURN SQLCancelHandle(SQLSMALLINT /*handle_type*/, SQLHANDLE /*handle*/) {
  return SQL_INVALID_HANDLE;
}
#endif  // _IODBCUNIX_H

#define SKIP_IODBC(message)                 \
  do {                                      \
    if (is_iodbc_test_suite()) {            \
      SKIP("Skipping for iODBC: " message); \
    }                                       \
  } while (0)
#define IODBC_ONLY if (is_iodbc_test_suite())
#define NON_IODBC if (!is_iodbc_test_suite())

#define SKIP_OLD_IODBC(bd, message)                                           \
  do {                                                                        \
    if (is_iodbc_test_suite() && get_driver_type() == DRIVER_TYPE::OLD) {     \
      SKIP("Skipping for old driver under iODBC: " << bd << ": " << message); \
    }                                                                         \
  } while (0)
#define SKIP_NEW_IODBC(bd, message)                                           \
  do {                                                                        \
    if (is_iodbc_test_suite() && get_driver_type() == DRIVER_TYPE::NEW) {     \
      SKIP("Skipping for new driver under iODBC: " << bd << ": " << message); \
    }                                                                         \
  } while (0)
#define OLD_IODBC_ONLY(x) if (is_iodbc_test_suite() && get_driver_type() == DRIVER_TYPE::OLD)
#define NEW_IODBC_ONLY(x) if (is_iodbc_test_suite() && get_driver_type() == DRIVER_TYPE::NEW)

#endif  // COMPATIBILITY_HPP
