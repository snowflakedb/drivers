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

// OAuth integration tests for the ODBC wrapper.
//
// Scope: connection-string parsing, parameter normalisation, and
// required-parameter validation paths for the three OAuth flows
// (AC, CC, legacy pre-acquired access token):
//
//   * AUTHENTICATOR=OAUTH                     -- legacy pre-acquired access token
//   * AUTHENTICATOR=OAUTH_AUTHORIZATION_CODE  -- AC + PKCE
//   * AUTHENTICATOR=OAUTH_CLIENT_CREDENTIALS  -- CC (external IdP only)
//
// These tests intentionally do NOT exercise "happy path" forwarding
// for AC or CC. The AC flow opens an OS browser as soon as
// configuration is valid, and the CC flow performs an HTTP token
// exchange against the configured IdP. Neither is appropriate for an
// integration test that must run offline. Happy-path coverage lives in
// the e2e suite (oauth.cpp) gated behind a real IdP, and sf_core's own
// integration tests (sf_core/tests/integration/authentication/oauth.rs)
// exercise the AC flow against a wiremock IdP -- the browser leg is
// suppressed there by the cfg-gated `browser_launcher` default that
// `OAuthAuthorizationCodeConfig::from_settings` installs under
// `cfg(any(test, feature = "test-utils"))`, so no real OS browser is
// ever launched in those test builds.
//
// What we DO cover here, mapped to scenarios in
// tests/definitions/shared/authentication/oauth.feature:
//
//   * @odbc_int legacy AUTHENTICATOR=OAUTH (token forwarding + missing token)
//   * @odbc_int OAUTH_CLIENT_SECRET literal redaction in diagnostic records

using Catch::Matchers::ContainsSubstring;

namespace {

std::string get_oauth_base_connection_string(const std::string& authenticator) {
  std::stringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "ACCOUNT=test_account;";
  ss << "UID=test_user;";
  ss << "PORT=8090;";
  ss << "AUTHENTICATOR=" << authenticator << ";";
  return ss.str();
}

EnvironmentHandleWrapper setup_oauth_environment() {
  ensure_driver_installed();
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_oauth_connection_handle(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

SQLRETURN attempt_oauth_connection(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);

  auto records = get_diag_rec(dbc);
  for (const auto& record : records) {
    CHECK_THAT(record.messageText, !ContainsSubstring("Can't open lib"));
    CHECK_THAT(record.messageText, !ContainsSubstring("Data source name not found and no default driver specified"));
  }

  return ret;
}

bool diag_contains_missing(const std::vector<DiagRec>& records, const std::string& name) {
  return std::any_of(records.begin(), records.end(), [&name](const auto& r) {
    return (ContainsSubstring("Missing required parameter") && ContainsSubstring(name)).match(r.messageText);
  });
}

}  // anonymous namespace

// =============================================================================
// OAuth Authorization Code (AC) flow
// =============================================================================
//
// We do NOT add validation tests for AC here. As soon as a complete
// AC configuration is supplied, sf_core spawns the loopback listener
// and launches the OS browser -- both unsafe in CI / offline test
// environments. AC happy-path coverage lives in the e2e suite (oauth.cpp)
// gated behind a real IdP, and sf_core's own integration tests
// (sf_core/tests/integration/authentication/oauth.rs) drive the AC flow
// against a wiremock IdP without ever launching a browser (the cfg-gated
// `browser_launcher` default installed by
// `OAuthAuthorizationCodeConfig::from_settings` under
// `cfg(any(test, feature = "test-utils"))` is a no-op).

// =============================================================================
// Legacy AUTHENTICATOR=OAUTH (pre-acquired access token)
// =============================================================================

TEST_CASE("should forward AUTHENTICATOR=OAUTH with TOKEN to core", "[oauth_int]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests legacy OAuth token forwarding");

  // Given Authentication is set to legacy OAUTH with a pre-acquired access token
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  std::stringstream ss;
  ss << get_oauth_base_connection_string("OAUTH");
  ss << "TOKEN=fake.jwt.token;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);

  // Then The wrapper forwards the token to sf_core without raising a missing-parameter error for it
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  CHECK_FALSE(diag_contains_missing(records, "token"));
}

TEST_CASE("should fail AUTHENTICATOR=OAUTH when TOKEN is missing", "[oauth_int]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests legacy OAuth required-param validation");

  // Given Authentication is set to legacy OAUTH without a TOKEN
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  std::string connection_string = get_oauth_base_connection_string("OAUTH");

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);

  // Then Connection fails with a missing-parameter error citing token
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  CHECK(diag_contains_missing(records, "token"));
}

// =============================================================================
// Secret redaction in driver logs
// =============================================================================

TEST_CASE("should not echo OAUTH_CLIENT_SECRET in diagnostics", "[oauth_int]") {
  SKIP_OLD_DRIVER("", "New-driver-only: secret redaction in OAuth diagnostics");

  // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a distinctive client secret literal
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  const std::string secret_literal = "ZZ_SECRET_NEEDLE_OAUTH_CC_ZZ";
  std::stringstream ss;
  ss << get_oauth_base_connection_string("OAUTH_CLIENT_CREDENTIALS");
  ss << "OAUTH_CLIENT_ID=test-client-id;";
  ss << "OAUTH_CLIENT_SECRET=" << secret_literal << ";";
  std::string connection_string = ss.str();

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);
  REQUIRE(ret == SQL_ERROR);

  // Then No diagnostic record contains the literal client secret
  auto records = get_diag_rec(dbc);
  for (const auto& record : records) {
    CHECK_THAT(record.messageText, !ContainsSubstring(secret_literal));
  }
}
