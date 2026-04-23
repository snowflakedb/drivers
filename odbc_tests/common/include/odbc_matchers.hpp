#ifndef ODBC_MATCHERS_HPP
#define ODBC_MATCHERS_HPP

#include <sql.h>
#include <sqlext.h>

#include <cerrno>
#include <csetjmp>
#include <csignal>
#include <cstring>
#include <string>
#include <vector>

#ifdef _WIN32
#include <windows.h>
#else
#include <sys/wait.h>
#include <unistd.h>
#endif

#include <catch2/catch_test_macros.hpp>
#include <catch2/catch_tostring.hpp>
#include <catch2/matchers/catch_matchers.hpp>

#include "get_diag_rec.hpp"
#include "odbc_return_code.hpp"

// Bundles an ODBC return code with diagnostic records extracted from the handle.
// Construct this and pass it to REQUIRE_THAT / CHECK_THAT with OdbcMatchers.
struct OdbcResult {
  SQLRETURN returnCode;
  std::vector<DiagRec> diagRecords;

  OdbcResult(SQLRETURN ret, SQLSMALLINT handleType, SQLHANDLE handle) : returnCode(ret) {
    if (ret != SQL_SUCCESS && ret != SQL_INVALID_HANDLE) {
      diagRecords = get_diag_rec(handleType, handle);
    }
  }

  OdbcResult(const SQLRETURN ret, const HandleWrapper& handle)
      : OdbcResult(ret, handle.getType(), handle.getHandle()) {}
};

namespace Catch {
template <>
struct StringMaker<OdbcResult> {
  static std::string convert(const OdbcResult& result) {
    std::string out = return_code_to_string(result.returnCode);
    for (size_t i = 0; i < result.diagRecords.size(); ++i) {
      const auto& rec = result.diagRecords[i];
      out += "\n  [" + std::to_string(i) + "] SQLSTATE=" + rec.sqlState +
             " NativeError=" + std::to_string(rec.nativeError) + "\n      " + rec.messageText;
    }
    return out;
  }
};
}  // namespace Catch

// ---------------------------------------------------------------------------
// InvalidHandleProbe — result of a crash-isolated SQLFreeHandle call.
// Used to verify that an ODBC handle has been invalidated without crashing
// the test runner if the Driver Manager dereferences freed memory.
// ---------------------------------------------------------------------------
struct InvalidHandleProbe {
  bool crashed = false;
  SQLRETURN returnCode = SQL_SUCCESS;
};

namespace Catch {
template <>
struct StringMaker<InvalidHandleProbe> {
  static std::string convert(const InvalidHandleProbe& probe) {
    if (probe.crashed) return "handle access caused crash (SIGSEGV/access violation)";
    return "SQLFreeHandle returned " + return_code_to_string(probe.returnCode);
  }
};
}  // namespace Catch

// ---------------------------------------------------------------------------
// probe_invalid_handle — crash-isolated SQLFreeHandle probe.
//
// Per the ODBC spec, using a freed handle is undefined behavior: some Driver
// Managers return SQL_INVALID_HANDLE while others let the driver dereference
// freed memory.  Both outcomes prove the handle is no longer valid.
//
// Platform strategies:
//   MSVC:          SEH __try/__except  (scoped, compiler-native)
//   MinGW/non-MSVC Windows: signal(SIGSEGV) + setjmp/longjmp  (MSVCRT translates
//                           EXCEPTION_ACCESS_VIOLATION → SIGSEGV)
//   POSIX:         fork() child process  (full address-space isolation)
// ---------------------------------------------------------------------------
#if defined(_WIN32) && defined(_MSC_VER)

inline InvalidHandleProbe probe_invalid_handle(SQLSMALLINT handle_type, SQLHANDLE handle) {
  InvalidHandleProbe probe;
  __try {
    probe.returnCode = SQLFreeHandle(handle_type, handle);
  } __except (GetExceptionCode() == EXCEPTION_ACCESS_VIOLATION ? EXCEPTION_EXECUTE_HANDLER
                                                               : EXCEPTION_CONTINUE_SEARCH) {
    probe.crashed = true;
  }
  return probe;
}

#elif defined(_WIN32)

inline InvalidHandleProbe probe_invalid_handle(SQLSMALLINT handle_type, SQLHANDLE handle) {
  static thread_local std::jmp_buf rih_jmp_buf;
  auto prev_handler = std::signal(SIGSEGV, [](int) { std::longjmp(rih_jmp_buf, 1); });

  InvalidHandleProbe probe;
  if (setjmp(rih_jmp_buf) == 0) {
    probe.returnCode = SQLFreeHandle(handle_type, handle);
  } else {
    probe.crashed = true;
  }

  std::signal(SIGSEGV, prev_handler);
  return probe;
}

#else

inline InvalidHandleProbe probe_invalid_handle(SQLSMALLINT handle_type, SQLHANDLE handle) {
  pid_t pid = fork();
  if (pid == -1) {
    FAIL("fork() failed: " << std::strerror(errno));
  }
  if (pid == 0) {
    SQLRETURN r = SQLFreeHandle(handle_type, handle);
    _exit(r == SQL_INVALID_HANDLE ? 0 : 1);
  }

  int status = 0;
  while (waitpid(pid, &status, 0) == -1) {
    if (errno != EINTR) {
      FAIL("waitpid() failed: " << std::strerror(errno));
    }
  }

  InvalidHandleProbe probe;
  probe.crashed = WIFSIGNALED(status) && (WTERMSIG(status) == SIGSEGV || WTERMSIG(status) == SIGBUS);
  if (WIFEXITED(status) && WEXITSTATUS(status) == 0) {
    probe.returnCode = SQL_INVALID_HANDLE;
  } else if (!probe.crashed) {
    probe.returnCode = SQL_SUCCESS;
  }
  return probe;
}

#endif

namespace OdbcMatchers {

// Matches SQL_SUCCESS or SQL_SUCCESS_WITH_INFO.
class Succeeded : public Catch::Matchers::MatcherBase<OdbcResult> {
 public:
  bool match(const OdbcResult& result) const override {
    return result.returnCode == SQL_SUCCESS || result.returnCode == SQL_SUCCESS_WITH_INFO;
  }
  std::string describe() const override { return "is SQL_SUCCESS or SQL_SUCCESS_WITH_INFO"; }
};

// Matches exactly SQL_SUCCESS (no info).
class IsSuccess : public Catch::Matchers::MatcherBase<OdbcResult> {
 public:
  bool match(const OdbcResult& result) const override { return result.returnCode == SQL_SUCCESS; }
  std::string describe() const override { return "is SQL_SUCCESS"; }
};

// Matches exactly SQL_SUCCESS_WITH_INFO.
class IsSuccessWithInfo : public Catch::Matchers::MatcherBase<OdbcResult> {
 public:
  bool match(const OdbcResult& result) const override { return result.returnCode == SQL_SUCCESS_WITH_INFO; }
  std::string describe() const override { return "is SQL_SUCCESS_WITH_INFO"; }
};

// Matches SQL_ERROR.
class IsError : public Catch::Matchers::MatcherBase<OdbcResult> {
 public:
  bool match(const OdbcResult& result) const override { return result.returnCode == SQL_ERROR; }
  std::string describe() const override { return "is SQL_ERROR"; }
};

// Matches SQL_NO_DATA.
class IsNoData : public Catch::Matchers::MatcherBase<OdbcResult> {
 public:
  bool match(const OdbcResult& result) const override { return result.returnCode == SQL_NO_DATA; }
  std::string describe() const override { return "is SQL_NO_DATA"; }
};

// Matches SQL_INVALID_HANDLE.
class IsInvalidHandle : public Catch::Matchers::MatcherBase<OdbcResult> {
 public:
  bool match(const OdbcResult& result) const override { return result.returnCode == SQL_INVALID_HANDLE; }
  std::string describe() const override { return "is SQL_INVALID_HANDLE"; }
};

// Matches SQL_NEED_DATA.
class IsNeedData : public Catch::Matchers::MatcherBase<OdbcResult> {
 public:
  bool match(const OdbcResult& result) const override { return result.returnCode == SQL_NEED_DATA; }
  std::string describe() const override { return "is SQL_NEED_DATA"; }
};

// Matches SQL_STILL_EXECUTING.
class IsStillExecuting : public Catch::Matchers::MatcherBase<OdbcResult> {
 public:
  bool match(const OdbcResult& result) const override { return result.returnCode == SQL_STILL_EXECUTING; }
  std::string describe() const override { return "is SQL_STILL_EXECUTING"; }
};

// Matches when any diagnostic record has the given SQLSTATE.
class HasSqlState : public Catch::Matchers::MatcherBase<OdbcResult> {
  std::string expectedState_;

 public:
  explicit HasSqlState(std::string state) : expectedState_(std::move(state)) {}

  bool match(const OdbcResult& result) const override {
    for (const auto& rec : result.diagRecords) {
      if (rec.sqlState == expectedState_) return true;
    }
    return false;
  }
  std::string describe() const override { return "has SQLSTATE " + expectedState_; }
};

// Matches when any diagnostic message contains the given substring.
class HasDiagMessage : public Catch::Matchers::MatcherBase<OdbcResult> {
  std::string substring_;

 public:
  explicit HasDiagMessage(std::string substr) : substring_(std::move(substr)) {}

  bool match(const OdbcResult& result) const override {
    for (const auto& rec : result.diagRecords) {
      if (rec.messageText.find(substring_) != std::string::npos) return true;
    }
    return false;
  }
  std::string describe() const override { return "has diagnostic message containing \"" + substring_ + "\""; }
};

// Matches when a handle probe shows the handle is invalid (either the DM
// returned SQL_INVALID_HANDLE or the call crashed with an access violation).
class IsHandleInvalid : public Catch::Matchers::MatcherBase<InvalidHandleProbe> {
 public:
  bool match(const InvalidHandleProbe& probe) const override {
    return probe.crashed || probe.returnCode == SQL_INVALID_HANDLE;
  }
  std::string describe() const override { return "handle is invalid (SQL_INVALID_HANDLE or access violation)"; }
};

}  // namespace OdbcMatchers

// ---------------------------------------------------------------------------
// Convenience macros — thin wrappers around REQUIRE_THAT with OdbcMatchers.
// On failure Catch2 prints the ODBC return code and all diagnostic records.
// ---------------------------------------------------------------------------

// Requires SQL_SUCCESS or SQL_SUCCESS_WITH_INFO.
#define REQUIRE_ODBC(ret, handle) REQUIRE_THAT(OdbcResult(ret, handle), OdbcMatchers::Succeeded())

// Requires exactly SQL_SUCCESS.
#define REQUIRE_ODBC_SUCCESS(ret, handle) REQUIRE_THAT(OdbcResult(ret, handle), OdbcMatchers::IsSuccess())

// Requires exactly SQL_SUCCESS_WITH_INFO.
#define REQUIRE_ODBC_SUCCESS_WITH_INFO(ret, handle) \
  REQUIRE_THAT(OdbcResult(ret, handle), OdbcMatchers::IsSuccessWithInfo())

// Requires SQL_ERROR.
#define REQUIRE_ODBC_ERROR(ret, handle) REQUIRE_THAT(OdbcResult(ret, handle), OdbcMatchers::IsError())

// Requires SQL_NO_DATA.
#define REQUIRE_ODBC_NO_DATA(ret, handle) REQUIRE_THAT(OdbcResult(ret, handle), OdbcMatchers::IsNoData())

// Requires SQL_INVALID_HANDLE.
#define REQUIRE_ODBC_INVALID_HANDLE(ret, handle) REQUIRE_THAT(OdbcResult(ret, handle), OdbcMatchers::IsInvalidHandle())

// Requires SQL_ERROR with the given SQLSTATE.
#define REQUIRE_EXPECTED_ERROR(ret, expectedState, handle, handleType) \
  REQUIRE_THAT(OdbcResult(ret, handleType, handle), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState(expectedState))

// Requires SQL_SUCCESS_WITH_INFO with the given SQLSTATE.
#define REQUIRE_EXPECTED_WARNING(ret, expectedState, handle, handleType) \
  REQUIRE_THAT(OdbcResult(ret, handleType, handle),                      \
               OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState(expectedState))

// Asserts that an ODBC handle has been invalidated (crash or SQL_INVALID_HANDLE).
#define REQUIRE_INVALID_HANDLE(handle_type, handle) \
  REQUIRE_THAT(probe_invalid_handle(handle_type, handle), OdbcMatchers::IsHandleInvalid())

#endif  // ODBC_MATCHERS_HPP
