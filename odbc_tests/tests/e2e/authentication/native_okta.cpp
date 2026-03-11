#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <sstream>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "require.hpp"
#include "test_setup.hpp"

using namespace Catch::Matchers;

// =============================================================================
// Helpers
// =============================================================================

std::string get_okta_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_USER", "UID");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_PASSWORD", "PWD");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_URL", "AUTHENTICATOR");
  ss << "ROLE=PUBLIC;";
  return ss.str();
}

EnvironmentHandleWrapper setup_okta_environment() {
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_okta_connection_handle(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

void attempt_okta_connection(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
  CHECK_ODBC(ret, dbc);
}

void verify_okta_simple_query_execution(ConnectionHandleWrapper& dbc) {
  StatementHandleWrapper stmt = dbc.createStatementHandle();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  SQLINTEGER result = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), NULL);
  CHECK_ODBC(ret, stmt);
  REQUIRE(result == 1);
}

// =============================================================================
// E2E Tests
// =============================================================================

TEST_CASE("should authenticate using native okta", "[native_okta]") {
  REQUIRE_VPN("Native Okta E2E tests need access to preprod Snowflake account");
  std::string connection_string = get_okta_connection_string();

  // Given Okta authentication is configured with valid credentials
  auto env = setup_okta_environment();
  auto dbc = get_okta_connection_handle(env);

  // When Trying to Connect
  attempt_okta_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  verify_okta_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("should fail native okta authentication with wrong credentials", "[native_okta]") {
  REQUIRE_VPN("Native Okta E2E tests need access to preprod Snowflake account");

  // Given Okta authentication is configured with wrong password
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_USER", "UID");
  ss << "PWD=wrong_password_12345;";
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_URL", "AUTHENTICATOR");
  ss << "ROLE=PUBLIC;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then Connection fails with authentication error
  REQUIRE(records.size() >= 1);
  CHECK(records[0].sqlState == "28000");
  CHECK_THAT(records[0].messageText, ContainsSubstring("rejected credentials"));
}

TEST_CASE("should fail native okta authentication with wrong okta url", "[native_okta]") {
  REQUIRE_VPN("Native Okta E2E tests need access to preprod Snowflake account");

  // Given Okta authentication is configured with invalid okta url
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_USER", "UID");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_PASSWORD", "PWD");
  ss << "AUTHENTICATOR=https://invalid.okta.com;";
  ss << "ROLE=PUBLIC;";
  std::string connection_string = ss.str();

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then Connection fails with authentication error
  REQUIRE(records.size() >= 1);
  CHECK(records[0].sqlState == "28000");
  CHECK_THAT(records[0].messageText,
             ContainsSubstring("does not match configured Okta URL") || ContainsSubstring("Native Okta SSO failed"));
}
