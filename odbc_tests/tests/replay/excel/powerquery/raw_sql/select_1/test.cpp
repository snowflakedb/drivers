#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: excel powerquery raw_sql select_1", "[excel][powerquery][raw_sql]") {
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

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt0, sqlchar("SELECT 1;"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
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

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt1, sqlchar("SELECT 1;"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt1, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
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

  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_ROWS_FETCHED_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    // SQLWCHAR is 4 bytes (UTF-32) under iODBC and 2 bytes (UTF-16) otherwise.
    // Column 1 is a numeric source (SELECT 1). Under iODBC the new driver writes
    // UTF-32 uniformly, but the old driver writes UTF-16 for numeric sources (BD#79).
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"1");
      CHECK(ind == 2);
    }
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

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsNoData());
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

  // --- Replay-only env cleanup (not present in the original trace) ---
  // ODBC-consuming hosts (Excel, Power Query, ...) deliberately leave their
  // SQL_HANDLE_ENV handles allocated at shutdown — the pool root for
  // `SQL_ATTR_CONNECTION_POOLING` is anchored on the env, and any teardown
  // done during DllMain(DLL_PROCESS_DETACH) is invisible to the trace logger.
  // Our replay binary runs many tests in one process, so we explicitly free
  // each leaked env here to avoid leaking pooled connections across tests.
  // SQLFreeHandle - SQLHENV (env0, replay-only)
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV (env1, replay-only)
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }

  // skipped 2 SQLColAttribute call(s) with undocumented field id
}
