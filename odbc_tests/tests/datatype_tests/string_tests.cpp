#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

TEST_CASE("Test string basic query", "[datatype][string]") {
  Connection conn;
  conn.execute("DROP TABLE IF EXISTS test_string_basic");
  conn.execute("CREATE TABLE test_string_basic (str_col VARCHAR(1000))");
  conn.execute("INSERT INTO test_string_basic (str_col) VALUES ('Hello World')");
  auto stmt = conn.createStatement();

  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT str_col FROM test_string_basic", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  char buffer[1000];
  SQLLEN indicator;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  REQUIRE(indicator > 0);

  REQUIRE(std::string(buffer, indicator) == "Hello World");
}

TEST_CASE("Test basic string binding", "[datatype][string]") {
  Connection conn;
  conn.execute("DROP TABLE IF EXISTS test_string_basic_binding");
  conn.execute("CREATE TABLE test_string_basic_binding (str_col VARCHAR(1000))");
  auto stmt = conn.createStatement();

  // Prepare insert statement
  SQLRETURN ret =
      SQLPrepare(stmt.getHandle(),
                 (SQLCHAR*)"INSERT INTO test_string_basic_binding (str_col) VALUES (?)", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Test value to bind
  const char* test_value = "Hello World";
  SQLLEN str_len = strlen(test_value);

  // Bind the parameter
  ret = SQLBindParameter(stmt.getHandle(),
                         1,                       // Parameter number
                         SQL_PARAM_INPUT,         // Input parameter
                         SQL_C_CHAR,              // C data type
                         SQL_VARCHAR,             // SQL data type
                         str_len,                 // Column size
                         0,                       // Decimal digits
                         (SQLPOINTER)test_value,  // Parameter value ptr
                         str_len,                 // Buffer length
                         &str_len                 // Length/Indicator
  );
  CHECK_ODBC(ret, stmt);

  // Execute the prepared statement
  ret = SQLExecute(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Verify the inserted data
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT str_col FROM test_string_basic_binding",
                      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  char buffer[1000];
  SQLLEN indicator;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  REQUIRE(indicator > 0);

  REQUIRE(std::string(buffer, indicator) == "Hello World");
}
