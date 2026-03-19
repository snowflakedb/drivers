#ifndef ODBC_MATCHERS_HPP
#define ODBC_MATCHERS_HPP

#include <sql.h>
#include <sqlext.h>

#include <string>
#include <vector>

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

  OdbcResult(SQLRETURN ret, const HandleWrapper& handle) : OdbcResult(ret, handle.getType(), handle.getHandle()) {}
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

#endif  // ODBC_MATCHERS_HPP
