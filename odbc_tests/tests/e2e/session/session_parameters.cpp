#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "test_setup.hpp"

TEST_CASE("should forward unrecognized connection option as session parameter", "[session]") {
  SKIP_OLD_DRIVER("", "Old driver does not forward unknown connection options as session parameters");
  // Given Snowflake client is logged in with connection option TIMEZONE set to
  // "Europe/Warsaw"
  auto conn_str = get_connection_string() + "TIMEZONE=Europe/Warsaw;";
  Connection conn(conn_str);

  // When Query "SHOW PARAMETERS LIKE 'TIMEZONE'" is executed
  auto stmt = conn.execute_fetch("SHOW PARAMETERS LIKE 'TIMEZONE'");

  // Then the session parameter value should be "Europe/Warsaw"
  auto value = get_data<SQL_C_CHAR>(stmt, 2);  // SHOW PARAMETERS: value is column 2
  CHECK(value == "Europe/Warsaw");
}

TEST_CASE("should enable session keep-alive via connection string", "[session]") {
  // Given Snowflake client is logged in with connection option CLIENT_SESSION_KEEP_ALIVE set to "true"
  auto conn_str = get_connection_string() + "CLIENT_SESSION_KEEP_ALIVE=true;";
  Connection conn(conn_str);

  // When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'" is executed
  auto stmt = conn.execute_fetch("SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'");

  // Then the session parameter value should be "true"
  auto value = get_data<SQL_C_CHAR>(stmt, 2);
  CHECK(value == "true");
}

TEST_CASE("should set heartbeat frequency via connection string", "[session]") {
  // Given Snowflake client is logged in with CLIENT_SESSION_KEEP_ALIVE=true and
  // CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY=1800
  auto conn_str = get_connection_string() +
                  "CLIENT_SESSION_KEEP_ALIVE=true;"
                  "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY=1800;";
  Connection conn(conn_str);

  // When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY'" is executed
  auto stmt = conn.execute_fetch("SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY'");

  // Then the session parameter value reflects the configured frequency
  auto value = get_data<SQL_C_CHAR>(stmt, 2);
  NEW_DRIVER_ONLY("BD#57") { CHECK(value == "1800"); }
  OLD_DRIVER_ONLY("BD#57") { CHECK(value == "3600"); }
}
