#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"

TEST_CASE("should bind SQL_C_BINARY to SQL_VARCHAR.", "[query][bind_parameter][c_binary_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  unsigned char param[] = {0xDE, 0xAD, 0xBE, 0xEF};
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARCHAR, 100, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  NEW_DRIVER_ONLY("BD#34") {
    REQUIRE_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    // Then the result should be the expected string
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "deadbeef");
  }
  OLD_DRIVER_ONLY("BD#34") { CHECK(ret == SQL_ERROR); }
}
