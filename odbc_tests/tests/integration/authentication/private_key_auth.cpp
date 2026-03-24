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

std::string get_jwt_connection_string_without_private_key() {
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
  // Deliberately omit PRIV_KEY_FILE parameter
  return ss.str();
}

std::string get_base_jwt_connection_string_int() {
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

std::string read_test_private_key_content() {
  auto key_path = test_utils::test_data_file_path("invalid_rsa_key.p8");
  std::ifstream file(key_path.string());
  REQUIRE(file.is_open());
  std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
  return content;
}

EnvironmentHandleWrapper setup_environment_integration() {
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_connection_handle_integration(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

SQLRETURN attempt_connection_expect_error_integration(ConnectionHandleWrapper& dbc,
                                                      const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);
  return ret;
}

void verify_connection_fails_with_missing_private_key_error(ConnectionHandleWrapper& dbc,
                                                            const std::string& connection_string) {
  attempt_connection_expect_error_integration(dbc, connection_string);

  auto records = get_diag_rec(dbc);
  REQUIRE(records.size() == 1);
  using Catch::Matchers::ContainsSubstring;
  OLD_DRIVER_ONLY("BD#1") {
    CHECK(records[0].sqlState == "28000");
    CHECK(records[0].nativeError == 20032);
    CHECK_THAT(records[0].messageText, ContainsSubstring("Required setting 'PRIV_KEY_FILE'"));
  }

  NEW_DRIVER_ONLY("BD#1") {
    CHECK(records[0].sqlState == "01S00");
    CHECK(records[0].nativeError == 0);
    CHECK_THAT(records[0].messageText,
               ContainsSubstring("Missing required parameter: private_key or private_key_file"));
  }
}

void verify_private_key_forwarded_to_core(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);

  if (ret == SQL_ERROR) {
    auto records = get_diag_rec(dbc);
    using Catch::Matchers::ContainsSubstring;
    for (const auto& record : records) {
      // Error must not be about a missing parameter (any other error is acceptable).
      CHECK_THAT(record.messageText, !ContainsSubstring("Missing required parameter"));
    }
  }
  // SQL_SUCCESS means the key was forwarded and used successfully.
}

// ============================================================================
// Integration test: missing parameter error
// ============================================================================

TEST_CASE("should fail JWT authentication when no private file provided", "[private_key_auth]") {
  // Given Authentication is set to JWT
  /* TODO: Explicit config installation */
  std::string connection_string = get_jwt_connection_string_without_private_key();

  // When Trying to Connect with no private file provided
  auto env = setup_environment_integration();
  auto dbc = get_connection_handle_integration(env);

  // Then There is error returned
  verify_connection_fails_with_missing_private_key_error(dbc, connection_string);
}

// ============================================================================
// Integration tests: SQLSetConnectAttr forwarding to core
// ============================================================================

TEST_CASE("should forward private key content set via SQLSetConnectAttr to core", "[private_key_auth]") {
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

TEST_CASE("should forward base64 private key set via SQLSetConnectAttr to core", "[private_key_auth]") {
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

TEST_CASE("should forward private key password set via SQLSetConnectAttr to core", "[private_key_auth]") {
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
