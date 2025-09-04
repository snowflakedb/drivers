#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <fstream>
#include <iostream>
#include <sstream>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

using namespace Catch::Matchers;

std::string get_private_key_path(picojson::object& params) {
  auto private_key = read_private_key(params);
  const std::string path = "./rsa_key.p8";
  std::ofstream file(path, std::ios::out | std::ios::trunc);
  REQUIRE(file.is_open());
  file << private_key;
  file.close();
  return path;
}

// TODO: Add more test cases for private key authentication
// - Test with private key contents
// - Test with private key contents base64
// - Test with private key file permissions
// - Test without private key password

std::string get_private_key_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params);
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD",
                                  "PRIV_KEY_FILE_PWD");
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_FILE=" << get_private_key_path(params) << ";";
  return ss.str();
}

TEST_CASE("Private Key Authentication - Basic Connection", "[private_key_auth]") {
  EnvironmentHandleWrapper env;

  SQLRETURN ret =
      SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC(ret, env);

  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  std::string connection_string = get_private_key_connection_string();
  ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL,
                         0, NULL, SQL_DRIVER_NOPROMPT);
  CHECK_ODBC(ret, dbc);

  {
    StatementHandleWrapper stmt = dbc.createStatementHandle();
    ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT CURRENT_USER()", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    SQLCHAR username[256];
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, username, sizeof(username), NULL);
    CHECK_ODBC(ret, stmt);

    INFO("Connected as user: " << username);
  }

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("Private Key Authentication - Missing Key File", "[private_key_auth]") {
  EnvironmentHandleWrapper env;

  SQLRETURN ret =
      SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC(ret, env);

  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  // Create connection string with non-existent key file
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  ss << "DRIVER=" << get_driver_path() << ";";
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  ss << "PRIV_KEY_FILE=/nonexistent/path/key.pem;";
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";

  std::string connection_string = ss.str();
  ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL,
                         0, NULL, SQL_DRIVER_NOPROMPT);

  // Should fail with an error
  REQUIRE(ret == SQL_ERROR);

  auto records = get_diag_rec(dbc);
  REQUIRE(records.size() == 1);  // Expecting one error record
  OLD_DRIVER_ONLY(1) {
    CHECK(records[0].sqlState == "HY000");
    // TODO: Add more specific error code and message
    CHECK(records[0].nativeError == 45);
    CHECK_THAT(records[0].messageText,
               ContainsSubstring("Error loading private key file: No such file or directory."));
  }

  NEW_DRIVER_ONLY(1) {
    CHECK(records[0].sqlState == "28000");
    CHECK(records[0].nativeError == 0);
    CHECK_THAT(records[0].messageText,
               ContainsSubstring(
                   "Invalid value '/nonexistent/path/key.pem' for parameter 'private_key_file'"));
  }
}

TEST_CASE("Private Key Authentication - No Private Key Parameter", "[private_key_auth]") {
  EnvironmentHandleWrapper env;

  SQLRETURN ret =
      SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC(ret, env);

  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  // Create connection string without private key parameter
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  ss << "DRIVER=" << get_driver_path() << ";";
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";

  std::string connection_string = ss.str();
  ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL,
                         0, NULL, SQL_DRIVER_NOPROMPT);

  // Should fail with an error
  REQUIRE(ret == SQL_ERROR);

  auto records = get_diag_rec(dbc);
  REQUIRE(records.size() == 1);  // Expecting one error record
  CHECK(records[0].sqlState == "28000");
  // TODO: Add more specific error code and message
  OLD_DRIVER_ONLY("BC#1: Native error code should be 0 when error originates from client side.") {
    CHECK(records[0].nativeError == 20032);
    CHECK_THAT(records[0].messageText, ContainsSubstring("Required setting 'PRIV_KEY_FILE'"));
  }

  NEW_DRIVER_ONLY("BC#1: Native error code should be 0 when error originates from client side.") {
    INFO("Message text: " << records[0].messageText);
    CHECK(records[0].nativeError == 0);
    CHECK_THAT(records[0].messageText,
               ContainsSubstring("Missing required parameter: private_key_file"));
  }
}
