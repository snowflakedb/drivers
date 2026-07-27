#include <sql.h>
#include <sqlext.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "test_setup.hpp"

TEST_CASE("should tag queries when QUERY_TAG is set at connection level", "[query]") {
  // Given Snowflake client is logged in with connection option QUERY_TAG set to "conn_tag_e2e"
  auto conn_str = get_connection_string() + "QUERY_TAG=conn_tag_e2e;";
  Connection conn(conn_str);

  // When Query "SELECT CURRENT_QUERY_TAG()" is executed
  auto stmt = conn.execute_fetch("SELECT CURRENT_QUERY_TAG()");

  // Then the result should contain value "conn_tag_e2e"
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  NEW_DRIVER_ONLY("BD#101") { CHECK(value == "conn_tag_e2e"); }
  OLD_DRIVER_ONLY("BD#101") { CHECK(value == ""); }
}
