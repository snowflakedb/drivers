#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "test_setup.hpp"

// Builds a connection string with SERVER but without ACCOUNT.
// The driver should extract the account from the SERVER hostname.
static std::string get_connection_string_without_account() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params, {"ACCOUNT"});
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
#ifdef SNOWFLAKE_OLD_DRIVER
  ss << "PRIV_KEY_FILE=" << get_or_create_private_key_file(params) << ";";
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", "PRIV_KEY_FILE_PWD");
#else
  ss << "PRIV_KEY_BASE64=" << test_utils::base64_encode(read_private_key(params)) << ";";
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", "PRIV_KEY_PWD");
#endif
  return ss.str();
}

TEST_CASE("Connect with SERVER only, no ACCOUNT parameter", "[session][account]") {
  auto conn_str = get_connection_string_without_account();

  // Should succeed — driver extracts account from SERVER hostname
  Connection conn(conn_str);

  // Verify the connection is functional
  auto stmt = conn.execute_fetch("SELECT 1");
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(value == "1");
}

TEST_CASE("Connect with SERVER only derives correct account", "[session][account]") {
  auto conn_str = get_connection_string_without_account();

  Connection conn(conn_str);

  // Verify the account was correctly derived from SERVER
  auto stmt = conn.execute_fetch("SELECT CURRENT_ACCOUNT()");
  auto actual_account = get_data<SQL_C_CHAR>(stmt, 1);
  REQUIRE(!actual_account.empty());

  // Also connect with explicit ACCOUNT and compare
  Connection ref_conn(get_connection_string());
  auto ref_stmt = ref_conn.execute_fetch("SELECT CURRENT_ACCOUNT()");
  auto expected_account = get_data<SQL_C_CHAR>(ref_stmt, 1);

  CHECK(actual_account == expected_account);
}
