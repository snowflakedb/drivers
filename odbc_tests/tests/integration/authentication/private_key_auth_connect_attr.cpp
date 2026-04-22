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
#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "put_get_utils.hpp"
#include "sf_odbc.h"
#include "test_setup.hpp"
#include "utils.hpp"

using pg_utils::TempTestDir;

static std::string get_base_jwt_connection_string_int() {
  std::stringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "ACCOUNT=test_account;";
  ss << "UID=test_user;";
  ss << "DATABASE=test_database;";
  ss << "SCHEMA=test_schema;";
  ss << "WAREHOUSE=test_warehouse;";
  ss << "ROLE=test_role;";
  ss << "PORT=8090;";
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  return ss.str();
}

static std::string read_test_private_key_content() {
  auto key_path = test_utils::test_data_file_path("invalid_rsa_key.p8");
  std::ifstream file(key_path.string());
  REQUIRE(file.is_open());
  std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
  return content;
}

static EnvironmentHandleWrapper setup_environment_integration() {
  ensure_driver_installed();
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

static ConnectionHandleWrapper get_connection_handle_integration(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

static void verify_private_key_forwarded_to_core(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);

  if (ret == SQL_ERROR) {
    auto records = get_diag_rec(dbc);
    using Catch::Matchers::ContainsSubstring;
    for (const auto& record : records) {
      // Error must not be about a missing parameter (any other error is acceptable).
      CHECK_THAT(record.messageText, !ContainsSubstring("Missing required parameter"));
      CHECK_THAT(record.messageText, !ContainsSubstring("Can't open lib"));
    }
  }
  // SQL_SUCCESS means the key was forwarded and used successfully.
}

// ============================================================================
// Integration tests: SQLSetConnectAttr forwarding to core
// ============================================================================

TEST_CASE("should forward private key content set via SQLSetConnectAttr to core", "[private_key_auth_connect_attr]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests direct attribute handling");

  // Given A connection handle is allocated and PRIV_KEY_CONTENT is set via SQLSetConnectAttr
  auto env = setup_environment_integration();
  auto dbc = get_connection_handle_integration(env);

  std::string test_key_pem = read_test_private_key_content();

  SQLRETURN ret = SQLSetConnectAttr(dbc.getHandle(), SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT,
                                    (SQLPOINTER)test_key_pem.c_str(), (SQLINTEGER)test_key_pem.size());
  REQUIRE_ODBC(ret, dbc);

  // When Trying to Connect
  std::string connection_string = get_base_jwt_connection_string_int();

  // Then The private key is forwarded to core and used for JWT authentication
  verify_private_key_forwarded_to_core(dbc, connection_string);
}

TEST_CASE("should forward base64 private key set via SQLSetConnectAttr to core", "[private_key_auth_connect_attr]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests direct attribute handling");

  // Given A connection handle is allocated and PRIV_KEY_BASE64 is set via SQLSetConnectAttr
  auto env = setup_environment_integration();
  auto dbc = get_connection_handle_integration(env);

  std::string test_key_pem = read_test_private_key_content();
  std::string test_key_b64 = test_utils::base64_encode(test_key_pem);

  SQLRETURN ret = SQLSetConnectAttr(dbc.getHandle(), SQL_SF_CONN_ATTR_PRIV_KEY_BASE64, (SQLPOINTER)test_key_b64.c_str(),
                                    (SQLINTEGER)test_key_b64.size());
  REQUIRE_ODBC(ret, dbc);

  // When Trying to Connect
  std::string connection_string = get_base_jwt_connection_string_int();

  // Then The private key is forwarded to core and used for JWT authentication
  verify_private_key_forwarded_to_core(dbc, connection_string);
}

TEST_CASE("should forward private key password set via SQLSetConnectAttr to core", "[private_key_auth_connect_attr]") {
  SKIP_OLD_DRIVER("", "New-driver-only: tests direct attribute handling");

  // Given A connection handle is allocated and PRIV_KEY_PASSWORD is set via SQLSetConnectAttr
  auto env = setup_environment_integration();
  auto dbc = get_connection_handle_integration(env);

  // Create an encrypted key file to test password forwarding
  TempTestDir tmp("int_auth_pwd_");
  std::string test_key_pem = read_test_private_key_content();
  const auto encrypted_path = tmp.path() / "encrypted.pem";
  const std::string test_password = "test_password_123";
  test_utils::encrypt_pem_key_to_file(test_key_pem, test_password, encrypted_path);

  // Set password via SQLSetConnectAttr
  SQLRETURN ret = SQLSetConnectAttr(dbc.getHandle(), SQL_SF_CONN_ATTR_PRIV_KEY_PASSWORD,
                                    (SQLPOINTER)test_password.c_str(), (SQLINTEGER)test_password.size());
  REQUIRE_ODBC(ret, dbc);

  // When Trying to Connect
  std::string connection_string = get_base_jwt_connection_string_int();
  connection_string += "PRIV_KEY_FILE=" + encrypted_path.string() + ";";

  // Then The private key password is forwarded to core and used for JWT authentication
  verify_private_key_forwarded_to_core(dbc, connection_string);
}
