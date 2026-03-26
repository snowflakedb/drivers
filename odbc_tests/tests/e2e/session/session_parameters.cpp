#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "test_setup.hpp"

TEST_CASE("should forward unrecognized connection option as session parameter", "[session]") {
  SKIP_OLD_DRIVER("", "Old driver does not forward unknown connection options as session parameters");
  // Given Snowflake client is logged in with connection option QUERY_TAG set to
  // "session_param_e2e_test"
  auto conn_str = get_connection_string() + "QUERY_TAG=session_param_e2e_test;";
  Connection conn(conn_str);

  // When Query "SELECT CURRENT_QUERY_TAG()" is executed
  auto stmt = conn.execute_fetch("SELECT CURRENT_QUERY_TAG()");

  // Then the result should contain value "session_param_e2e_test"
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(value == "session_param_e2e_test");
}
