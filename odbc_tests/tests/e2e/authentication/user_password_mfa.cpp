// Username/password MFA authentication E2E tests.
//
// Requires the snowdrivers-test-external-browser-universal-driver Docker container
// (/externalbrowser/totpGenerator.js generates TOTP passcodes for the MFA test user).
//
// Mirrors python/tests/e2e/authentication/test_user_password_mfa.py and the Gherkin
// scenarios in tests/definitions/shared/authentication/user_password_mfa.feature.
//
// Run locally (VPN required for preprod Snowflake access):
//   ./tests/auth/run_auth_browser_local.sh odbc

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>
#include <utility>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "mfa_auth_helpers.hpp"
#include "odbc_matchers.hpp"
#include "require.hpp"
#include "test_setup.hpp"

namespace {

void verify_simple_query_execution(ConnectionHandleWrapper& dbc) {
  StatementHandleWrapper stmt = dbc.createStatementHandle();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER result = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), nullptr);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 1);
}

// unixODBC reads ODBCSYSINI when the environment handle is allocated, so the driver
// must be registered before SQLAllocHandle(SQL_HANDLE_ENV).
EnvironmentHandleWrapper setup_mfa_environment() {
  ensure_driver_installed();
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

std::vector<std::pair<std::string, std::string>> mfa_token_cache_attrs() {
  // BD#16: old driver uses CLIENT_REQUEST_MFA_TOKEN; new driver uses
  // CLIENT_STORE_TEMPORARY_CREDENTIAL (with backward-compatible alias).
  OLD_DRIVER_ONLY("BD#16") { return {{"CLIENT_REQUEST_MFA_TOKEN", "true"}}; }
  NEW_DRIVER_ONLY("BD#16") { return {{"CLIENT_STORE_TEMPORARY_CREDENTIAL", "true"}}; }
  return {};
}

}  // namespace

// =============================================================================
// Passcode flow
// =============================================================================

TEST_CASE("should authenticate using username password and TOTP passcode", "[mfa_auth][requires_browser]") {
  REQUIRE_BROWSER("MFA E2E needs the headless browser container for TOTP generation");

  // Given Authentication is set to username_password_mfa and user, password and passcode are provided
  const auto creds = mfa_auth::load_mfa_credentials();
  const auto params = mfa_auth::get_mfa_test_parameters();
  auto env = setup_mfa_environment();

  // When Trying to Connect
  auto dbc = mfa_auth::connect_with_totp_retry(env, creds.totp_seed, creds.password, false, params);

  // Then Login is successful and simple query can be executed
  verify_simple_query_execution(dbc);
  SQLRETURN disconnect_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(disconnect_ret, dbc);
}

TEST_CASE("should authenticate using username password with appended TOTP passcode", "[mfa_auth][requires_browser]") {
  REQUIRE_BROWSER("MFA E2E needs the headless browser container for TOTP generation");

  // Given Authentication is set to username_password_mfa and user, password with appended passcode are provided and
  // passcodeInPassword is set
  const auto creds = mfa_auth::load_mfa_credentials();
  const auto params = mfa_auth::get_mfa_test_parameters();
  auto env = setup_mfa_environment();

  // When Trying to Connect
  auto dbc = mfa_auth::connect_with_totp_retry(env, creds.totp_seed, creds.password, true, params);

  // Then Login is successful and simple query can be executed
  verify_simple_query_execution(dbc);
  SQLRETURN disconnect_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(disconnect_ret, dbc);
}

// =============================================================================
// Token caching flow
// =============================================================================

TEST_CASE("should reuse cached MFA token without passcode", "[mfa_auth][requires_browser]") {
  REQUIRE_BROWSER("MFA E2E needs the headless browser container for TOTP generation");

  // Given Authentication is set to username_password_mfa and MFA token has been cached from a previous connection
  const auto creds = mfa_auth::load_mfa_credentials();
  const auto params = mfa_auth::get_mfa_test_parameters();
  const auto cache_attrs = mfa_token_cache_attrs();
  auto env = setup_mfa_environment();
  auto first = mfa_auth::connect_with_totp_retry(env, creds.totp_seed, creds.password, false, params, cache_attrs);
  verify_simple_query_execution(first);
  SQLRETURN first_disconnect_ret = SQLDisconnect(first.getHandle());
  REQUIRE_ODBC(first_disconnect_ret, first);

  const std::string cached_connection_string =
      mfa_auth::build_mfa_connection_string(params, creds.password, nullptr, false, cache_attrs);
  auto second = env.createConnectionHandle();

  // When Trying to Connect without passcode
  SQLRETURN ret = SQLDriverConnect(second.getHandle(), nullptr, (SQLCHAR*)cached_connection_string.c_str(), SQL_NTS,
                                   nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, second);

  // Then Login is successful and simple query can be executed
  verify_simple_query_execution(second);
  ret = SQLDisconnect(second.getHandle());
  REQUIRE_ODBC(ret, second);
}

// =============================================================================
// Error cases
// =============================================================================

TEST_CASE("should fail authentication when wrong password is provided", "[mfa_auth][requires_browser]") {
  SKIP("Disabled: bad-secret tests cause pipeline flakiness by blocking the test account");
  REQUIRE_BROWSER("MFA E2E needs the headless browser container for TOTP generation");

  // Given Authentication is set to username_password_mfa and user is provided but password is skipped or invalid
  const auto creds = mfa_auth::load_mfa_credentials();
  const auto params = mfa_auth::get_mfa_test_parameters();
  ensure_driver_installed();
  const std::string passcode = mfa_auth::acquire_totp_passcode(creds.totp_seed);
  const std::string connection_string =
      mfa_auth::build_mfa_connection_string(params, "wrong_password", &passcode, false);

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then There is error returned
  REQUIRE(records.size() >= 1);
  CHECK(records[0].sqlState == "28000");
}
