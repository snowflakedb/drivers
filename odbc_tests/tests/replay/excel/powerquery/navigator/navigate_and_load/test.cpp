#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: excel powerquery navigator navigate_and_load", "[excel][powerquery][navigator]") {
  // TODO(SNOW-4039377): The new driver's SQLPrimaryKeys/SQLForeignKeys string IRD remains SQL_VARCHAR (12).
  // TODO(SNOW-4039378): The new driver's GEOGRAPHY SQLColumns sizes remain 16M/64M instead of the pinned 128M.
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  auto config = DataSourceConfig::Snowflake().install();

  SQLHENV env0 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env0);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env0 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env0, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  // SQLSetEnvAttr - SQL_ATTR_CONNECTION_POOLING
  {
    SQLRETURN ret = SQLSetEnvAttr(env0, SQL_ATTR_CONNECTION_POOLING, (SQLPOINTER)2, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  SQLHENV env1 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env1);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env1 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env1, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }

  // SQLSetEnvAttr - SQL_ATTR_CONNECTION_POOLING
  {
    SQLRETURN ret = SQLSetEnvAttr(env1, SQL_ATTR_CONNECTION_POOLING, (SQLPOINTER)2, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc0 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env1, &dbc0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
    REQUIRE(dbc0 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc0, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_IDENTIFIER_QUOTE_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_IDENTIFIER_QUOTE_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\"");
  }

  // SQLGetInfo - SQL_OWNER_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_OWNER_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15u);
  }

  // SQLGetInfo - SQL_CATALOG_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15u);
  }

  // SQLGetInfo - SQL_CATALOG_NAME_SEPARATOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_NAME_SEPARATOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == ".");
  }

  // SQLGetInfo - SQL_CATALOG_LOCATION
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_LOCATION, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_SQL_CONFORMANCE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SQL_CONFORMANCE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_MAX_COLUMNS_IN_ORDER_BY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_MAX_COLUMNS_IN_ORDER_BY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFFFu);
  }

  // SQLGetInfo - SQL_MAX_IDENTIFIER_LEN
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_MAX_IDENTIFIER_LEN, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFu);
  }

  // SQLGetInfo - SQL_MAX_COLUMNS_IN_GROUP_BY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_MAX_COLUMNS_IN_GROUP_BY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFFFu);
  }

  // SQLGetInfo - SQL_MAX_COLUMNS_IN_SELECT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_MAX_COLUMNS_IN_SELECT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFFFu);
  }

  // SQLGetInfo - SQL_ORDER_BY_COLUMNS_IN_SELECT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_ORDER_BY_COLUMNS_IN_SELECT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "N");
  }

  // SQLGetInfo - SQL_STRING_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_STRING_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFD7FFFu);
  }

  // SQLGetInfo - 169
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, 169, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_SQL92_PREDICATES
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SQL92_PREDICATES, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3F05u);
  }

  // SQLGetInfo - SQL_SQL92_RELATIONAL_JOIN_OPERATORS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SQL92_RELATIONAL_JOIN_OPERATORS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15Au);
  }

  // SQLGetInfo - SQL_SQL92_VALUE_EXPRESSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SQL92_VALUE_EXPRESSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFu);
  }

  // SQLGetInfo - SQL_COLUMN_ALIAS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_COLUMN_ALIAS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "Y");
  }

  // SQLGetInfo - SQL_GROUP_BY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_GROUP_BY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x2u);
  }

  // SQLGetInfo - SQL_NUMERIC_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_NUMERIC_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF7FFFFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_TIMEDATE_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFFFFu);
  }

  // SQLGetInfo - SQL_SYSTEM_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SYSTEM_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_TIMEDATE_ADD_INTERVALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_TIMEDATE_ADD_INTERVALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_DIFF_INTERVALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_TIMEDATE_DIFF_INTERVALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFu);
  }

  // SQLGetInfo - SQL_CONCAT_NULL_BEHAVIOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONCAT_NULL_BEHAVIOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_CATALOG_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "Y");
  }

  // SQLGetInfo - SQL_CATALOG_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "database");
  }

  // SQLGetInfo - SQL_OWNER_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_OWNER_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "schema");
  }

  // SQLGetInfo - SQL_ODBC_INTERFACE_CONFORMANCE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_ODBC_INTERFACE_CONFORMANCE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_SEARCH_PATTERN_ESCAPE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SEARCH_PATTERN_ESCAPE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\\");
  }

  // SQLGetInfo - SQL_CONVERT_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_CONVERT_BIGINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_BIGINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_BINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_BINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA00D01u);
  }

  // SQLGetInfo - SQL_CONVERT_BIT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_BIT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xE47FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA3EDFFu);
  }

  // SQLGetInfo - SQL_CONVERT_DECIMAL
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_DECIMAL, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_DOUBLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_DOUBLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_FLOAT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_FLOAT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - 173
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, 173, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_CONVERT_INTEGER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_INTEGER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_LONGVARBINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_LONGVARBINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xE40F01u);
  }

  // SQLGetInfo - SQL_CONVERT_LONGVARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_LONGVARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FBF3FFu);
  }

  // SQLGetInfo - SQL_CONVERT_NUMERIC
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_NUMERIC, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_REAL
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_REAL, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_SMALLINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_SMALLINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_TIMESTAMP
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_TIMESTAMP, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA38101u);
  }

  // SQLGetInfo - SQL_CONVERT_TINYINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_TINYINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_DATE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_DATE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA28101u);
  }

  // SQLGetInfo - SQL_CONVERT_TIME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_TIME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA10101u);
  }

  // SQLGetInfo - SQL_CONVERT_VARBINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_VARBINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA00D01u);
  }

  // SQLGetInfo - SQL_CONVERT_VARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_VARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA38DAFu);
  }

  // SQLGetInfo - SQL_CONVERT_WCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_WCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFFFFFu);
  }

  // SQLGetInfo - SQL_CONVERT_WLONGVARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_WLONGVARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FBF3FFu);
  }

  // SQLGetInfo - SQL_CONVERT_WVARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_WVARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFFFFFu);
  }

  // SQLGetInfo - SQL_SPECIAL_CHARACTERS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SPECIAL_CHARACTERS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - 180
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, 180, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsError());
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc1 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env1, &dbc1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
    REQUIRE(dbc1 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc1, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc1, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt0 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt0 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt0, sqlchar("%"), 1, nullptr, 0, nullptr, 0, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 5);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // account-wide SQLTables('%') catalog enumeration:
  // the row count and catalog names are a property of the
  // connected account, not the driver, so drain the cursor
  // structurally rather than pinning environment-specific rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt0)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      std::vector<char> buf(2048, static_cast<char>(0xFF));
      SQLLEN ind = 0;
      SQLRETURN g = SQLGetData(stmt0, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
      CHECK_THAT(OdbcResult(g, SQL_HANDLE_STMT, stmt0), OdbcMatchers::Succeeded());
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc2 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env1, &dbc2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
    REQUIRE(dbc2 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc2, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc2, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt1 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc2, &stmt1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    REQUIRE(stmt1 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt1, sqlchar("%"), 1, nullptr, 0, nullptr, 0, sqlchar("TABLE,VIEW"), 10);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt1, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numCols == 5);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // account-wide SQLTables('%') catalog enumeration:
  // the row count and catalog names are a property of the
  // connected account, not the driver, so drain the cursor
  // structurally rather than pinning environment-specific rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt1)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      std::vector<char> buf(2048, static_cast<char>(0xFF));
      SQLLEN ind = 0;
      SQLRETURN g = SQLGetData(stmt1, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
      CHECK_THAT(OdbcResult(g, SQL_HANDLE_STMT, stmt1), OdbcMatchers::Succeeded());
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  SQLHENV env2 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env2);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env2 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env2, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
  }

  // SQLSetEnvAttr - SQL_ATTR_CONNECTION_POOLING
  {
    SQLRETURN ret = SQLSetEnvAttr(env2, SQL_ATTR_CONNECTION_POOLING, (SQLPOINTER)2, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
  }

  SQLHENV env3 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env3);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env3 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env3, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
  }

  // SQLSetEnvAttr - SQL_ATTR_CONNECTION_POOLING
  {
    SQLRETURN ret = SQLSetEnvAttr(env3, SQL_ATTR_CONNECTION_POOLING, (SQLPOINTER)2, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc3 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc3 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc3, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc3, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_IDENTIFIER_QUOTE_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_IDENTIFIER_QUOTE_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\"");
  }

  // SQLGetInfo - SQL_OWNER_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_OWNER_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15u);
  }

  // SQLGetInfo - SQL_CATALOG_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CATALOG_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15u);
  }

  // SQLGetInfo - SQL_CATALOG_NAME_SEPARATOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CATALOG_NAME_SEPARATOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == ".");
  }

  // SQLGetInfo - SQL_CATALOG_LOCATION
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CATALOG_LOCATION, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_SQL_CONFORMANCE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SQL_CONFORMANCE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_MAX_COLUMNS_IN_ORDER_BY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_MAX_COLUMNS_IN_ORDER_BY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFFFu);
  }

  // SQLGetInfo - SQL_MAX_IDENTIFIER_LEN
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_MAX_IDENTIFIER_LEN, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFu);
  }

  // SQLGetInfo - SQL_MAX_COLUMNS_IN_GROUP_BY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_MAX_COLUMNS_IN_GROUP_BY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFFFu);
  }

  // SQLGetInfo - SQL_MAX_COLUMNS_IN_SELECT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_MAX_COLUMNS_IN_SELECT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFFFu);
  }

  // SQLGetInfo - SQL_ORDER_BY_COLUMNS_IN_SELECT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_ORDER_BY_COLUMNS_IN_SELECT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "N");
  }

  // SQLGetInfo - SQL_STRING_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_STRING_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFD7FFFu);
  }

  // SQLGetInfo - 169
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, 169, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_SQL92_PREDICATES
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SQL92_PREDICATES, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3F05u);
  }

  // SQLGetInfo - SQL_SQL92_RELATIONAL_JOIN_OPERATORS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SQL92_RELATIONAL_JOIN_OPERATORS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15Au);
  }

  // SQLGetInfo - SQL_SQL92_VALUE_EXPRESSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SQL92_VALUE_EXPRESSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFu);
  }

  // SQLGetInfo - SQL_COLUMN_ALIAS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_COLUMN_ALIAS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "Y");
  }

  // SQLGetInfo - SQL_GROUP_BY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_GROUP_BY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x2u);
  }

  // SQLGetInfo - SQL_NUMERIC_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_NUMERIC_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF7FFFFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_TIMEDATE_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFFFFu);
  }

  // SQLGetInfo - SQL_SYSTEM_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SYSTEM_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_TIMEDATE_ADD_INTERVALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_TIMEDATE_ADD_INTERVALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_DIFF_INTERVALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_TIMEDATE_DIFF_INTERVALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFu);
  }

  // SQLGetInfo - SQL_CONCAT_NULL_BEHAVIOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONCAT_NULL_BEHAVIOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_CATALOG_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CATALOG_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "Y");
  }

  // SQLGetInfo - SQL_CATALOG_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CATALOG_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "database");
  }

  // SQLGetInfo - SQL_OWNER_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_OWNER_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "schema");
  }

  // SQLGetInfo - SQL_ODBC_INTERFACE_CONFORMANCE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_ODBC_INTERFACE_CONFORMANCE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_SEARCH_PATTERN_ESCAPE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SEARCH_PATTERN_ESCAPE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\\");
  }

  // SQLGetInfo - SQL_CONVERT_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_CONVERT_BIGINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_BIGINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_BINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_BINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA00D01u);
  }

  // SQLGetInfo - SQL_CONVERT_BIT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_BIT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xE47FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA3EDFFu);
  }

  // SQLGetInfo - SQL_CONVERT_DECIMAL
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_DECIMAL, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_DOUBLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_DOUBLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_FLOAT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_FLOAT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - 173
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, 173, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_CONVERT_INTEGER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_INTEGER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_LONGVARBINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_LONGVARBINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xE40F01u);
  }

  // SQLGetInfo - SQL_CONVERT_LONGVARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_LONGVARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FBF3FFu);
  }

  // SQLGetInfo - SQL_CONVERT_NUMERIC
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_NUMERIC, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_REAL
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_REAL, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_SMALLINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_SMALLINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_TIMESTAMP
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_TIMESTAMP, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA38101u);
  }

  // SQLGetInfo - SQL_CONVERT_TINYINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_TINYINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_DATE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_DATE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA28101u);
  }

  // SQLGetInfo - SQL_CONVERT_TIME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_TIME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA10101u);
  }

  // SQLGetInfo - SQL_CONVERT_VARBINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_VARBINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA00D01u);
  }

  // SQLGetInfo - SQL_CONVERT_VARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_VARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA38DAFu);
  }

  // SQLGetInfo - SQL_CONVERT_WCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_WCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFFFFFu);
  }

  // SQLGetInfo - SQL_CONVERT_WLONGVARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_WLONGVARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FBF3FFu);
  }

  // SQLGetInfo - SQL_CONVERT_WVARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_WVARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFFFFFu);
  }

  // SQLGetInfo - SQL_SPECIAL_CHARACTERS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SPECIAL_CHARACTERS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - 180
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, 180, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsError());
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc4 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc4);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc4 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc4, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc4, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc4, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc4, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc4, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc4, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt2 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc4, &stmt2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
    REQUIRE(stmt2 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc4, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc4, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
  }

  // SQLColumns
  {
    SQLRETURN ret = SQLColumns(stmt2, sqlchar("ODBCMETADATATESTDB"), 18, sqlchar("DATATYPETESTS"), 13,
                               sqlchar("ALLDATATYPESNAV"), 15, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt2, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numCols == 19);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_INTEGER);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_INTEGER);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_INTEGER);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_INTEGER);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_INTEGER);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc4, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ROWKIND");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ROWKIND");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARCHAR");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARCHAR");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(64));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(64));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(1));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"INTVAL");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"INTVAL");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DECIMAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DECIMAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16)); }
    NEW_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(40)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(2));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BIGINTVAL");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BIGINTVAL");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DECIMAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DECIMAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16)); }
    NEW_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(40)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(3));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"SMALLINTVAL");
      CHECK(ind == 22);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"SMALLINTVAL");
      CHECK(ind == 44);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DECIMAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DECIMAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16)); }
    NEW_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(40)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(4));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TINYINTVAL");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TINYINTVAL");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DECIMAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DECIMAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16)); }
    NEW_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(40)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(5));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"NUM38");
      CHECK(ind == 10);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"NUM38");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DECIMAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DECIMAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16)); }
    NEW_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(40)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(6));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"NUM18S6");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"NUM18S6");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DECIMAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DECIMAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(18));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(8)); }
    NEW_DRIVER_ONLY("BD#122") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(20)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(6));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(7));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"FLOATVAL");
      CHECK(ind == 16);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"FLOATVAL");
      CHECK(ind == 32);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(8));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DOUBLE");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DOUBLE");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#123") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38)); }
    NEW_DRIVER_ONLY("BD#123") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(15)); }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(8));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(8));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(8));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DOUBLEVAL");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DOUBLEVAL");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(8));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DOUBLE");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DOUBLE");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#123") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38)); }
    NEW_DRIVER_ONLY("BD#123") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(15)); }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(8));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(8));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(9));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"REALVAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"REALVAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(8));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DOUBLE");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DOUBLE");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#123") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38)); }
    NEW_DRIVER_ONLY("BD#123") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(15)); }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(8));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(8));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARCHARVAL");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARCHARVAL");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARCHAR");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARCHAR");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(256));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(1024));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(1024));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(11));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TEXTVAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TEXTVAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARCHAR");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARCHAR");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(67108864));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(67108864));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(12));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"CHARVAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"CHARVAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARCHAR");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARCHAR");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(40));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(40));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(13));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BINARYVAL");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BINARYVAL");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-2));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BINARY");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BINARY");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-2));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(14));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARBINARYVAL");
      CHECK(ind == 24);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARBINARYVAL");
      CHECK(ind == 48);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-2));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BINARY");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BINARY");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(8388608));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(8388608));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-2));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(8388608));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(15));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BOOLVAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BOOLVAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-7));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BOOLEAN");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BOOLEAN");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(1));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-7));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATEVAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATEVAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(91));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATE");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATE");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#133") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10)); }
    NEW_DRIVER_ONLY("BD#133") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(6)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(91)); }
    NEW_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9)); }
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(17));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIMEVAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIMEVAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(92));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIME");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIME");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(18));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#133") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(18)); }
    NEW_DRIVER_ONLY("BD#133") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(6)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(92)); }
    NEW_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9)); }
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(18));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TSNTZ");
      CHECK(ind == 10);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TSNTZ");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(93));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIMESTAMP");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIMESTAMP");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#128") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35)); }
    NEW_DRIVER_ONLY("BD#128") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(29)); }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#129") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35)); }
    NEW_DRIVER_ONLY("BD#129") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(93)); }
    NEW_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9)); }
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(19));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TSLTZ");
      CHECK(ind == 10);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TSLTZ");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(93));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIMESTAMP");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIMESTAMP");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#128") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35)); }
    NEW_DRIVER_ONLY("BD#128") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(29)); }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#129") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35)); }
    NEW_DRIVER_ONLY("BD#129") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(93)); }
    NEW_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9)); }
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(20));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TSTZ");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TSTZ");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(93));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIMESTAMP");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIMESTAMP");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#128") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35)); }
    NEW_DRIVER_ONLY("BD#128") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(29)); }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#129") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35)); }
    NEW_DRIVER_ONLY("BD#129") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(93)); }
    NEW_DRIVER_ONLY("BD#125") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9)); }
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(21));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARIANTVAL");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARIANTVAL");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARIANT");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARIANT");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
    NEW_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
    }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
    NEW_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
    }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
    NEW_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
    }
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(22));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"OBJECTVAL");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"OBJECTVAL");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"STRUCT");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"STRUCT");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
    NEW_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
    }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
    NEW_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
    }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
    NEW_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
    }
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(23));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ARRAYVAL");
      CHECK(ind == 16);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ARRAYVAL");
      CHECK(ind == 32);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ARRAY");
      CHECK(ind == 10);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ARRAY");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
    NEW_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
    }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
    NEW_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
    }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
    NEW_DRIVER_ONLY("BD#130") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(16777216));
    }
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(24));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ODBCMETADATATESTDB");
      CHECK(ind == 36);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ODBCMETADATATESTDB");
      CHECK(ind == 72);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATATYPETESTS");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATATYPETESTS");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ALLDATATYPESNAV");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ALLDATATYPESNAV");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"GEOVAL");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"GEOVAL");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 5, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"GEOGRAPHY");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"GEOGRAPHY");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 7, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 8, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 10, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#121") { CHECK(ind == 0); }
    NEW_DRIVER_ONLY("BD#121") { CHECK(ind == SQL_NULL_DATA); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 16, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 17, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(25));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 18, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YES");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YES");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt2, 19, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsNoData());
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc5 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc5);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc5 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc5, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc5, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc5, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc5, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc5, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc5, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt3 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc5, &stmt3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
    REQUIRE(stmt3 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc5, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc5, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
  }

  // SQLGetTypeInfo
  {
    SQLRETURN ret = SQLGetTypeInfo(stmt3, SQL_ALL_TYPES);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt3, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numCols == 20);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_INTEGER);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_INTEGER);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc5, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"CHAR");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"CHAR");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"LENGTH");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"LENGTH");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"CHAR");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"CHAR");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"NUMERIC");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"NUMERIC");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"precision,scale");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"precision,scale");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"NUMERIC");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"NUMERIC");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(8192));
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DECIMAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DECIMAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(38));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"precision,scale");
      CHECK(ind == 30);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"precision,scale");
      CHECK(ind == 60);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DECIMAL");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DECIMAL");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(38));
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"INTEGER");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"INTEGER");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(4));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"INTEGER");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"INTEGER");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(4));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(2));
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BIGINT");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BIGINT");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-5));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(19));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BIGINT");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BIGINT");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-5));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(2));
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"FLOAT");
      CHECK(ind == 10);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"FLOAT");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(6));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(15));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"FLOAT");
      CHECK(ind == 10);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"FLOAT");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(6));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(2));
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"REAL");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"REAL");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(7));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(7));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"REAL");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"REAL");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(7));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(2));
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DOUBLE");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DOUBLE");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(8));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(15));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DOUBLE");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DOUBLE");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(8));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(2));
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARCHAR");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARCHAR");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"max length");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"max length");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARCHAR");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARCHAR");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(12));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BINARY");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BINARY");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-2));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(67108864));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"0x");
      CHECK(ind == 4);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"0x");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 0);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"LENGTH");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"LENGTH");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BINARY");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BINARY");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-2));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARBINARY");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARBINARY");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-3));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(67108864));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"0x");
      CHECK(ind == 4);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"0x");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 0);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"max length");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"max length");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARBINARY");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARBINARY");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-3));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"DATE");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"DATE");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(91));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(10));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TYPE_DATE");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TYPE_DATE");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIME");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIME");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(92));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(18));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TYPE_TIME");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TYPE_TIME");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIMESTAMP_LTZ");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIMESTAMP_LTZ");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2000));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"STAMP_LTZ");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"STAMP_LTZ");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2000));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIMESTAMP_NTZ");
      CHECK(ind == 26);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIMESTAMP_NTZ");
      CHECK(ind == 52);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2002));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"STAMP_NTZ");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"STAMP_NTZ");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2002));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIMESTAMP_TZ");
      CHECK(ind == 24);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIMESTAMP_TZ");
      CHECK(ind == 48);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2001));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"STAMP_TZ");
      CHECK(ind == 16);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"STAMP_TZ");
      CHECK(ind == 32);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2001));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TIMESTAMP");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TIMESTAMP");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(93));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(35));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"TYPE_TIMESTAMP");
      CHECK(ind == 28);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"TYPE_TIMESTAMP");
      CHECK(ind == 56);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(9));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"ARRAY");
      CHECK(ind == 10);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"ARRAY");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2003));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"max length");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"max length");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"OWN");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"OWN");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2003));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"OBJECT");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"OBJECT");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2004));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"max length");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"max length");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"OWN");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"OWN");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2004));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"VARIANT");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARIANT");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2005));
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"max length");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"max length");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"OWN");
      CHECK(ind == 6);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"OWN");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2005));
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      OLD_DRIVER_ONLY("BD#119") { CHECK(actual == u"CHAR"); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(actual == u"VECTOR"); }
      OLD_DRIVER_ONLY("BD#119") { CHECK(ind == 8); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(ind == 12); }
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"CHAR");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-8)); }
    NEW_DRIVER_ONLY("BD#119") {
      CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2006));
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      OLD_DRIVER_ONLY("BD#119") { CHECK(actual == u"LENGTH"); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(actual == u"max length"); }
      OLD_DRIVER_ONLY("BD#119") { CHECK(ind == 12); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(ind == 20); }
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"LENGTH");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1)); }
    NEW_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      OLD_DRIVER_ONLY("BD#119") { CHECK(actual == u"WCHAR"); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(actual == u"OWN"); }
      OLD_DRIVER_ONLY("BD#119") { CHECK(ind == 10); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(ind == 6); }
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"WCHAR");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-8)); }
    NEW_DRIVER_ONLY("BD#119") {
      CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2006));
    }
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      OLD_DRIVER_ONLY("BD#119") { CHECK(actual == u"VARCHAR"); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(actual == u"CHAR"); }
      OLD_DRIVER_ONLY("BD#119") { CHECK(ind == 14); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(ind == 8); }
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"VARCHAR");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-9)); }
    NEW_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-8)); }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"'");
      CHECK(ind == 2);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"'");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"LENGTH");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"LENGTH");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      OLD_DRIVER_ONLY("BD#119") { CHECK(actual == u"WVARCHAR"); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(actual == u"WCHAR"); }
      OLD_DRIVER_ONLY("BD#119") { CHECK(ind == 16); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(ind == 10); }
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"WVARCHAR");
      CHECK(ind == 32);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-9)); }
    NEW_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-8)); }
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      OLD_DRIVER_ONLY("BD#119") { CHECK(actual == u"BOOLEAN"); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(actual == u"VARCHAR"); }
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BOOLEAN");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 2, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-7)); }
    NEW_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-9)); }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 3, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(1)); }
    NEW_DRIVER_ONLY("BD#119") {
      CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(134217728));
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#119") { CHECK(ind == SQL_NULL_DATA); }
    NEW_DRIVER_ONLY("BD#119") { CHECK(ind == 2); }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#119") { CHECK(ind == SQL_NULL_DATA); }
    NEW_DRIVER_ONLY("BD#119") { CHECK(ind == 2); }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    OLD_DRIVER_ONLY("BD#119") { CHECK(ind == SQL_NULL_DATA); }
    NEW_DRIVER_ONLY("BD#119") { CHECK(ind == 12); }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 7, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1));
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 8, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0)); }
    NEW_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(1)); }
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 9, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(2)); }
    NEW_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(3)); }
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 10, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 11, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(0));
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 12, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      OLD_DRIVER_ONLY("BD#119") { CHECK(actual == u"BIT"); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(actual == u"WVARCHAR"); }
      OLD_DRIVER_ONLY("BD#119") { CHECK(ind == 6); }
      NEW_DRIVER_ONLY("BD#119") { CHECK(ind == 16); }
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BIT");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 14, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 15, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 16, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    OLD_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-7)); }
    NEW_DRIVER_ONLY("BD#119") { CHECK((*reinterpret_cast<SQLSMALLINT*>(buf.data())) == static_cast<SQLSMALLINT>(-9)); }
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 17, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 18, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 19, SQL_C_SSHORT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt3, 20, SQL_C_SLONG, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    CHECK((*reinterpret_cast<SQLINTEGER*>(buf.data())) == static_cast<SQLINTEGER>(0));
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    OLD_DRIVER_ONLY("BD#119") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsNoData()); }
    NEW_DRIVER_ONLY("BD#119") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess()); }
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc6 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc6);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc6 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc6, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc6, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc6, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc6, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc6, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc6, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt4 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc6, &stmt4);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
    REQUIRE(stmt4 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc6, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc6, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
  }

  // SQLPrimaryKeys
  {
    SQLRETURN ret = SQLPrimaryKeys(stmt4, sqlchar("ODBCMETADATATESTDB"), 18, sqlchar("DATATYPETESTS"), 13,
                                   sqlchar("ALLDATATYPESNAV"), 15);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt4, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numCols == 6);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc6, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsNoData());
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc6), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc7 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc7);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc7 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc7, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc7, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc7, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc7, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc7, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc7, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt5 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc7, &stmt5);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
    REQUIRE(stmt5 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc7, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc7, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
  }

  // SQLForeignKeys
  {
    SQLRETURN ret = SQLForeignKeys(stmt5, nullptr, 0, nullptr, 0, nullptr, 0, sqlchar("ODBCMETADATATESTDB"), 18,
                                   sqlchar("DATATYPETESTS"), 13, sqlchar("ALLDATATYPESNAV"), 15);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt5, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numCols == 14);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc7, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt5, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt5, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsNoData());
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc7), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc8 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc8);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc8 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc8, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc8, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc8, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc8, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc8, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc8, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt6 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc8, &stmt6);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
    REQUIRE(stmt6 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc8, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc8, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
  }

  // SQLForeignKeys
  {
    SQLRETURN ret = SQLForeignKeys(stmt6, sqlchar("ODBCMETADATATESTDB"), 18, sqlchar("DATATYPETESTS"), 13,
                                   sqlchar("ALLDATATYPESNAV"), 15, nullptr, 0, nullptr, 0, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt6, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numCols == 14);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc8, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt6, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt6, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsNoData());
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc8), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc9 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc9);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc9 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc9, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc9, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc9, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc9, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc9, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc9, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt7 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc9, &stmt7);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
    REQUIRE(stmt7 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc9, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc9, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(
        stmt7,
        sqlchar("select \"ROWKIND\",\r\n    \"INTVAL\",\r\n    \"BIGINTVAL\",\r\n    \"SMALLINTVAL\",\r\n    "
                "\"TINYINTVAL\",\r\n    \"NUM38\",\r\n    \"NUM18S6\",\r\n    \"FLOATVAL\",\r\n    \"DOUBLEVAL\",\r\n  "
                "  \"REALVAL\",\r\n    \"VARCHARVAL\",\r\n    \"TEXTVAL\",\r\n    \"CHARVAL\",\r\n    \"BOOLVAL\",\r\n "
                "   \"DATEVAL\",\r\n    \"TIMEVAL\",\r\n    \"TSNTZ\",\r\n    \"TSLTZ\",\r\n    \"TSTZ\",\r\n    "
                "\"VARIANTVAL\",\r\n    \"OBJECTVAL\",\r\n    \"ARRAYVAL\",\r\n    \"GEOVAL\"\r\nfrom "
                "\"ODBCMETADATATESTDB\".\"DATATYPETESTS\".\"ALLDATATYPESNAV\""),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt7, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numCols == 23);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -7);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_DATE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIME);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc9, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt7, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"NORMAL");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"NORMAL");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"42");
      CHECK(ind == 4);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"42");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"100000000000");
      CHECK(ind == 24);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"100000000000");
      CHECK(ind == 48);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"1000");
      CHECK(ind == 8);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"1000");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"100");
      CHECK(ind == 6);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"100");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"123456789012345678901234567890");
      CHECK(ind == 60);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"123456789012345678901234567890");
      CHECK(ind == 120);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"12345.678901");
      CHECK(ind == 24);
    }
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"12345.678901");
      CHECK(ind == 24);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"12345.678901");
      CHECK(ind == 48);
    }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 3.14);
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 2.718281828459045);
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 1.4142135);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"hello world");
      CHECK(ind == 22);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"hello world");
      CHECK(ind == 44);
    }
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"representative text payload");
      CHECK(ind == 54);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"representative text payload");
      CHECK(ind == 108);
    }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"fixedchar");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"fixedchar");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 14, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 1);
    CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == true);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 15, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());
    CHECK(_ds->year == 2024);
    CHECK(_ds->month == 1);
    CHECK(_ds->day == 15);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 16, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());
    CHECK(_ts->hour == 13);
    CHECK(_ts->minute == 45);
    CHECK(_ts->second == 30);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 17, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 1);
    CHECK(_ts->day == 15);
    CHECK(_ts->hour == 13);
    CHECK(_ts->minute == 45);
    CHECK(_ts->second == 30);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 18, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 1);
    CHECK(_ts->day == 14);
    CHECK(_ts->hour == 21);
    CHECK(_ts->minute == 45);
    CHECK(_ts->second == 30);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 1);
    CHECK(_ts->day == 14);
    CHECK(_ts->hour == 21);
    CHECK(_ts->minute == 45);
    CHECK(_ts->second == 30);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 20, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"a\": 1\n}");
      CHECK(ind == 24);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"a\": 1\n}");
      CHECK(ind == 48);
    }
  }

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 21, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"k\": \"v\"\n}");
      CHECK(ind == 28);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"k\": \"v\"\n}");
      CHECK(ind == 56);
    }
  }

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"[\n  1,\n  2,\n  3\n]");
      CHECK(ind == 34);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"[\n  1,\n  2,\n  3\n]");
      CHECK(ind == 68);
    }
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"coordinates\": [\n    -122,\n    37\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 124);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"coordinates\": [\n    -122,\n    37\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 248);
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BOUNDARY");
      CHECK(ind == 16);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BOUNDARY");
      CHECK(ind == 32);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"2147483647");
      CHECK(ind == 20);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"2147483647");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"9223372036854775807");
      CHECK(ind == 38);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"9223372036854775807");
      CHECK(ind == 76);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"32767");
      CHECK(ind == 10);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"32767");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"127");
      CHECK(ind == 6);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"127");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"99999999999999999999999999999999999999");
      CHECK(ind == 76);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"99999999999999999999999999999999999999");
      CHECK(ind == 152);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"999999999999.999999");
      CHECK(ind == 38);
    }
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"999999999999.999999");
      CHECK(ind == 38);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"999999999999.999999");
      CHECK(ind == 76);
    }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK(
        (std::isinf(*reinterpret_cast<double*>(buf.data())) && !std::signbit(*reinterpret_cast<double*>(buf.data()))));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK(std::isnan(*reinterpret_cast<double*>(buf.data())));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((std::isinf(*reinterpret_cast<double*>(buf.data())) && std::signbit(*reinterpret_cast<double*>(buf.data()))));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(
          actual ==
          u"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
          u"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
          u"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
      CHECK(ind == 512);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(
          actual ==
          U"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
          U"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
          U"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
      CHECK(ind == 1024);
    }
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"boundary text payload");
      CHECK(ind == 42);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"boundary text payload");
      CHECK(ind == 84);
    }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YYYYYYYYYY");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YYYYYYYYYY");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 14, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 1);
    CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == false);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 15, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());
    CHECK(_ds->year == 9999);
    CHECK(_ds->month == 12);
    CHECK(_ds->day == 31);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 16, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == 6);
    const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());
    CHECK(_ts->hour == 23);
    CHECK(_ts->minute == 59);
    CHECK(_ts->second == 59);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 17, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 9999);
    CHECK(_ts->month == 12);
    CHECK(_ts->day == 31);
    CHECK(_ts->hour == 23);
    CHECK(_ts->minute == 59);
    CHECK(_ts->second == 59);
    CHECK(_ts->fraction == 999999999);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 18, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 9999);
    CHECK(_ts->month == 12);
    CHECK(_ts->day == 31);
    CHECK(_ts->hour == 7);
    CHECK(_ts->minute == 59);
    CHECK(_ts->second == 59);
    CHECK(_ts->fraction == 999999999);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 9999);
    CHECK(_ts->month == 12);
    CHECK(_ts->day == 30);
    CHECK(_ts->hour == 23);
    CHECK(_ts->minute == 59);
    CHECK(_ts->second == 59);
    CHECK(_ts->fraction == 999999999);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 20, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"null");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"null");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 21, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{}");
      CHECK(ind == 4);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{}");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"[]");
      CHECK(ind == 4);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"[]");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"coordinates\": [\n    180,\n    90\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 122);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"coordinates\": [\n    180,\n    90\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 244);
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"UNICODE");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"UNICODE");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"7");
      CHECK(ind == 2);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"7");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"8");
      CHECK(ind == 2);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"8");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"9");
      CHECK(ind == 2);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"9");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"1");
      CHECK(ind == 2);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"1");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"42");
      CHECK(ind == 4);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"42");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"3.141593");
      CHECK(ind == 16);
    }
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"3.141593");
      CHECK(ind == 16);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"3.141593");
      CHECK(ind == 32);
    }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 1);
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 2);
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 3);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    // The iODBC width is not 2x the unixODBC byte count here: the value
    // holds a surrogate pair, which is a single UTF-32 code unit.
    NON_IODBC { CHECK(ind == 90); }
    IODBC_ONLY { CHECK(ind == 176); }
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    // The iODBC width is not 2x the unixODBC byte count here: the value
    // holds a surrogate pair, which is a single UTF-32 code unit.
    NON_IODBC { CHECK(ind == 62); }
    IODBC_ONLY { CHECK(ind == 120); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 10); }
    IODBC_ONLY { CHECK(ind == 20); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 14, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 1);
    CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == true);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 15, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());
    CHECK(_ds->year == 2024);
    CHECK(_ds->month == 2);
    CHECK(_ds->day == 29);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 16, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());
    CHECK(_ts->hour == 12);
    CHECK(_ts->minute == 0);
    CHECK(_ts->second == 0);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 17, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 2);
    CHECK(_ts->day == 29);
    CHECK(_ts->hour == 12);
    CHECK(_ts->minute == 0);
    CHECK(_ts->second == 0);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 18, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 2);
    CHECK(_ts->day == 28);
    CHECK(_ts->hour == 20);
    CHECK(_ts->minute == 0);
    CHECK(_ts->second == 0);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 2);
    CHECK(_ts->day == 28);
    CHECK(_ts->hour == 3);
    CHECK(_ts->minute == 0);
    CHECK(_ts->second == 0);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 20, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    // The iODBC width is not 2x the unixODBC byte count here: the value
    // holds a surrogate pair, which is a single UTF-32 code unit.
    NON_IODBC { CHECK(ind == 68); }
    IODBC_ONLY { CHECK(ind == 132); }
  }

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 21, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 36); }
    IODBC_ONLY { CHECK(ind == 72); }
  }

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 46); }
    IODBC_ONLY { CHECK(ind == 92); }
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"coordinates\": [\n    139.6917,\n    35.6895\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 142);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"coordinates\": [\n    139.6917,\n    35.6895\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 284);
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"NULLROW");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"NULLROW");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 14, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 15, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 16, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 17, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 18, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 20, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 21, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt7, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsNoData());
  }

  SQLHDBC dbc10 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc10);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc10 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc10, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc10, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc10, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc10, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc10, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc10, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt8 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc10, &stmt8);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
    REQUIRE(stmt8 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc10, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc10, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(
        stmt8,
        sqlchar("select \"ROWKIND\",\r\n    \"INTVAL\",\r\n    \"BIGINTVAL\",\r\n    \"SMALLINTVAL\",\r\n    "
                "\"TINYINTVAL\",\r\n    \"NUM38\",\r\n    \"NUM18S6\",\r\n    \"FLOATVAL\",\r\n    \"DOUBLEVAL\",\r\n  "
                "  \"REALVAL\",\r\n    \"VARCHARVAL\",\r\n    \"TEXTVAL\",\r\n    \"CHARVAL\",\r\n    \"BOOLVAL\",\r\n "
                "   \"DATEVAL\",\r\n    \"TIMEVAL\",\r\n    \"TSNTZ\",\r\n    \"TSLTZ\",\r\n    \"TSTZ\",\r\n    "
                "\"VARIANTVAL\",\r\n    \"OBJECTVAL\",\r\n    \"ARRAYVAL\",\r\n    \"GEOVAL\"\r\nfrom "
                "\"ODBCMETADATATESTDB\".\"DATATYPETESTS\".\"ALLDATATYPESNAV\""),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt8, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numCols == 23);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -7);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_DATE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIME);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc10, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt8, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"NORMAL");
      CHECK(ind == 12);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"NORMAL");
      CHECK(ind == 24);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"42");
      CHECK(ind == 4);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"42");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"100000000000");
      CHECK(ind == 24);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"100000000000");
      CHECK(ind == 48);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"1000");
      CHECK(ind == 8);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"1000");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"100");
      CHECK(ind == 6);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"100");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"123456789012345678901234567890");
      CHECK(ind == 60);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"123456789012345678901234567890");
      CHECK(ind == 120);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"12345.678901");
      CHECK(ind == 24);
    }
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"12345.678901");
      CHECK(ind == 24);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"12345.678901");
      CHECK(ind == 48);
    }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 3.14);
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 2.718281828459045);
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 1.4142135);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"hello world");
      CHECK(ind == 22);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"hello world");
      CHECK(ind == 44);
    }
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"representative text payload");
      CHECK(ind == 54);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"representative text payload");
      CHECK(ind == 108);
    }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"fixedchar");
      CHECK(ind == 18);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"fixedchar");
      CHECK(ind == 36);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 14, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 1);
    CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == true);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 15, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());
    CHECK(_ds->year == 2024);
    CHECK(_ds->month == 1);
    CHECK(_ds->day == 15);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 16, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());
    CHECK(_ts->hour == 13);
    CHECK(_ts->minute == 45);
    CHECK(_ts->second == 30);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 17, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 1);
    CHECK(_ts->day == 15);
    CHECK(_ts->hour == 13);
    CHECK(_ts->minute == 45);
    CHECK(_ts->second == 30);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 18, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 1);
    CHECK(_ts->day == 14);
    CHECK(_ts->hour == 21);
    CHECK(_ts->minute == 45);
    CHECK(_ts->second == 30);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 1);
    CHECK(_ts->day == 14);
    CHECK(_ts->hour == 21);
    CHECK(_ts->minute == 45);
    CHECK(_ts->second == 30);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 20, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"a\": 1\n}");
      CHECK(ind == 24);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"a\": 1\n}");
      CHECK(ind == 48);
    }
  }

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 21, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"k\": \"v\"\n}");
      CHECK(ind == 28);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"k\": \"v\"\n}");
      CHECK(ind == 56);
    }
  }

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"[\n  1,\n  2,\n  3\n]");
      CHECK(ind == 34);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"[\n  1,\n  2,\n  3\n]");
      CHECK(ind == 68);
    }
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"coordinates\": [\n    -122,\n    37\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 124);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"coordinates\": [\n    -122,\n    37\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 248);
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"BOUNDARY");
      CHECK(ind == 16);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"BOUNDARY");
      CHECK(ind == 32);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"2147483647");
      CHECK(ind == 20);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"2147483647");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"9223372036854775807");
      CHECK(ind == 38);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"9223372036854775807");
      CHECK(ind == 76);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"32767");
      CHECK(ind == 10);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"32767");
      CHECK(ind == 20);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"127");
      CHECK(ind == 6);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"127");
      CHECK(ind == 12);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"99999999999999999999999999999999999999");
      CHECK(ind == 76);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"99999999999999999999999999999999999999");
      CHECK(ind == 152);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"999999999999.999999");
      CHECK(ind == 38);
    }
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"999999999999.999999");
      CHECK(ind == 38);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"999999999999.999999");
      CHECK(ind == 76);
    }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK(
        (std::isinf(*reinterpret_cast<double*>(buf.data())) && !std::signbit(*reinterpret_cast<double*>(buf.data()))));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK(std::isnan(*reinterpret_cast<double*>(buf.data())));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((std::isinf(*reinterpret_cast<double*>(buf.data())) && std::signbit(*reinterpret_cast<double*>(buf.data()))));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(
          actual ==
          u"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
          u"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
          u"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
      CHECK(ind == 512);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(
          actual ==
          U"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
          U"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
          U"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
      CHECK(ind == 1024);
    }
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"boundary text payload");
      CHECK(ind == 42);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"boundary text payload");
      CHECK(ind == 84);
    }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"YYYYYYYYYY");
      CHECK(ind == 20);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"YYYYYYYYYY");
      CHECK(ind == 40);
    }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 14, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 1);
    CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == false);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 15, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());
    CHECK(_ds->year == 9999);
    CHECK(_ds->month == 12);
    CHECK(_ds->day == 31);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 16, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == 6);
    const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());
    CHECK(_ts->hour == 23);
    CHECK(_ts->minute == 59);
    CHECK(_ts->second == 59);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 17, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 9999);
    CHECK(_ts->month == 12);
    CHECK(_ts->day == 31);
    CHECK(_ts->hour == 23);
    CHECK(_ts->minute == 59);
    CHECK(_ts->second == 59);
    CHECK(_ts->fraction == 999999999);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 18, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 9999);
    CHECK(_ts->month == 12);
    CHECK(_ts->day == 31);
    CHECK(_ts->hour == 7);
    CHECK(_ts->minute == 59);
    CHECK(_ts->second == 59);
    CHECK(_ts->fraction == 999999999);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 9999);
    CHECK(_ts->month == 12);
    CHECK(_ts->day == 30);
    CHECK(_ts->hour == 23);
    CHECK(_ts->minute == 59);
    CHECK(_ts->second == 59);
    CHECK(_ts->fraction == 999999999);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 20, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"null");
      CHECK(ind == 8);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"null");
      CHECK(ind == 16);
    }
  }

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 21, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{}");
      CHECK(ind == 4);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{}");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"[]");
      CHECK(ind == 4);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"[]");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"coordinates\": [\n    180,\n    90\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 122);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"coordinates\": [\n    180,\n    90\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 244);
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"UNICODE");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"UNICODE");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"7");
      CHECK(ind == 2);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"7");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"8");
      CHECK(ind == 2);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"8");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"9");
      CHECK(ind == 2);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"9");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"1");
      CHECK(ind == 2);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"1");
      CHECK(ind == 4);
    }
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"42");
      CHECK(ind == 4);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"42");
      CHECK(ind == 8);
    }
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"3.141593");
      CHECK(ind == 16);
    }
    OLD_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"3.141593");
      CHECK(ind == 16);
    }
    NEW_IODBC_ONLY("BD#79") {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"3.141593");
      CHECK(ind == 32);
    }
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 1);
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 2);
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 3);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    // The iODBC width is not 2x the unixODBC byte count here: the value
    // holds a surrogate pair, which is a single UTF-32 code unit.
    NON_IODBC { CHECK(ind == 90); }
    IODBC_ONLY { CHECK(ind == 176); }
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    // The iODBC width is not 2x the unixODBC byte count here: the value
    // holds a surrogate pair, which is a single UTF-32 code unit.
    NON_IODBC { CHECK(ind == 62); }
    IODBC_ONLY { CHECK(ind == 120); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 10); }
    IODBC_ONLY { CHECK(ind == 20); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 14, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 1);
    CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == true);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 15, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());
    CHECK(_ds->year == 2024);
    CHECK(_ds->month == 2);
    CHECK(_ds->day == 29);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 16, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());
    CHECK(_ts->hour == 12);
    CHECK(_ts->minute == 0);
    CHECK(_ts->second == 0);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 17, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 2);
    CHECK(_ts->day == 29);
    CHECK(_ts->hour == 12);
    CHECK(_ts->minute == 0);
    CHECK(_ts->second == 0);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 18, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 2);
    CHECK(_ts->day == 28);
    CHECK(_ts->hour == 20);
    CHECK(_ts->minute == 0);
    CHECK(_ts->second == 0);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());
    CHECK(_ts->year == 2024);
    CHECK(_ts->month == 2);
    CHECK(_ts->day == 28);
    CHECK(_ts->hour == 3);
    CHECK(_ts->minute == 0);
    CHECK(_ts->second == 0);
    CHECK(_ts->fraction == 0);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 20, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    // The iODBC width is not 2x the unixODBC byte count here: the value
    // holds a surrogate pair, which is a single UTF-32 code unit.
    NON_IODBC { CHECK(ind == 68); }
    IODBC_ONLY { CHECK(ind == 132); }
  }

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 21, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 36); }
    IODBC_ONLY { CHECK(ind == 72); }
  }

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 46); }
    IODBC_ONLY { CHECK(ind == 92); }
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"{\n  \"coordinates\": [\n    139.6917,\n    35.6895\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 142);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"{\n  \"coordinates\": [\n    139.6917,\n    35.6895\n  ],\n  \"type\": \"Point\"\n}");
      CHECK(ind == 284);
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"NULLROW");
      CHECK(ind == 14);
    }
    IODBC_ONLY {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char32_t), buf.size() / sizeof(char32_t));
      std::u32string actual(reinterpret_cast<const char32_t*>(buf.data()), code_units);
      CHECK(actual == U"NULLROW");
      CHECK(ind == 28);
    }
  }

  // SQLGetData col 2
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 14, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 15, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 16, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 17, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 18, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 20, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 21, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt8, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsNoData());
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc10);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc10);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc10), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc9);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc9);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc9), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
  }

  // --- Replay-only env cleanup (not present in the original trace) ---
  // ODBC-consuming hosts (Excel, Power Query, ...) deliberately leave their
  // SQL_HANDLE_ENV handles allocated at shutdown — the pool root for
  // `SQL_ATTR_CONNECTION_POOLING` is anchored on the env, and any teardown
  // done during DllMain(DLL_PROCESS_DETACH) is invisible to the trace logger.
  // Our replay binary runs many tests in one process, so we explicitly free
  // each leaked env here to avoid leaking pooled connections across tests.
  // SQLFreeHandle - SQLHENV (env1, replay-only)
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }

  // skipped 129 SQLColAttribute call(s) with undocumented field id
}
