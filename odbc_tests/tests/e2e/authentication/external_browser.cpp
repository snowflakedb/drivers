// External browser authentication E2E test.
//
// Requires the snowdrivers-test-external-browser-universal-driver Docker container
// (headless Chromium + /externalbrowser/provideBrowserCredentials.js). The driver
// opens Chromium via the real browser opener (SF_TEST_BROWSER_OPENER is NOT set to
// "noop" here, unlike the WireMock integration test), and the Node automation script
// drives the Okta IdP login over Chromium's remote-debugging port.
//
// This mirrors python/tests/e2e/authentication/test_external_browser.py.
//
// Run locally:
//   ./tests/auth/run_auth_browser.sh odbc

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "oauth_auth_helpers.hpp"
#include "odbc_cast.hpp"
#include "require.hpp"
#include "test_setup.hpp"

// The browser machinery (Chromium debug port, provideBrowserCredentials.js,
// cleanBrowserProcesses.js) lives in oauth_auth_helpers.hpp and is shared with
// the OAuth E2E suite. It only exists inside the
// snowdrivers-test-external-browser-universal-driver container, so this test is
// gated behind REQUIRE_BROWSER.
namespace {

std::string get_external_browser_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  // Use the default test connection (host, account, etc.) but override UID to
  // the Okta user and switch the authenticator to EXTERNALBROWSER — mirroring
  // what the Python E2E test does via connection_factory(**browser_credentials).
  read_default_params(ss, params, {"UID", "AUTHENTICATOR", "ROLE"});
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_USER", "UID");
  ss << "AUTHENTICATOR=EXTERNALBROWSER;";
  ss << "ROLE=PUBLIC;";
  ss << "CLIENT_STORE_TEMPORARY_CREDENTIAL=false;";
  return ss.str();
}

void verify_simple_query_execution(ConnectionHandleWrapper& dbc) {
  StatementHandleWrapper stmt = dbc.createStatementHandle();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER result = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator != SQL_NULL_DATA);
  REQUIRE(result == 1);
}

}  // namespace

TEST_CASE("should authenticate with external browser via Okta IdP", "[external_browser_e2e][requires_browser]") {
  REQUIRE_BROWSER("External browser E2E needs the headless Chromium container");
  SKIP_OLD_DRIVER("", "New-driver-only: external browser E2E against headless Chromium");

  oauth_auth::clean_browser_processes();

  // Given External browser authentication is configured with valid Okta user
  std::string connection_string = get_external_browser_connection_string();

  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  auto params = get_test_parameters("testconnection");
  std::string login = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_USER");
  std::string password = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_PASSWORD");

  struct CleanupGuard {
    ConnectionHandleWrapper& dbc;
    bool connected = false;
    ~CleanupGuard() {
      if (connected) {
        SQLDisconnect(dbc.getHandle());
      }
      oauth_auth::clean_browser_processes();
    }
  } cleanup{dbc};

  // When Trying to Connect with headless browser providing valid credentials
  ret = oauth_auth::connect_with_browser_automation(dbc, connection_string, "success", login, password);

  // Then Login is successful and simple query can be executed
  REQUIRE_ODBC(ret, dbc);
  cleanup.connected = true;

  verify_simple_query_execution(dbc);

  ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(ret, dbc);
  cleanup.connected = false;
}
