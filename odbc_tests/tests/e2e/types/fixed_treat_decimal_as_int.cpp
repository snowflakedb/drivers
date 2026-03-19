#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "TestTable.hpp"
#include "get_data.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("TREAT_DECIMAL_AS_INT SQL_C_DEFAULT resolves to SBIGINT for scale=0", "[fixed][treat_decimal_as_int]") {
  // Given A Snowflake connection with ODBC_TREAT_DECIMAL_AS_INT=true
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  auto random_schema = Schema::use_random_schema(conn);

  // When DECIMAL values with scale=0 are fetched via SQL_C_DEFAULT
  (void)0;
  // Then SQL_C_DEFAULT resolves to SBIGINT and returns the correct value
  {
    INFO("positive integer");
    auto stmt = conn.execute_fetch("SELECT 42::DECIMAL(10,0)");

    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 42);
  }

  {
    INFO("negative integer");
    auto stmt = conn.execute_fetch("SELECT -123::DECIMAL(10,0)");

    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == -123);
  }

  {
    INFO("zero");
    auto stmt = conn.execute_fetch("SELECT 0::DECIMAL(10,0)");

    SQLBIGINT value = -1;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 0);
  }

  {
    INFO("max precision 18");
    auto stmt = conn.execute_fetch("SELECT 999999999999999999::DECIMAL(18,0)");

    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 999999999999999999LL);
  }
}

TEST_CASE("TREAT_DECIMAL_AS_INT does not affect scale > 0", "[fixed][treat_decimal_as_int]") {
  // Given A Snowflake connection with ODBC_TREAT_DECIMAL_AS_INT=true
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  auto random_schema = Schema::use_random_schema(conn);

  // When A DECIMAL(10,2) value is fetched via SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT 123.45::DECIMAL(10,2)");

  char buffer[100];
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);

  // Then SQL_C_DEFAULT still resolves to SQL_C_CHAR for scale > 0
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  CHECK(std::string(buffer, indicator) == "123.45");
}

TEST_CASE("TREAT_DECIMAL_AS_INT applies to precision > 18 too", "[fixed][treat_decimal_as_int]") {
  // Given A Snowflake connection with ODBC_TREAT_DECIMAL_AS_INT=true
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  auto random_schema = Schema::use_random_schema(conn);

  // When A NUMBER(38,0) value is fetched via SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT 42::NUMBER(38,0)");

  SQLBIGINT value = 0;
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);

  // Then SQL_C_DEFAULT resolves to SBIGINT even for precision > 18
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQLBIGINT));
  CHECK(value == 42);
}

TEST_CASE("TREAT_BIG_NUMBER_AS_STRING overrides TREAT_DECIMAL_AS_INT for precision > 18",
          "[fixed][treat_big_number_as_string]") {
  // Given A Snowflake connection with both TREAT_DECIMAL_AS_INT and TREAT_BIG_NUMBER_AS_STRING
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  conn.execute("ALTER SESSION SET ODBC_TREAT_BIG_NUMBER_AS_STRING=true");
  auto random_schema = Schema::use_random_schema(conn);

  // When NUMBER(38,0) and DECIMAL(10,0) values are fetched via SQL_C_DEFAULT
  (void)0;
  // Then NUMBER(38,0) resolves to SQL_C_CHAR and DECIMAL(10,0) still resolves to SBIGINT
  {
    INFO("NUMBER(38,0) resolves to SQL_C_CHAR when both settings are true");
    auto stmt = conn.execute_fetch("SELECT 42::NUMBER(38,0)");

    char buffer[100];
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
    CHECK(std::string(buffer, indicator) == "42");
  }

  {
    INFO("DECIMAL(10,0) still resolves to SBIGINT (precision <= 18)");
    auto stmt = conn.execute_fetch("SELECT 42::DECIMAL(10,0)");

    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 42);
  }
}

TEST_CASE("TREAT_DECIMAL_AS_INT with table columns", "[fixed][treat_decimal_as_int]") {
  // Given A Snowflake connection with ODBC_TREAT_DECIMAL_AS_INT=true and a table with mixed columns
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  auto random_schema = Schema::use_random_schema(conn);

  std::string table_name = "test_decimal_as_int";
  TestTable table(conn, table_name, "d_int DECIMAL(10,0), d_frac DECIMAL(10,2), d_big NUMBER(38,0)",
                  "(42, 123.45, 42)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  // When Each column is fetched via SQL_C_DEFAULT
  (void)0;
  // Then DECIMAL(10,0) and NUMBER(38,0) resolve to SBIGINT while DECIMAL(10,2) resolves to CHAR
  {
    INFO("DECIMAL(10,0) column returns SBIGINT via SQL_C_DEFAULT");
    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 42);
  }

  {
    INFO("DECIMAL(10,2) column returns CHAR via SQL_C_DEFAULT");
    char buffer[100];
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 2, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
    CHECK(std::string(buffer, indicator) == "123.45");
  }

  {
    INFO("NUMBER(38,0) column returns SBIGINT via SQL_C_DEFAULT (BigInt, precision > 18 but no big_number_as_string)");
    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 3, SQL_C_DEFAULT, &value, sizeof(value), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 42);
  }
}
