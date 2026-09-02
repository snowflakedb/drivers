#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "oauth_auth_helpers.hpp"
#include "require.hpp"
#include "test_setup.hpp"

// End-to-end OAuth tests for the ODBC wrapper, mirroring the Python suite
// (python/tests/e2e/authentication/test_oauth.py and auth_helpers.py) and
// sf_core's `oauth_should_*` methods against the shared Gherkin scenarios in
// tests/definitions/shared/authentication/oauth.feature.
//
// Every test needs the headless-browser Docker container
// (snowdrivers-test-external-browser-universal-driver) -- for the Authorization
// Code browser leg and for preprod Okta IdP connectivity -- so all are gated
// behind REQUIRE_BROWSER. Rather than reading a pre-acquired token, the legacy
// OAUTH flow mints a fresh access token from Okta and the Authorization Code
// flow drives the container's provideBrowserCredentials.js. The required
// parameters.json keys are listed inline at each test's get_param_required call.

namespace {

EnvironmentHandleWrapper setup_oauth_environment() {
  // unixODBC reads ODBCSYSINI when the environment handle is allocated, so the driver
  // must be registered before SQLAllocHandle(SQL_HANDLE_ENV).
  ensure_driver_installed();
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

std::string get_oauth_base_connection_string(const picojson::object& params, const std::string& authenticator,
                                             const std::string& uid) {
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  ss << "UID=" << uid << ";";
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_WAREHOUSE", "WAREHOUSE");
  // ROLE required, else login fails with "No default role assigned" (390194).
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_ROLE", "ROLE");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_DATABASE", "DATABASE");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_SCHEMA", "SCHEMA");
  ss << "AUTHENTICATOR=" << authenticator << ";";
  return ss.str();
}

void verify_oauth_simple_query_execution(ConnectionHandleWrapper& dbc) {
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

}  // anonymous namespace

// =============================================================================
// Legacy AUTHENTICATOR=OAUTH (pre-acquired access token)
//
// A fresh OAuth access token is minted from the Okta IdP and passed via
// TOKEN; it is presented to Snowflake as-is.
// =============================================================================

TEST_CASE("oauth should authenticate with pre acquired access token", "[oauth_e2e][requires_browser]") {
  REQUIRE_BROWSER("OAuth E2E needs the headless browser container (preprod IdP connectivity)");

  auto params = get_test_parameters("testconnection");
  const std::string user = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_USER");
  const std::string access_token = oauth_auth::retrieve_oauth_access_token(
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL"),
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_ID"),
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_SECRET"), user,
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_PASSWORD"),
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_ROLE"));

  // Given Authentication is set to legacy OAUTH and a pre-acquired
  //       OAuth access token is supplied via `token=`
  std::stringstream ss;
  ss << get_oauth_base_connection_string(params, "OAUTH", user);
  ss << "TOKEN=" << access_token << ";";
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();
  auto dbc = env.createConnectionHandle();

  // When Trying to Connect
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, (SQLCHAR*)connection_string.c_str(), SQL_NTS, nullptr, 0,
                                   nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);

  // Then Login is successful and a simple query can be executed
  verify_oauth_simple_query_execution(dbc);

  SQLRETURN disconnect_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(disconnect_ret, dbc);
}

TEST_CASE("oauth should authenticate using lowercase oauth authenticator", "[oauth_e2e][requires_browser]") {
  REQUIRE_BROWSER("OAuth E2E needs the headless browser container (preprod IdP connectivity)");

  auto params = get_test_parameters("testconnection");
  const std::string user = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_USER");
  const std::string access_token = oauth_auth::retrieve_oauth_access_token(
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL"),
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_ID"),
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_SECRET"), user,
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_PASSWORD"),
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_ROLE"));

  // Given Authentication is set to lowercase oauth and a valid pre-acquired OAuth access token is supplied via TOKEN
  std::stringstream ss;
  ss << get_oauth_base_connection_string(params, "oauth", user);
  ss << "TOKEN=" << access_token << ";";
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();
  auto dbc = env.createConnectionHandle();

  // When Trying to Connect
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, (SQLCHAR*)connection_string.c_str(), SQL_NTS, nullptr, 0,
                                   nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);

  // Then Login is successful and a simple query can be executed
  verify_oauth_simple_query_execution(dbc);

  SQLRETURN disconnect_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(disconnect_ret, dbc);
}

TEST_CASE("oauth should fail legacy authentication with invalid token", "[oauth_e2e][requires_browser]") {
  SKIP("Disabled: bad-secret tests cause pipeline flakiness by blocking the test account");
  REQUIRE_BROWSER("OAuth E2E needs the headless browser container (preprod IdP connectivity)");

  auto params = get_test_parameters("testconnection");
  const std::string user = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_USER");

  // Given Authentication is set to legacy OAUTH and an invalid
  //       OAuth access token is supplied
  std::stringstream ss;
  ss << get_oauth_base_connection_string(params, "OAUTH", user);
  ss << "TOKEN=invalid_oauth_token_12345;";
  std::string connection_string = ss.str();

  ensure_driver_installed();

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then Connection fails with an authentication / login error
  REQUIRE(records.size() >= 1);
  CHECK(records[0].sqlState == "28000");
}

// =============================================================================
// OAuth Authorization Code (AC) flow
//
// An interactive, user-based flow that authenticates a real user through a
// browser login leg. The connect thread spawns Chromium via the OS browser
// opener; the browser thread drives the Snowflake IdP login over Chromium's
// remote-debugging port.
// =============================================================================

TEST_CASE("oauth should authenticate using authorization code flow", "[oauth_e2e][requires_browser]") {
  REQUIRE_BROWSER("Authorization Code happy path needs the headless Chromium container");

  auto params = get_test_parameters("testconnection");
  const std::string user = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_USER");
  const std::string password = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_PASSWORD");
  const std::string client_id = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_ID");
  const std::string client_secret =
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_SECRET");
  const std::string redirect_uri =
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_REDIRECT_URI");
  const std::string totp_seed = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_MFA_SEED");

  // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id / secret. `oauth_authorization_url`
  // and `oauth_token_request_url` are forwarded from parameters when present (otherwise the driver falls back to the
  // Snowflake-IdP defaults `https://{host}/oauth/authorize` and `https://{host}/oauth/token-request`).
  // `client_store_temporary_credential=true` lets the AC flow short-circuit on subsequent runs by re-using the cached
  // access / refresh token (AC state machine: cache → refresh → interactive).
  std::stringstream ss;
  ss << get_oauth_base_connection_string(params, "OAUTH_AUTHORIZATION_CODE", user);
  ss << "OAUTH_CLIENT_ID=" << client_id << ";";
  ss << "OAUTH_CLIENT_SECRET=" << client_secret << ";";
  ss << "OAUTH_REDIRECT_URI=" << redirect_uri << ";";
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();
  auto dbc = env.createConnectionHandle();

  oauth_auth::clean_browser_processes();

  // When Trying to Connect (this will spawn the local-loopback HTTP listener and `xdg-open`/`open`/`ShellExecute` the
  // IdP login URL unless a previously cached access token short-circuits the leg)
  SQLRETURN ret = oauth_auth::connect_with_browser_automation(dbc, connection_string, "internalOauthSnowflakeSuccess",
                                                              user, password, totp_seed);
  oauth_auth::clean_browser_processes();

  // Then Login is successful and a simple query can be executed
  REQUIRE_ODBC(ret, dbc);
  verify_oauth_simple_query_execution(dbc);

  SQLRETURN disconnect_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(disconnect_ret, dbc);
}

TEST_CASE("oauth should reuse cached access token without browser interaction", "[oauth_e2e][requires_browser]") {
  REQUIRE_BROWSER("Authorization Code token caching needs the headless Chromium container");

  auto params = get_test_parameters("testconnection");
  const std::string user = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_USER");
  const std::string password = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_PASSWORD");
  const std::string client_id = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_ID");
  const std::string client_secret =
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_SECRET");
  const std::string redirect_uri =
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_REDIRECT_URI");
  const std::string totp_seed = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_MFA_SEED");

  // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with client_store_temporary_credential=true
  //       and a token has been cached from a previous browser authentication
  std::stringstream ss;
  ss << get_oauth_base_connection_string(params, "OAUTH_AUTHORIZATION_CODE", user);
  ss << "OAUTH_CLIENT_ID=" << client_id << ";";
  ss << "OAUTH_CLIENT_SECRET=" << client_secret << ";";
  ss << "OAUTH_REDIRECT_URI=" << redirect_uri << ";";
  ss << "CLIENT_STORE_TEMPORARY_CREDENTIAL=true;";
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();

  oauth_auth::clean_browser_processes();
  {
    auto first = env.createConnectionHandle();
    SQLRETURN ret = oauth_auth::connect_with_browser_automation(
        first, connection_string, "internalOauthSnowflakeSuccess", user, password, totp_seed);
    oauth_auth::clean_browser_processes();
    REQUIRE_ODBC(ret, first);
    verify_oauth_simple_query_execution(first);

    ret = SQLDisconnect(first.getHandle());
    REQUIRE_ODBC(ret, first);
  }

  // When Trying to Connect without browser interaction
  auto second = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(second.getHandle(), nullptr, (SQLCHAR*)connection_string.c_str(), SQL_NTS, nullptr,
                                   0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, second);

  // Then Login is successful and a simple query can be executed
  verify_oauth_simple_query_execution(second);

  ret = SQLDisconnect(second.getHandle());
  REQUIRE_ODBC(ret, second);
}

TEST_CASE("oauth should fail authorization code flow with bad client secret", "[oauth_e2e][requires_browser]") {
  SKIP("Disabled: bad-secret tests cause pipeline flakiness by blocking the test account");
  REQUIRE_BROWSER("Authorization Code negative path needs the headless Chromium container");

  auto params = get_test_parameters("testconnection");
  const std::string user = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_USER");
  const std::string password = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_PASSWORD");
  const std::string client_id = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_ID");
  const std::string redirect_uri =
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_REDIRECT_URI");
  const std::string totp_seed = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_MFA_SEED");

  // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a
  //       valid client id but a deliberately invalid client secret.
  //       The IdP token-exchange step must reject the credentials
  //       and the driver must surface an authentication / login error.
  std::stringstream ss;
  ss << get_oauth_base_connection_string(params, "OAUTH_AUTHORIZATION_CODE", user);
  ss << "OAUTH_CLIENT_ID=" << client_id << ";";
  ss << "OAUTH_CLIENT_SECRET=invalid_client_secret_12345;";  // pragma: allowlist secret
  ss << "OAUTH_REDIRECT_URI=" << redirect_uri << ";";
  ss << "CLIENT_STORE_TEMPORARY_CREDENTIAL=false;";
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();
  auto dbc = env.createConnectionHandle();

  oauth_auth::clean_browser_processes();

  // When Trying to Connect
  SQLRETURN ret = oauth_auth::connect_with_browser_automation(dbc, connection_string, "internalOauthSnowflakeSuccess",
                                                              user, password, totp_seed);
  oauth_auth::clean_browser_processes();

  // Then Connection fails with an authentication / login error
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(records.size() >= 1);
  // BD#83: the new driver maps an IdP token-exchange rejection to SQLSTATE 28000
  // (invalid authorization specification); the legacy driver surfaces it as the
  // generic HY000.
  OLD_DRIVER_ONLY("BD#83") { CHECK(records[0].sqlState == "HY000"); }
  NEW_DRIVER_ONLY("BD#83") { CHECK(records[0].sqlState == "28000"); }
}

// =============================================================================
// OAuth Client Credentials (CC) flow
//
// A non-interactive, machine-to-machine flow where an external IdP mints the
// token from a client id / secret. Snowflake's GS does not mint CC tokens, so
// OAUTH_TOKEN_REQUEST_URL is required up-front.
// =============================================================================

TEST_CASE("oauth should authenticate using client credentials flow", "[oauth_e2e][requires_browser]") {
  REQUIRE_BROWSER("OAuth E2E needs the headless browser container (preprod IdP connectivity)");

  auto params = get_test_parameters("testconnection");
  const std::string token_url = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL");
  const std::string client_id = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_ID");
  const std::string client_secret =
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_SECRET");

  // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and an external IdP token
  // URL. Snowflake's GS does not mint CC tokens, so `oauth_token_request_url` is required up-front.
  std::stringstream ss;
  ss << get_oauth_base_connection_string(params, "OAUTH_CLIENT_CREDENTIALS", client_id);
  ss << "OAUTH_CLIENT_ID=" << client_id << ";";
  ss << "OAUTH_CLIENT_SECRET=" << client_secret << ";";
  ss << "OAUTH_TOKEN_REQUEST_URL=" << token_url << ";";
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();
  auto dbc = env.createConnectionHandle();

  // When Trying to Connect
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, (SQLCHAR*)connection_string.c_str(), SQL_NTS, nullptr, 0,
                                   nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);

  // Then Login is successful and a simple query can be executed
  verify_oauth_simple_query_execution(dbc);

  SQLRETURN disconnect_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(disconnect_ret, dbc);
}

TEST_CASE("oauth should fail client credentials flow with bad client secret", "[oauth_e2e][requires_browser]") {
  SKIP("Disabled: bad-secret tests cause pipeline flakiness by blocking the test account");
  REQUIRE_BROWSER("OAuth E2E needs the headless browser container (preprod IdP connectivity)");

  auto params = get_test_parameters("testconnection");
  const std::string token_url = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL");
  const std::string client_id = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_ID");

  // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid client secret and a
  // valid token_request_url
  std::stringstream ss;
  ss << get_oauth_base_connection_string(params, "OAUTH_CLIENT_CREDENTIALS", client_id);
  ss << "OAUTH_CLIENT_ID=" << client_id << ";";
  ss << "OAUTH_CLIENT_SECRET=invalid_client_secret_12345;";  // pragma: allowlist secret
  ss << "OAUTH_TOKEN_REQUEST_URL=" << token_url << ";";
  std::string connection_string = ss.str();

  ensure_driver_installed();

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then Connection fails with an authentication / login error
  REQUIRE(records.size() >= 1);
  // BD#83: the new driver maps an IdP token-exchange rejection to SQLSTATE 28000
  // (invalid authorization specification); the legacy driver surfaces it as the
  // generic HY000.
  OLD_DRIVER_ONLY("BD#83") { CHECK(records[0].sqlState == "HY000"); }
  NEW_DRIVER_ONLY("BD#83") { CHECK(records[0].sqlState == "28000"); }
}
