#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"

TEST_CASE("Test integer single column, single row binding", "[bindings_tests]") {
  Connection conn;
  conn.execute("DROP TABLE IF EXISTS universal_driver_odbc_small_binding_integer_test_table");
  conn.execute("CREATE TABLE universal_driver_odbc_small_binding_integer_test_table (id NUMBER)");

  {
    auto stmt = conn.createStatement();

    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO universal_driver_odbc_small_binding_integer_test_table (id) VALUES (?)", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLINTEGER value = 1;
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0,
                           &value, 0, NULL);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
  }

  {
    auto stmt = conn.createStatement();

    SQLRETURN ret = SQLExecDirect(
        stmt.getHandle(),
        (SQLCHAR*)"SELECT * FROM universal_driver_odbc_small_binding_integer_test_table", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    SQLINTEGER result = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(result == 1);
  }
}
