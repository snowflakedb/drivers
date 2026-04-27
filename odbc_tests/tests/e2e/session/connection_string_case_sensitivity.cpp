#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "test_setup.hpp"

/// Build a connection string with lowercase key names from the test parameters.
static std::string get_lowercase_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);

  auto req = [&](const std::string& cfg, const std::string& conn) {
    add_param_required<std::string>(ss, params, cfg, conn);
  };
  auto opt = [&](const std::string& cfg, const std::string& conn) {
    add_param_optional<std::string>(ss, params, cfg, conn);
  };

  // Use all-lowercase key names instead of the usual UPPERCASE ones.
  req("SNOWFLAKE_TEST_HOST", "server");
  req("SNOWFLAKE_TEST_ACCOUNT", "account");
  req("SNOWFLAKE_TEST_USER", "uid");
  if (params.count("SNOWFLAKE_TEST_WAREHOUSE_ODBC")) {
    add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_WAREHOUSE_ODBC", "warehouse");
  } else {
    add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_WAREHOUSE", "warehouse");
  }
  opt("SNOWFLAKE_TEST_ROLE", "role");
  opt("SNOWFLAKE_TEST_SCHEMA", "schema");
  opt("SNOWFLAKE_TEST_DATABASE", "database");
  opt("SNOWFLAKE_TEST_PORT", "port");
  opt("SNOWFLAKE_TEST_PROTOCOL", "protocol");

  ss << "authenticator=SNOWFLAKE_JWT;";
#ifdef SNOWFLAKE_OLD_DRIVER
  ss << "priv_key_file=" << get_or_create_private_key_file(params) << ";";
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", "priv_key_file_pwd");
#else
  ss << "priv_key_base64=" << test_utils::base64_encode(read_private_key(params)) << ";";
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", "priv_key_pwd");
#endif
  return ss.str();
}

/// Build a connection string with mixed-case key names.
static std::string get_mixed_case_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);

  auto req = [&](const std::string& cfg, const std::string& conn) {
    add_param_required<std::string>(ss, params, cfg, conn);
  };
  auto opt = [&](const std::string& cfg, const std::string& conn) {
    add_param_optional<std::string>(ss, params, cfg, conn);
  };

  req("SNOWFLAKE_TEST_HOST", "Server");
  req("SNOWFLAKE_TEST_ACCOUNT", "Account");
  req("SNOWFLAKE_TEST_USER", "Uid");
  if (params.count("SNOWFLAKE_TEST_WAREHOUSE_ODBC")) {
    add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_WAREHOUSE_ODBC", "Warehouse");
  } else {
    add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_WAREHOUSE", "Warehouse");
  }
  opt("SNOWFLAKE_TEST_ROLE", "Role");
  opt("SNOWFLAKE_TEST_SCHEMA", "Schema");
  opt("SNOWFLAKE_TEST_DATABASE", "Database");
  opt("SNOWFLAKE_TEST_PORT", "Port");
  opt("SNOWFLAKE_TEST_PROTOCOL", "Protocol");

  ss << "Authenticator=SNOWFLAKE_JWT;";
#ifdef SNOWFLAKE_OLD_DRIVER
  ss << "Priv_Key_File=" << get_or_create_private_key_file(params) << ";";
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", "Priv_Key_File_Pwd");
#else
  ss << "Priv_Key_Base64=" << test_utils::base64_encode(read_private_key(params)) << ";";
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", "Priv_Key_Pwd");
#endif
  return ss.str();
}

TEST_CASE("connection string keys are case-insensitive (lowercase)", "[session][case_sensitivity]") {
  SKIP_OLD_DRIVER("", "Old driver may not support all lowercase keys");
  // Given Snowflake ODBC connection string uses all-lowercase key names
  Connection conn(get_lowercase_connection_string());

  // When Connection is established and "SELECT 1" is executed
  auto stmt = conn.execute_fetch("SELECT 1");

  // Then the query should succeed and return 1
  auto value = get_data<SQL_C_LONG>(stmt, 1);
  CHECK(value == 1);
}

TEST_CASE("connection string keys are case-insensitive (mixed case)", "[session][case_sensitivity]") {
  SKIP_OLD_DRIVER("", "Old driver may not support mixed-case keys");
  // Given Snowflake ODBC connection string uses mixed-case key names
  Connection conn(get_mixed_case_connection_string());

  // When Connection is established and "SELECT 1" is executed
  auto stmt = conn.execute_fetch("SELECT 1");

  // Then the query should succeed and return 1
  auto value = get_data<SQL_C_LONG>(stmt, 1);
  CHECK(value == 1);
}
