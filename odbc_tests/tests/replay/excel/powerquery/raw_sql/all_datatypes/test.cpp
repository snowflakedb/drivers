#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: excel powerquery raw_sql all_datatypes", "[excel][powerquery][raw_sql]") {
  // New driver: skipped until BD-azure is fixed (heap corruption during key-pair
  // SQLDriverConnect on windows-x64-azure).
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

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt0,
                                  sqlchar("SELECT * REPLACE(\n  DATEADD('day', -1, TSLTZ) AS TSLTZ,\n  DATEADD('day', "
                                          "-1, TSTZ)  AS TSTZ\n) FROM ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPES;"),
                                  SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 25);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
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
    CHECK(numAttr == SQL_DECIMAL);
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
    CHECK(numAttr == SQL_DECIMAL);
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
    CHECK(numAttr == SQL_DECIMAL);
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
    CHECK(numAttr == SQL_DECIMAL);
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

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -2);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -2);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -7);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_DATE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIME);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 25;
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
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLExecDirect(stmt1,
                                  sqlchar("SELECT * REPLACE(\n  DATEADD('day', -1, TSLTZ) AS TSLTZ,\n  DATEADD('day', "
                                          "-1, TSTZ)  AS TSTZ\n) FROM ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPES;"),
                                  SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt1, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numCols == 25);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
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
    CHECK(numAttr == SQL_DECIMAL);
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
    CHECK(numAttr == SQL_DECIMAL);
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
    CHECK(numAttr == SQL_DECIMAL);
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
    CHECK(numAttr == SQL_DECIMAL);
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

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -2);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -2);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -7);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_DATE);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIME);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[4096] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_TYPE_NAME, buf, 4096, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 25;
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
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 25;
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
    SQLRETURN ret = SQLGetData(stmt1, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"42");
      CHECK(ind == 4);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"100000000000");
      CHECK(ind == 24);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"1000");
      CHECK(ind == 8);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"100");
      CHECK(ind == 6);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"123456789012345678901234567890");
      CHECK(ind == 60);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 3.14);
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 2.718281828459045);
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 1.4142135);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 14, SQL_C_BINARY, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    const unsigned char expected[] = {0xde, 0xad, 0xbe, 0xef, 0xde, 0xad, 0xbe, 0xef,
                                      0xde, 0xad, 0xbe, 0xef, 0xde, 0xad, 0xbe, 0xef};
    CHECK(n == sizeof(expected));
    CHECK(std::memcmp(buf.data(), expected, n) == 0);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 15, SQL_C_BINARY, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    const unsigned char expected[] = {0xca, 0xfe, 0xba, 0xbe};
    CHECK(n == sizeof(expected));
    CHECK(std::memcmp(buf.data(), expected, n) == 0);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 16, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 1);
    CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == true);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 17, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());
    CHECK(_ds->year == 2024);
    CHECK(_ds->month == 1);
    CHECK(_ds->day == 15);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 18, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());
    CHECK(_ts->hour == 13);
    CHECK(_ts->minute == 45);
    CHECK(_ts->second == 30);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 20, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 21, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 24
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 24, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 25
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 25, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLFetch(stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"2147483647");
      CHECK(ind == 20);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"9223372036854775807");
      CHECK(ind == 38);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"32767");
      CHECK(ind == 10);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"127");
      CHECK(ind == 6);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"99999999999999999999999999999999999999");
      CHECK(ind == 76);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK(
        (std::isinf(*reinterpret_cast<double*>(buf.data())) && !std::signbit(*reinterpret_cast<double*>(buf.data()))));
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK(std::isnan(*reinterpret_cast<double*>(buf.data())));
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((std::isinf(*reinterpret_cast<double*>(buf.data())) && std::signbit(*reinterpret_cast<double*>(buf.data()))));
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 14, SQL_C_BINARY, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    const unsigned char expected[] = {0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff};
    CHECK(n == sizeof(expected));
    CHECK(std::memcmp(buf.data(), expected, n) == 0);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 15, SQL_C_BINARY, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    const unsigned char expected[] = {0xff, 0xff};
    CHECK(n == sizeof(expected));
    CHECK(std::memcmp(buf.data(), expected, n) == 0);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 16, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 1);
    CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == false);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 17, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());
    CHECK(_ds->year == 9999);
    CHECK(_ds->month == 12);
    CHECK(_ds->day == 31);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 18, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == 6);
    const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());
    CHECK(_ts->hour == 23);
    CHECK(_ts->minute == 59);
    CHECK(_ts->second == 59);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 20, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 21, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 24
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 24, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 25
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 25, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLFetch(stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"7");
      CHECK(ind == 2);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"8");
      CHECK(ind == 2);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"9");
      CHECK(ind == 2);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    NON_IODBC {
      const size_t code_units =
          std::min<size_t>(static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));
      std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);
      CHECK(actual == u"42");
      CHECK(ind == 4);
    }
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
    SQLRETURN ret = SQLGetData(stmt1, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 1);
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 2);
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 8);
    CHECK((*reinterpret_cast<double*>(buf.data())) == 3);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 90); }
    IODBC_ONLY { CHECK(ind == 176); }
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 62); }
    IODBC_ONLY { CHECK(ind == 120); }
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 10); }
    IODBC_ONLY { CHECK(ind == 20); }
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 14, SQL_C_BINARY, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 16);
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    const unsigned char expected[] = {0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                                      0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff};
    CHECK(n == sizeof(expected));
    CHECK(std::memcmp(buf.data(), expected, n) == 0);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 15, SQL_C_BINARY, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    const unsigned char expected[] = {0xca, 0xfe};
    CHECK(n == sizeof(expected));
    CHECK(std::memcmp(buf.data(), expected, n) == 0);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 16, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 1);
    CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == true);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 17, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());
    CHECK(_ds->year == 2024);
    CHECK(_ds->month == 2);
    CHECK(_ds->day == 29);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 18, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == 6);
    const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());
    CHECK(_ts->hour == 12);
    CHECK(_ts->minute == 0);
    CHECK(_ts->second == 0);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 20, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 21, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 68); }
    IODBC_ONLY { CHECK(ind == 132); }
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 36); }
    IODBC_ONLY { CHECK(ind == 72); }
  }

  // SQLGetData col 24
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 24, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    // SQL_C_WCHAR value not pinned: trace rendering used CP_ACP
    // and may have replaced unmappable codepoints with '?'.
    NON_IODBC { CHECK(ind == 46); }
    IODBC_ONLY { CHECK(ind == 92); }
  }

  // SQLGetData col 25
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 25, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLFetch(stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 1, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLGetData(stmt1, 2, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 3
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 3, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 4
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 4, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 5
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 5, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 6, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 7
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 7, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 8
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 8, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 9, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 10, SQL_C_DOUBLE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 11, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 12, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 13, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 14
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 14, SQL_C_BINARY, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 15, SQL_C_BINARY, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 16
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 16, SQL_C_BIT, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 17
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 17, SQL_C_TYPE_DATE, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 18
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 18, SQL_C_TYPE_TIME, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 19
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 19, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 20
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 20, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 21
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 21, SQL_C_TYPE_TIMESTAMP, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 22
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 22, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 23
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 23, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 24
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 24, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 25
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2048, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt1, 25, SQL_C_WCHAR, buf.data(), 2048, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
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

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }

  // skipped 50 SQLColAttribute call(s) with undocumented field id
}
