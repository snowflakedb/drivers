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
#include "odbc_matchers.hpp"
#include "require.hpp"
#include "test_setup.hpp"

using namespace Catch::Matchers;

static const bool PASSCODE_IN_PASSWORD = true;
static const bool PASSCODE_NOT_IN_PASSWORD = false;

// =============================================================================
// Helpers
// =============================================================================

std::string get_passcode() {
  return "totp_secret";  // pragma allowed secret
}

std::string get_mfa_connection_string(bool passcodeInPassword) {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  std::string pwd = get_param_required<std::string>(params, "SNOWFLAKE_TEST_PASSWORD");
  std::string passcode = get_passcode();
  ss << "AUTHENTICATOR=USERNAME_PASSWORD_MFA;";
  if (passcodeInPassword) {
    ss << "PWD=" << pwd << passcode << ";";
    ss << "PASSCODEINPASSWORD=true;";
  } else {
    ss << "PWD=" << pwd << ";";
    ss << "PASSCODE=" << passcode << ";";
  }
  return ss.str();
}

EnvironmentHandleWrapper setup_environment() {
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_connection_handle(EnvironmentHandleWrapper& env) { return env.createConnectionHandle(); }

void attempt_connection(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);
}

void verify_simple_query_execution(ConnectionHandleWrapper& dbc) {
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

// =============================================================================
// E2E Tests
// =============================================================================

TEST_CASE("should authenticate using user password mfa with passcode in password", "[mfa_auth]") {
  // TOTP codes are one time and would be extremely flaky on CI
  REQUIRE_DAILY_RUN_JENKINS_JOB("MFA specific setup required");
  std::string connection_string = get_mfa_connection_string(PASSCODE_IN_PASSWORD);
  // Given MFA authentication is configured with valid credentials
  auto env = setup_environment();
  auto dbc = get_connection_handle(env);

  // When Trying to Connect
  attempt_connection(dbc, connection_string);

  // Then Login is successful
  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("should authenticate using user password mfa with passcode explicit", "[mfa_auth]") {
  // TOTP codes are one time and would be extremely flaky on CI
  REQUIRE_DAILY_RUN_JENKINS_JOB("MFA specific setup required");

  std::string connection_string = get_mfa_connection_string(PASSCODE_NOT_IN_PASSWORD);
  // Given MFA authentication is configured with valid credentials
  auto env = setup_environment();
  auto dbc = get_connection_handle(env);

  // When Trying to Connect
  attempt_connection(dbc, connection_string);

  // Then Login is successful
  SQLDisconnect(dbc.getHandle());
}
