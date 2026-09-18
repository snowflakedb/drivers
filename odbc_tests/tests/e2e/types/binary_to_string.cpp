#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"

TEST_CASE("should encode binary as HEX string by default", "[datatype][binary][conversion][string]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'0123456789ABCDEF' AS bin" is executed
  const auto stmt = conn.execute_fetch("SELECT X'0123456789ABCDEF' AS bin");

  // And bin is converted to string representation
  const std::string hex = get_data<SQL_C_CHAR>(stmt, 1);

  // Then the string should be '0123456789ABCDEF'
  REQUIRE(hex == "0123456789ABCDEF");
}
