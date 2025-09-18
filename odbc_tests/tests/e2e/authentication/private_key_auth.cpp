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
#include "utils.hpp"

std::string get_private_key_path_for_auth(picojson::object& params) {
  auto private_key = read_private_key(params);
  const std::string path = "./rsa_key_auth.p8";
  std::ofstream file(path, std::ios::out | std::ios::trunc);
  REQUIRE(file.is_open());
  file << private_key;
  file.close();
  return path;
}

std::string get_jwt_connection_string_with_private_key() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params);
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD",
                                  "PRIV_KEY_FILE_PWD");
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_FILE=" << get_private_key_path_for_auth(params) << ";";
  return ss.str();
}

std::string get_jwt_connection_string_with_invalid_private_key() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params);
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD",
                                  "PRIV_KEY_FILE_PWD");
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_FILE=" << test_utils::test_data_file_path("invalid_rsa_key.p8").string() << ";";
  return ss.str();
}

EnvironmentHandleWrapper setup_environment() {
  EnvironmentHandleWrapper env;
  SQLRETURN ret =
      SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_connection_handle(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

void attempt_connection(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(),
                                   SQL_NTS, NULL, 0, NULL, SQL_DRIVER_NOPROMPT);
  CHECK_ODBC(ret, dbc);
}

void verify_simple_query_execution(ConnectionHandleWrapper& dbc) {
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

SQLRETURN attempt_connection_expect_error(ConnectionHandleWrapper& dbc,
                                          const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(),
                                   SQL_NTS, NULL, 0, NULL, SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);
  return ret;
}

void assert_login_error(ConnectionHandleWrapper& dbc) {
  auto records = get_diag_rec(dbc);
  REQUIRE(records.size() == 1);
  CHECK(records[0].sqlState == "28000");
  REQUIRE(!records[0].messageText.empty());
}

TEST_CASE("should authenticate using private file with password", "[private_key_auth]") {
  // Given Authentication is set to JWT and private file with password is provided
  auto env = setup_environment();
  auto dbc = get_connection_handle(env);
  std::string connection_string = get_jwt_connection_string_with_private_key();

  // When Trying to Connect
  attempt_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  verify_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("should fail JWT authentication when invalid private key provided",
          "[private_key_auth]") {
  // Given Authentication is set to JWT and invalid private key file is provided
  auto env = setup_environment();
  auto dbc = get_connection_handle(env);
  std::string connection_string = get_jwt_connection_string_with_invalid_private_key();

  // When Trying to Connect
  attempt_connection_expect_error(dbc, connection_string);

  // Then There is error returned
  assert_login_error(dbc);
}
