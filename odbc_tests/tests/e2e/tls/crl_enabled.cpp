#include <catch2/catch_test_macros.hpp>

#include "../../../common/include/Connection.hpp"

// Scenario: Should connect and select with CRL enabled
TEST_CASE("Should connect and select with CRL enabled") {
  // Given Snowflake client is logged in
  auto connection_string = get_connection_string();
  // And CRL is enabled
  connection_string += "CRL_MODE=ENABLED;";

  // When Query "SELECT 1" is executed
  Connection conn(connection_string);
  auto stmt = conn.execute_fetch("SELECT 1");

  // Then the request attempt should be successful
  SQLLEN value = 0;
  SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &value, 0, nullptr);
  REQUIRE(value == 1);
}
