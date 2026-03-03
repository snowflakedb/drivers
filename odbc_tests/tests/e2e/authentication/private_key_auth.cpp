#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <fstream>
#include <iostream>
#include <sstream>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "put_get_utils.hpp"
#include "test_setup.hpp"
#include "utils.hpp"

using pg_utils::TempTestDir;

std::string get_private_key_path_for_auth(picojson::object& params, const TempTestDir& tmp) {
  // First check if a private key file path is provided
  auto private_key_file_path = get_private_key_file_path(params);
  if (!private_key_file_path.empty()) {
    return private_key_file_path;
  }

  // Otherwise, create a temporary file from contents
  auto private_key = read_private_key(params);
  auto path = tmp.path() / "rsa_key_auth.p8";
  std::ofstream file(path, std::ios::out | std::ios::trunc);
  REQUIRE(file.is_open());
  file << private_key;
  file.close();
  return path.string();
}

std::string get_jwt_connection_string_with_private_key(const TempTestDir& tmp) {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params);
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", "PRIV_KEY_FILE_PWD");
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_FILE=" << get_private_key_path_for_auth(params, tmp) << ";";
  return ss.str();
}

std::string get_jwt_connection_string_with_invalid_private_key() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params);
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", "PRIV_KEY_FILE_PWD");
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_FILE=" << test_utils::test_data_file_path("invalid_rsa_key.p8").string() << ";";
  return ss.str();
}

EnvironmentHandleWrapper setup_environment() {
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_connection_handle(EnvironmentHandleWrapper& env) { return env.createConnectionHandle(); }

void attempt_connection(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
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

SQLRETURN attempt_connection_expect_error(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
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
  TempTestDir tmp("e2e_auth_pwd_");
  auto env = setup_environment();
  auto dbc = get_connection_handle(env);
  std::string connection_string = get_jwt_connection_string_with_private_key(tmp);

  // When Trying to Connect
  attempt_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  verify_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("should fail JWT authentication when invalid private key provided", "[private_key_auth]") {
  // Given Authentication is set to JWT and invalid private key file is provided
  auto env = setup_environment();
  auto dbc = get_connection_handle(env);
  std::string connection_string = get_jwt_connection_string_with_invalid_private_key();

  // When Trying to Connect
  attempt_connection_expect_error(dbc, connection_string);

  // Then There is error returned
  assert_login_error(dbc);
}

TEST_CASE("should authenticate using unencrypted private key file", "[private_key_auth]") {
  // Given Authentication is set to JWT and an unencrypted private key file is provided (no password)
  TempTestDir tmp("e2e_auth_unenc_");
  auto params = get_test_parameters("testconnection");
  auto env = setup_environment();
  auto dbc = get_connection_handle(env);

  // Decrypt the test key to produce an unencrypted PEM file.
  std::string encrypted_pem = read_private_key(params);
  const auto unencrypted_path = tmp.path() / "unencrypted.p8";

  auto pwd_it = params.find("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD");
  if (pwd_it == params.end() || !pwd_it->second.is<std::string>() || pwd_it->second.get<std::string>().empty()) {
    SKIP("No private key password configured; cannot decrypt key for unencrypted test");
  }
  std::string password = pwd_it->second.get<std::string>();
  test_utils::decrypt_pem_key_to_file(encrypted_pem, password, unencrypted_path);

  // Build connection string without PRIV_KEY_FILE_PWD
  std::stringstream ss;
  read_default_params(ss, params);
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_FILE=" << unencrypted_path.string() << ";";
  std::string connection_string = ss.str();

  // When Trying to Connect
  attempt_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  verify_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("should authenticate using private_key as base64 string", "[private_key_auth]") {
  // Old driver requires PRIV_KEY_FILE even when PRIV_KEY_BASE64 is provided.
  // TODO: Re-enable once DSN support is implemented (provide PRIV_KEY_FILE via DSN).
  SKIP_OLD_DRIVER("", "Old driver requires PRIV_KEY_FILE even when PRIV_KEY_BASE64 is set");

  // Given Authentication is set to JWT and private key is provided as base64-encoded string
  auto params = get_test_parameters("testconnection");
  auto env = setup_environment();
  auto dbc = get_connection_handle(env);

  // Base64-encode the PEM private key
  std::string private_key_pem = read_private_key(params);
  std::string private_key_b64 = test_utils::base64_encode(private_key_pem);

  // Build connection string with PRIV_KEY_BASE64 (key is encrypted, so include password)
  std::stringstream ss;
  read_default_params(ss, params);
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", "PRIV_KEY_PWD");
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_BASE64=" << private_key_b64 << ";";
  std::string connection_string = ss.str();

  // When Trying to Connect
  attempt_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  verify_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("should authenticate using PRIV_KEY_PWD as alias for private key password", "[private_key_auth]") {
  // Old driver's PRIV_KEY_PWD only works with inline keys, not PRIV_KEY_FILE.
  // TODO: Re-enable once DSN support is implemented.
  SKIP_OLD_DRIVER("", "Old driver's PRIV_KEY_PWD only works with inline keys, not PRIV_KEY_FILE");

  // Given Authentication is set to JWT with encrypted key file and PRIV_KEY_PWD parameter
  auto params = get_test_parameters("testconnection");

  // Need a password-protected key for this test
  auto pwd_it = params.find("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD");
  if (pwd_it == params.end() || !pwd_it->second.is<std::string>() || pwd_it->second.get<std::string>().empty()) {
    SKIP("No private key password configured; skipping PRIV_KEY_PWD test");
  }

  TempTestDir tmp("e2e_auth_pwd_alias_");
  auto env = setup_environment();
  auto dbc = get_connection_handle(env);

  // Write encrypted key to file
  std::string key_path = get_private_key_path_for_auth(params, tmp);
  std::string password = pwd_it->second.get<std::string>();

  // Use PRIV_KEY_PWD instead of PRIV_KEY_FILE_PWD
  std::stringstream ss;
  read_default_params(ss, params);
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_FILE=" << key_path << ";";
  ss << "PRIV_KEY_PWD=" << password << ";";
  std::string connection_string = ss.str();

  // When Trying to Connect
  attempt_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  verify_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}
