#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <algorithm>
#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

using Catch::Matchers::ContainsSubstring;

// =============================================================================
// Helpers
// =============================================================================

std::string get_mfa_base_connection_string(bool passcodeInPassword) {
  std::stringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "ACCOUNT=test_account;";
  ss << "UID=test_user;";
  ss << "PWD=test_password" << (passcodeInPassword ? "123456;" : ";");
  ss << "PORT=8090;";
  ss << "AUTHENTICATOR=USERNAME_PASSWORD_MFA;";
  return ss.str();
}

std::string get_mfa_connection_string_without_password() {
  std::stringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "ACCOUNT=test_account;";
  ss << "UID=test_user;";
  ss << "PORT=8090;";
  ss << "AUTHENTICATOR=USERNAME_PASSWORD_MFA;";
  return ss.str();
}

EnvironmentHandleWrapper setup_mfa_environment() {
  ensure_driver_installed();
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_mfa_connection_handle(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

SQLRETURN attempt_mfa_connection(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
  // connection failure is expected as the test is not E2E test
  REQUIRE(ret == SQL_ERROR);

  // however driver/environment setup error is unwanted
  auto records = get_diag_rec(dbc);
  using Catch::Matchers::ContainsSubstring;
  for (const auto& record : records) {
    CHECK_THAT(record.messageText, !ContainsSubstring("Can't open lib"));
    CHECK_THAT(record.messageText, !ContainsSubstring("Data source name not found and no default driver specified"));
  }

  return ret;
}

// =============================================================================
// MFA DUO push flow
// =============================================================================

TEST_CASE("should forward USERNAME_PASSWORD_MFA parameters to core", "[mfa_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests MFA parameter forwarding");

  // Given Authentication is set to USERNAME_PASSWORD_MFA with user and password
  auto env = setup_mfa_environment();
  auto dbc = get_mfa_connection_handle(env);
  std::string connection_string = get_mfa_base_connection_string(false);

  // When Trying to Connect
  SQLRETURN ret = attempt_mfa_connection(dbc, connection_string);

  // Then Connection reaches sf_core without a missing-parameter error for MFA fields
  if (ret == SQL_ERROR) {
    auto records = get_diag_rec(dbc);
    for (const auto& record : records) {
      CHECK_THAT(record.messageText, !ContainsSubstring("Missing required parameter"));
    }
  }
}

// =============================================================================
// MFA TOTP passcode flow
// =============================================================================

TEST_CASE("should forward PASSCODE parameter to core", "[mfa_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests MFA parameter forwarding");

  // Given Authentication is set to USERNAME_PASSWORD_MFA with a TOTP passcode
  auto env = setup_mfa_environment();
  auto dbc = get_mfa_connection_handle(env);
  std::string connection_string = get_mfa_base_connection_string(false);
  connection_string += "PASSCODE=123456;";

  // When Trying to Connect
  SQLRETURN ret = attempt_mfa_connection(dbc, connection_string);

  // Then Connection reaches sf_core without a missing-parameter error
  if (ret == SQL_ERROR) {
    auto records = get_diag_rec(dbc);
    for (const auto& record : records) {
      CHECK_THAT(record.messageText, !ContainsSubstring("Missing required parameter"));
    }
  }
}

// =============================================================================
// MFA passcode-in-password flow
// =============================================================================

TEST_CASE("should forward PASSCODEINPASSWORD parameter to core", "[mfa_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests MFA parameter forwarding");

  // Given Authentication is set to USERNAME_PASSWORD_MFA with passcode appended to password
  auto env = setup_mfa_environment();
  auto dbc = get_mfa_connection_handle(env);
  std::string connection_string = get_mfa_base_connection_string(true);
  connection_string += "PASSCODEINPASSWORD=true;";

  // When Trying to Connect
  SQLRETURN ret = attempt_mfa_connection(dbc, connection_string);

  // Then Connection reaches sf_core without a missing-parameter error
  if (ret == SQL_ERROR) {
    auto records = get_diag_rec(dbc);
    for (const auto& record : records) {
      CHECK_THAT(record.messageText, !ContainsSubstring("Missing required parameter"));
    }
  }
}

// =============================================================================
// Missing password error
// =============================================================================

TEST_CASE("should fail MFA authentication when password is not provided", "[mfa_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests MFA parameter validation");

  // Given Authentication is set to USERNAME_PASSWORD_MFA but password is omitted
  auto env = setup_mfa_environment();
  auto dbc = get_mfa_connection_handle(env);
  std::string connection_string = get_mfa_connection_string_without_password();

  // When Trying to Connect
  SQLRETURN ret = attempt_mfa_connection(dbc, connection_string);

  // Then Connection fails with a missing-parameter error
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  bool found_password_error = std::any_of(records.begin(), records.end(), [](const auto& r) {
    return (ContainsSubstring("Missing required parameter") && ContainsSubstring("password")).match(r.messageText);
  });
  CHECK(found_password_error);
}

// =============================================================================
// CLIENT_STORE_TEMPORARY_CREDENTIAL forwarding
// =============================================================================

TEST_CASE("should forward CLIENT_STORE_TEMPORARY_CREDENTIAL parameter to core", "[mfa_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests MFA parameter forwarding");

  // Given Authentication is set to USERNAME_PASSWORD_MFA with token caching enabled
  auto env = setup_mfa_environment();
  auto dbc = get_mfa_connection_handle(env);
  std::string connection_string = get_mfa_base_connection_string(false);
  connection_string += "CLIENT_STORE_TEMPORARY_CREDENTIAL=true;";

  // When Trying to Connect
  SQLRETURN ret = attempt_mfa_connection(dbc, connection_string);

  // Then Connection reaches sf_core without a missing-parameter error
  if (ret == SQL_ERROR) {
    auto records = get_diag_rec(dbc);
    for (const auto& record : records) {
      CHECK_THAT(record.messageText, !ContainsSubstring("Missing required parameter"));
    }
  }
}
