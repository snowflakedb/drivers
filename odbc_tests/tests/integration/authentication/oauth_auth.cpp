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
// (analysis_feature_oauth.md §3 / §4 / §6):
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
// the e2e suite (substep 6) gated behind a real IdP.
//
// What we DO cover here:
//   * Missing-required-parameter diagnostics for AC, CC, and legacy
//     OAUTH (sf_core surfaces these synchronously, before any browser
//     launch or network call).
//   * Case-insensitive AUTHENTICATOR matching for OAuth flow names.
//   * Invalid AUTHENTICATOR rejection.
//   * Legacy AUTHENTICATOR=OAUTH with a TOKEN forwards the token to
//     sf_core (the localhost backend then fails to connect, but no
//     missing-parameter diagnostic is raised for the token).
//
// Gherkin scenarios for these test cases live in
// tests/definitions/shared/authentication/oauth.feature (added by
// substep 6 of this stack).

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
  // Connection failure is expected: no live Snowflake instance is
  // reachable here. Fail loudly only if the driver itself is broken.
  REQUIRE(ret == SQL_ERROR);

  auto records = get_diag_rec(dbc);
  for (const auto& record : records) {
    CHECK_THAT(record.messageText, !ContainsSubstring("Can't open lib"));
    CHECK_THAT(record.messageText, !ContainsSubstring("Data source name not found and no default driver specified"));
  }

  return ret;
}

bool diag_contains(const std::vector<DiagRec>& records, const std::string& needle) {
  return std::any_of(records.begin(), records.end(),
                     [&needle](const auto& r) { return ContainsSubstring(needle).match(r.messageText); });
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
// environments. AC happy-path coverage lives in the e2e suite
// (substep 6) gated behind a real IdP, and the BROWSER_LAUNCH_DISABLED
// kill switch is exercised by sf_core's own integration tests
// (sf_core/tests/integration/authentication/oauth.rs).

// =============================================================================
// OAuth Client Credentials (CC) flow -- validation only (token exchange would
// hit the IdP; that path is exercised in the e2e suite).
// =============================================================================

// Gherkin: Scenario: should fail OAUTH_CLIENT_CREDENTIALS when client_id is missing
TEST_CASE("should fail OAUTH_CLIENT_CREDENTIALS when client_id is missing", "[oauth_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests OAuth CC required-param validation");

  // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with the
  //       client_id field omitted -- per analysis §4, all three of
  //       client_id / client_secret / oauth_token_request_url are
  //       required for CC because Snowflake's GS does not mint CC
  //       tokens.
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  std::stringstream ss;
  ss << get_oauth_base_connection_string("OAUTH_CLIENT_CREDENTIALS");
  ss << "OAUTH_CLIENT_SECRET=test-client-secret;";
  ss << "OAUTH_TOKEN_REQUEST_URL=https://idp.example.com/oauth/token;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);

  // Then Connection fails with a missing-parameter error citing
  //      oauth_client_id.
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  CHECK(diag_contains_missing(records, "oauth_client_id"));
}

// Gherkin: Scenario: should fail OAUTH_CLIENT_CREDENTIALS when client_secret is missing
TEST_CASE("should fail OAUTH_CLIENT_CREDENTIALS when client_secret is missing", "[oauth_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests OAuth CC required-param validation");

  // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without a
  //       client_secret.
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  std::stringstream ss;
  ss << get_oauth_base_connection_string("OAUTH_CLIENT_CREDENTIALS");
  ss << "OAUTH_CLIENT_ID=test-client-id;";
  ss << "OAUTH_TOKEN_REQUEST_URL=https://idp.example.com/oauth/token;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);

  // Then Connection fails with a missing-parameter error citing
  //      oauth_client_secret.
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  CHECK(diag_contains_missing(records, "oauth_client_secret"));
}

// Gherkin: Scenario: should fail OAUTH_CLIENT_CREDENTIALS when token_request_url is missing
TEST_CASE("should fail OAUTH_CLIENT_CREDENTIALS when token_request_url is missing", "[oauth_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests OAuth CC required-param validation");

  // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without
  //       oauth_token_request_url -- mandatory because Snowflake does
  //       not host a CC token endpoint.
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  std::stringstream ss;
  ss << get_oauth_base_connection_string("OAUTH_CLIENT_CREDENTIALS");
  ss << "OAUTH_CLIENT_ID=test-client-id;";
  ss << "OAUTH_CLIENT_SECRET=test-client-secret;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);

  // Then Connection fails with a missing-parameter error citing
  //      oauth_token_request_url.
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  CHECK(diag_contains_missing(records, "oauth_token_request_url"));
}

// =============================================================================
// Legacy AUTHENTICATOR=OAUTH (pre-acquired access token)
// =============================================================================

// Gherkin: Scenario: should forward AUTHENTICATOR=OAUTH with TOKEN to core
TEST_CASE("should forward AUTHENTICATOR=OAUTH with TOKEN to core", "[oauth_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests legacy OAuth token forwarding");

  // Given Authentication is set to legacy OAUTH with a pre-acquired
  //       access token (analysis §6 / §10.1). This path does NOT spawn
  //       a browser or perform an IdP exchange -- the wrapper hands the
  //       token straight to the Snowflake login request.
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  std::stringstream ss;
  ss << get_oauth_base_connection_string("OAUTH");
  ss << "TOKEN=fake.jwt.token;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);

  // Then Connection reaches sf_core without a missing-parameter
  //      error for the token.
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  CHECK_FALSE(diag_contains_missing(records, "token"));
}

// Gherkin: Scenario: should fail AUTHENTICATOR=OAUTH when TOKEN is missing
TEST_CASE("should fail AUTHENTICATOR=OAUTH when TOKEN is missing", "[oauth_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests legacy OAuth required-param validation");

  // Given Authentication is set to legacy OAUTH without a TOKEN.
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  std::string connection_string = get_oauth_base_connection_string("OAUTH");

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);

  // Then Connection fails with a missing-parameter error citing the
  //      token (analysis §6: legacy OAUTH requires `token=`).
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  CHECK(diag_contains_missing(records, "token"));
}

// Gherkin: Scenario: should accept lowercase oauth authenticator value (legacy)
TEST_CASE("should accept lowercase oauth authenticator value", "[oauth_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests case-insensitive AUTHENTICATOR matching for legacy OAUTH");

  // Given Authentication is set to lowercase oauth (legacy) with a
  //       TOKEN. analysis §9 -- case-insensitive matching of OAuth
  //       flow names.
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  std::stringstream ss;
  ss << get_oauth_base_connection_string("oauth");
  ss << "TOKEN=fake.jwt.token;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);

  // Then The wrapper does not reject the AUTHENTICATOR value as
  //      unknown.
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  for (const auto& record : records) {
    CHECK_THAT(record.messageText, !ContainsSubstring("Invalid authenticator"));
    CHECK_THAT(record.messageText, !ContainsSubstring("Unknown authenticator"));
  }
}

// =============================================================================
// Negative path: invalid authenticator value
// =============================================================================

// Gherkin: Scenario: should fail when AUTHENTICATOR is set to an unknown OAuth-like value
TEST_CASE("should fail when AUTHENTICATOR is an unknown OAuth-like value", "[oauth_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests unknown-authenticator validation");

  // Given Authentication is set to a typo of an OAuth flow name
  //       (e.g. OAUTH_AUTHORIZATION_TYPO).
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  std::stringstream ss;
  ss << get_oauth_base_connection_string("OAUTH_AUTHORIZATION_TYPO");
  ss << "OAUTH_CLIENT_ID=test-client-id;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);

  // Then Connection fails with an authenticator-related error.
  //      sf_core's exact wording may evolve; we only assert that some
  //      diagnostic mentions the bogus authenticator value or flags it
  //      as invalid/unknown.
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  bool found = diag_contains(records, "Invalid authenticator") || diag_contains(records, "Unknown authenticator") ||
               diag_contains(records, "OAUTH_AUTHORIZATION_TYPO") || diag_contains(records, "authenticator");
  CHECK(found);
}

// =============================================================================
// Secret redaction in driver logs
// =============================================================================

// Gherkin: Scenario: should not echo OAUTH_CLIENT_SECRET back in any diagnostic record
TEST_CASE("should not echo OAUTH_CLIENT_SECRET in diagnostics", "[oauth_auth]") {
  SKIP_OLD_DRIVER("", "New-driver-only: secret redaction in OAuth diagnostics");

  // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a
  //       distinctive client secret literal.
  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);
  const std::string secret_literal = "ZZ_SECRET_NEEDLE_OAUTH_CC_ZZ";
  std::stringstream ss;
  ss << get_oauth_base_connection_string("OAUTH_CLIENT_CREDENTIALS");
  ss << "OAUTH_CLIENT_ID=test-client-id;";
  ss << "OAUTH_CLIENT_SECRET=" << secret_literal << ";";
  // Omit the token URL so the flow fails fast on validation.
  std::string connection_string = ss.str();

  // When Trying to Connect
  SQLRETURN ret = attempt_oauth_connection(dbc, connection_string);
  REQUIRE(ret == SQL_ERROR);

  // Then No diagnostic record contains the literal secret.
  auto records = get_diag_rec(dbc);
  for (const auto& record : records) {
    CHECK_THAT(record.messageText, !ContainsSubstring(secret_literal));
  }
}
