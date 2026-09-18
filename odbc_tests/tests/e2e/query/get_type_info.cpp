#include <sql.h>
#include <sqlext.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "odbc_matchers.hpp"
#include "snowflake_odbc_constants.hpp"
#include "test_setup.hpp"

static SQLINTEGER type_info_column_size(StatementHandleWrapper& stmt, SQLSMALLINT sqlType) {
  SQLRETURN ret = SQLGetTypeInfo(stmt.getHandle(), sqlType);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER columnSize = 0x7FFFFFFF;
  SQLLEN indicator = -1;
  ret = SQLGetData(stmt.getHandle(), 3, SQL_C_SLONG, &columnSize, sizeof(columnSize), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator != SQL_NULL_DATA);
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  return columnSize;
}

TEST_CASE("should report TIMESTAMP COLUMN_SIZE 29 and vendor timestamp types 35", "[query][gettypeinfo]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLGetTypeInfo is called for SQL_TYPE_TIMESTAMP
  // Then COLUMN_SIZE is 29 on the new driver and 35 on the old driver
  NEW_DRIVER_ONLY("BD#151") { CHECK(type_info_column_size(stmt, SQL_TYPE_TIMESTAMP) == 29); }
  OLD_DRIVER_ONLY("BD#151") { CHECK(type_info_column_size(stmt, SQL_TYPE_TIMESTAMP) == 35); }

  // And TIMESTAMP_LTZ, TIMESTAMP_NTZ, and TIMESTAMP_TZ COLUMN_SIZE stay 35
  CHECK(type_info_column_size(stmt, SQL_SF_TIMESTAMP_LTZ) == 35);
  CHECK(type_info_column_size(stmt, SQL_SF_TIMESTAMP_NTZ) == 35);
  CHECK(type_info_column_size(stmt, SQL_SF_TIMESTAMP_TZ) == 35);
}
