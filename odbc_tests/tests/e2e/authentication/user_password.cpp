#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "require.hpp"
#include "test_setup.hpp"

using namespace Catch::Matchers;

std::string get_password_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_PASSWORD", "PWD");
  return ss.str();
}

std::string get_password_connection_string_with_explicit_authenticator() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_PASSWORD", "PWD");
  ss << "AUTHENTICATOR=snowflake;";
  return ss.str();
}

std::string get_wrong_password_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  ss << "PWD=definitely_not_a_valid_password_12345;";
  return ss.str();
}

void verify_simple_query(ConnectionHandleWrapper& dbc) {
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

TEST_CASE("should authenticate using username and password", "[user_password][requires_no_mfa]") {
  REQUIRE_NO_MFA("requires parameters_aws_local.json");

  // Given Authentication is set to default (snowflake) with valid username and password
  std::string connection_string = get_password_connection_string();

  // When Trying to Connect
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);

  auto dbc = env.createConnectionHandle();
  ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                         SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);

  // Then Login is successful and simple query can be executed
  verify_simple_query(dbc);
  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("should authenticate using explicit snowflake authenticator", "[user_password][requires_no_mfa]") {
  REQUIRE_NO_MFA("requires parameters_aws_local.json");
  SKIP_OLD_DRIVER("", "Old driver already accepts 'snowflake' — test verifies new driver does too");

  // Given Authentication is explicitly set to snowflake with valid username and password
  std::string connection_string = get_password_connection_string_with_explicit_authenticator();

  // When Trying to Connect
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);

  auto dbc = env.createConnectionHandle();
  ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                         SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);

  // Then Login is successful and simple query can be executed
  verify_simple_query(dbc);
  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("should fail authentication when wrong password is provided", "[user_password][requires_no_mfa]") {
  REQUIRE_NO_MFA("requires parameters_aws_local.json");

  // Given Authentication is set to default with valid username and wrong password
  std::string connection_string = get_wrong_password_connection_string();

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then There is error returned
  REQUIRE(records.size() >= 1);
  CHECK(records[0].sqlState == "28000");
}
