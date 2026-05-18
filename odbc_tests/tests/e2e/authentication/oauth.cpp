#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "require.hpp"
#include "test_setup.hpp"

// End-to-end OAuth tests for the ODBC wrapper.
//
// These tests drive a real Snowflake account / IdP through the full
// SQLDriverConnect path. They are gated behind the
// SNOWFLAKE_TEST_OAUTH_* parameters in parameters.json and SKIP() when
// the relevant fields are missing -- mirroring the
// `private_key_auth.cpp` pattern and the `oauth.rs` E2E gating in
// sf_core/tests/e2e/authentication/.
//
// The Authorization Code happy-path scenarios spawn the OS browser
// (via sf_core's loopback listener); we gate them behind
// SNOWFLAKE_OAUTH_E2E_BROWSER=1 so a developer can opt in. The CC and
// legacy OAUTH paths do not require a browser and run whenever the
// matching parameters are configured.
//
// Required parameters.json keys (cross-driver configuration matrix):
//
//   * SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN     legacy OAUTH / AC short-circuit
//   * SNOWFLAKE_TEST_OAUTH_CLIENT_ID        AC + CC
//   * SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET    AC + CC
//   * SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL AC (optional; defaults to host)
//   * SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL AC (optional) / CC (required)
//   * SNOWFLAKE_TEST_OAUTH_REDIRECT_URI     AC (optional)
//   * SNOWFLAKE_TEST_OAUTH_SCOPE            AC + CC (optional)
//
// Test method names mirror sf_core's existing `oauth_should_*`
// methods in sf_core/tests/e2e/authentication/oauth.rs so the same
// Gherkin scenarios in tests/definitions/shared/authentication/oauth.feature
// validate against both implementations. Scenario step text below is
// taken verbatim from sf_core's oauth.rs comments where the @core_e2e
// tag pairs the two suites.

using namespace Catch::Matchers;

namespace {

#define REQUIRE_OAUTH_AC_BROWSER(message)                                                                    \
  do {                                                                                                       \
    if (std::getenv("SNOWFLAKE_OAUTH_E2E_BROWSER") == nullptr) {                                             \
      SKIP("OAuth AC E2E spawns a real OS browser; opt in with SNOWFLAKE_OAUTH_E2E_BROWSER=1: " << message); \
    }                                                                                                        \
  } while (0)

std::string require_oauth_param(const picojson::object& params, const std::string& key) {
  auto it = params.find(key);
  if (it == params.end() || !it->second.is<std::string>() || it->second.get<std::string>().empty()) {
    SKIP("OAuth E2E test requires " << key << " in parameters.json");
  }
  return it->second.get<std::string>();
}

void add_oauth_param_optional(std::stringstream& ss, const picojson::object& params, const std::string& cfg_key,
                              const std::string& conn_key) {
  auto it = params.find(cfg_key);
  if (it != params.end() && it->second.is<std::string>() && !it->second.get<std::string>().empty()) {
    ss << conn_key << "=" << it->second.get<std::string>() << ";";
  }
}

std::stringstream get_oauth_base_connection_stream(const std::string& authenticator) {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_WAREHOUSE", "WAREHOUSE");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_ROLE", "ROLE");
  ss << "AUTHENTICATOR=" << authenticator << ";";
  return ss;
}

EnvironmentHandleWrapper setup_oauth_environment() {
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_oauth_connection_handle(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

void attempt_oauth_connection(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);
}

void verify_oauth_simple_query_execution(ConnectionHandleWrapper& dbc) {
  StatementHandleWrapper stmt = dbc.createStatementHandle();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER result = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), NULL);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 1);
}

}  // anonymous namespace

// =============================================================================
// Legacy AUTHENTICATOR=OAUTH (pre-acquired access token)
// =============================================================================

TEST_CASE("oauth should authenticate with pre acquired access token", "[oauth_e2e]") {
  SKIP_OLD_DRIVER("", "OAuth flows are new-driver-only");
  auto params = get_test_parameters("testconnection");
  std::string access_token = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN");

  // Given Authentication is set to legacy OAUTH and a pre-acquired
  //       OAuth access token is supplied via `token=`
  std::stringstream ss = get_oauth_base_connection_stream("OAUTH");
  ss << "TOKEN=" << access_token << ";";
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);

  // When Trying to Connect
  attempt_oauth_connection(dbc, connection_string);

  // Then Login is successful and a simple query can be executed
  verify_oauth_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("oauth should fail legacy authentication with invalid token", "[oauth_e2e]") {
  SKIP_OLD_DRIVER("", "OAuth flows are new-driver-only");

  // Given Authentication is set to legacy OAUTH and an invalid
  //       OAuth access token is supplied
  std::stringstream ss = get_oauth_base_connection_stream("OAUTH");
  ss << "TOKEN=invalid_oauth_token_12345;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then Connection fails with an authentication / login error
  REQUIRE(records.size() >= 1);
  CHECK(records[0].sqlState == "28000");
}

TEST_CASE("oauth should authenticate using lowercase oauth authenticator", "[oauth_e2e]") {
  SKIP_OLD_DRIVER("", "OAuth flows are new-driver-only");
  auto params = get_test_parameters("testconnection");
  std::string access_token = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN");

  // Given Authentication is set to lowercase oauth and a valid pre-acquired OAuth access token is supplied via TOKEN
  std::stringstream ss = get_oauth_base_connection_stream("oauth");
  ss << "TOKEN=" << access_token << ";";
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);

  // When Trying to Connect
  attempt_oauth_connection(dbc, connection_string);

  // Then Login is successful and a simple query can be executed
  verify_oauth_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

// =============================================================================
// OAuth Authorization Code (AC) flow
// =============================================================================
//
// AC requires a real browser leg unless an OAuth access token has been
// pre-seeded in the OS keyring. The seeding helper lives in the Rust
// e2e test (sf_core/tests/e2e/authentication/oauth.rs) and is not
// exposed to C++ ODBC tests, so the keyring-short-circuit scenario
// stays sf_core-only. The two scenarios below opt in via
// SNOWFLAKE_OAUTH_E2E_BROWSER=1.

TEST_CASE("oauth should authenticate using authorization code flow", "[oauth_e2e]") {
  SKIP_OLD_DRIVER("", "OAuth flows are new-driver-only");
  REQUIRE_OAUTH_AC_BROWSER("Authorization Code happy path");
  auto params = get_test_parameters("testconnection");
  std::string client_id = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_CLIENT_ID");
  std::string client_secret = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET");

  // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a
  //       valid client id / secret. `oauth_authorization_url` and
  //       `oauth_token_request_url` are forwarded from parameters
  //       when present (otherwise the driver falls back to the
  //       Snowflake-IdP defaults `https://{host}/oauth/authorize`
  //       and `https://{host}/oauth/token-request`).
  //       `client_store_temporary_credential=true` lets the AC flow
  //       short-circuit on subsequent runs by re-using the cached
  //       access / refresh token (AC state machine: cache → refresh → interactive).
  std::stringstream ss = get_oauth_base_connection_stream("OAUTH_AUTHORIZATION_CODE");
  ss << "OAUTH_CLIENT_ID=" << client_id << ";";
  ss << "OAUTH_CLIENT_SECRET=" << client_secret << ";";
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL", "OAUTH_AUTHORIZATION_URL");
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL", "OAUTH_TOKEN_REQUEST_URL");
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_REDIRECT_URI", "OAUTH_REDIRECT_URI");
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_SCOPE", "OAUTH_SCOPE");
  ss << "CLIENT_STORE_TEMPORARY_CREDENTIAL=true;";
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);

  // When Trying to Connect (this will spawn the local-loopback HTTP
  //      listener and `xdg-open`/`open`/`ShellExecute` the IdP login URL
  //      unless a previously cached access token short-circuits the leg)
  attempt_oauth_connection(dbc, connection_string);

  // Then Login is successful and a simple query can be executed
  verify_oauth_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("oauth should fail authorization code flow with bad client secret", "[oauth_e2e]") {
  SKIP_OLD_DRIVER("", "OAuth flows are new-driver-only");
  REQUIRE_OAUTH_AC_BROWSER("Authorization Code negative path (browser leg still required)");
  auto params = get_test_parameters("testconnection");
  std::string client_id = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_CLIENT_ID");

  // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a
  //       valid client id but a deliberately invalid client secret.
  //       The IdP token-exchange step must reject the credentials
  //       and the driver must surface an authentication / login
  //       error.
  std::stringstream ss = get_oauth_base_connection_stream("OAUTH_AUTHORIZATION_CODE");
  ss << "OAUTH_CLIENT_ID=" << client_id << ";";
  ss << "OAUTH_CLIENT_SECRET=invalid_client_secret_12345;";  // pragma: allowlist secret
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL", "OAUTH_AUTHORIZATION_URL");
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL", "OAUTH_TOKEN_REQUEST_URL");
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_REDIRECT_URI", "OAUTH_REDIRECT_URI");
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_SCOPE", "OAUTH_SCOPE");
  ss << "CLIENT_STORE_TEMPORARY_CREDENTIAL=false;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then Connection fails with an authentication / login error
  REQUIRE(records.size() >= 1);
  CHECK(records[0].sqlState == "28000");
}

// =============================================================================
// OAuth Client Credentials (CC) flow
// =============================================================================
//
// CC does not launch a browser -- it performs an HTTP token exchange
// directly against the configured IdP token URL. Snowflake itself does
// not mint CC tokens, so SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL
// is required up-front.

TEST_CASE("oauth should authenticate using client credentials flow", "[oauth_e2e]") {
  SKIP_OLD_DRIVER("", "OAuth flows are new-driver-only");
  auto params = get_test_parameters("testconnection");
  std::string client_id = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_CLIENT_ID");
  std::string client_secret = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET");
  std::string token_url = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL");

  // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a
  //       valid client id / secret and an external IdP token URL.
  //       Snowflake's GS does not mint CC tokens, so
  //       `oauth_token_request_url` is required up-front.
  std::stringstream ss = get_oauth_base_connection_stream("OAUTH_CLIENT_CREDENTIALS");
  ss << "OAUTH_CLIENT_ID=" << client_id << ";";
  ss << "OAUTH_CLIENT_SECRET=" << client_secret << ";";
  ss << "OAUTH_TOKEN_REQUEST_URL=" << token_url << ";";
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_SCOPE", "OAUTH_SCOPE");
  std::string connection_string = ss.str();

  auto env = setup_oauth_environment();
  auto dbc = get_oauth_connection_handle(env);

  // When Trying to Connect
  attempt_oauth_connection(dbc, connection_string);

  // Then Login is successful and a simple query can be executed
  verify_oauth_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("oauth should fail client credentials flow with bad client secret", "[oauth_e2e]") {
  SKIP_OLD_DRIVER("", "OAuth flows are new-driver-only");
  auto params = get_test_parameters("testconnection");
  std::string client_id = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_CLIENT_ID");
  std::string token_url = require_oauth_param(params, "SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL");

  // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid client secret and a
  // valid token_request_url
  std::stringstream ss = get_oauth_base_connection_stream("OAUTH_CLIENT_CREDENTIALS");
  ss << "OAUTH_CLIENT_ID=" << client_id << ";";
  ss << "OAUTH_CLIENT_SECRET=invalid_client_secret_12345;";  // pragma: allowlist secret
  ss << "OAUTH_TOKEN_REQUEST_URL=" << token_url << ";";
  add_oauth_param_optional(ss, params, "SNOWFLAKE_TEST_OAUTH_SCOPE", "OAUTH_SCOPE");
  std::string connection_string = ss.str();

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then Connection fails with an authentication / login error
  REQUIRE(records.size() >= 1);
  CHECK(records[0].sqlState == "28000");
}
