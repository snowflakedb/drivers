#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ReadOnlyDbFixture.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: tableau generic_odbc navigate_and_load", "[tableau][generic_odbc][navigate_and_load]") {
  // The new driver skips this replay while these are open:
  // SNOW-4283109 SQLTables returns no catalogs without a current database.
  // SNOW-4283110 SQLPrimaryKeys and SQLForeignKeys SMALLINT display size is 136.
  // SNOW-4283111 SQLTables SQL_C_WCHAR fetch posts 01004 when the values fit.
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  auto base = DataSourceConfig::Snowflake();
  auto config = base.install();
  // no_db and fixture_db copy base, so they use the driver install() registered.
  // dbc1 has no current database, so unqualified CREATE and DROP fail.
  // dbc3 names the fixture database and has no current schema, so qualified
  // names resolve and those same unqualified statements still fail.
  auto no_db = base;
  no_db.remove("DATABASE").remove("SCHEMA");
  auto fixture_db = base;
  fixture_db.set("DATABASE", READONLY_DB_NAME).remove("SCHEMA");

  SQLHENV env0 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env0);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env0 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env0, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env0);
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
    SQLRETURN ret = SQLSetEnvAttr(env1, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLSetEnvAttr(env2, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc0 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env2, &dbc0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
    REQUIRE(dbc0 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)30, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc0, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLSetConnectAttr - SQL_ATTR_QUIET_MODE
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_QUIET_MODE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_TXN_CAPABLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_TXN_CAPABLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_CURSOR_COMMIT_BEHAVIOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CURSOR_COMMIT_BEHAVIOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
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

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
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

  // SQLGetInfo - SQL_COLUMN_ALIAS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_COLUMN_ALIAS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "Y");
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

  // SQLGetInfo - SQL_IDENTIFIER_QUOTE_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_IDENTIFIER_QUOTE_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\"");
  }

  // SQLGetInfo - SQL_CATALOG_NAME_SEPARATOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_NAME_SEPARATOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == ".");
  }

  // SQLGetInfo - SQL_SPECIAL_CHARACTERS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SPECIAL_CHARACTERS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_QUOTED_IDENTIFIER_CASE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_QUOTED_IDENTIFIER_CASE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - 169
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, 169, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
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

  // SQLGetInfo - SQL_DATETIME_LITERALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATETIME_LITERALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
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

  // SQLGetInfo - 173
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, 173, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
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

  // SQLGetInfo - SQL_CONVERT_INTERVAL_DAY_TIME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CONVERT_INTERVAL_DAY_TIME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    OLD_DRIVER_ONLY("BD#convert-interval-day-time-omits-wide-char") { CHECK(numericValue == 0xF06F1Fu); }
    NEW_DRIVER_ONLY("BD#convert-interval-day-time-omits-wide-char") { CHECK(numericValue == 0x106F1Fu); }
  }

  // SQLGetInfo - SQL_SQL92_NUMERIC_VALUE_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SQL92_NUMERIC_VALUE_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x39u);
  }

  // SQLGetInfo - SQL_SQL92_STRING_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SQL92_STRING_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xEFu);
  }

  // SQLGetInfo - SQL_SQL92_DATETIME_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SQL92_DATETIME_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_OJ_CAPABILITIES
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_OJ_CAPABILITIES, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
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

  // SQLSetConnectAttr - SQL_ATTR_TXN_ISOLATION
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_TXN_ISOLATION, (SQLPOINTER)8, -5);
    OLD_DRIVER_ONLY("BD#txn-serializable-substituted-read-committed") {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    }
    NEW_DRIVER_ONLY("BD#txn-serializable-substituted-read-committed") {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01S02"));
    }
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
    SQLRETURN ret = SQLSetEnvAttr(env3, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc1 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc1 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc1, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)30, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc1, nullptr, sqlchar(no_db.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::Succeeded());
  }

  // SQLSetConnectAttr - SQL_ATTR_QUIET_MODE
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc1, SQL_ATTR_QUIET_MODE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_TXN_CAPABLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_TXN_CAPABLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_CURSOR_COMMIT_BEHAVIOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CURSOR_COMMIT_BEHAVIOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
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

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_ODBC_INTERFACE_CONFORMANCE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_ODBC_INTERFACE_CONFORMANCE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_SQL_CONFORMANCE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SQL_CONFORMANCE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_CATALOG_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CATALOG_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15u);
  }

  // SQLGetInfo - SQL_OWNER_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_OWNER_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15u);
  }

  // SQLGetInfo - SQL_COLUMN_ALIAS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_COLUMN_ALIAS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "Y");
  }

  // SQLGetInfo - SQL_MAX_IDENTIFIER_LEN
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_MAX_IDENTIFIER_LEN, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFu);
  }

  // SQLGetInfo - SQL_IDENTIFIER_QUOTE_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_IDENTIFIER_QUOTE_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\"");
  }

  // SQLGetInfo - SQL_CATALOG_NAME_SEPARATOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CATALOG_NAME_SEPARATOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == ".");
  }

  // SQLGetInfo - SQL_SPECIAL_CHARACTERS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SPECIAL_CHARACTERS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_QUOTED_IDENTIFIER_CASE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_QUOTED_IDENTIFIER_CASE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - 169
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, 169, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_NUMERIC_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_NUMERIC_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF7FFFFu);
  }

  // SQLGetInfo - SQL_STRING_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_STRING_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFD7FFFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_TIMEDATE_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFFFFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_ADD_INTERVALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_TIMEDATE_ADD_INTERVALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_DIFF_INTERVALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_TIMEDATE_DIFF_INTERVALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFu);
  }

  // SQLGetInfo - SQL_DATETIME_LITERALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DATETIME_LITERALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_SYSTEM_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SYSTEM_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_CONVERT_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_SQL92_VALUE_EXPRESSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SQL92_VALUE_EXPRESSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFu);
  }

  // SQLGetInfo - SQL_CONVERT_TINYINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_TINYINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_SMALLINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_SMALLINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_INTEGER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_INTEGER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_BIGINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_BIGINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_REAL
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_REAL, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_FLOAT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_FLOAT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_DOUBLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_DOUBLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA3EDFFu);
  }

  // SQLGetInfo - SQL_CONVERT_VARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_VARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA38DAFu);
  }

  // SQLGetInfo - SQL_CONVERT_DECIMAL
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_DECIMAL, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_NUMERIC
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_NUMERIC, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_BIT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_BIT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xE47FFFu);
  }

  // SQLGetInfo - 173
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, 173, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_CONVERT_BINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_BINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA00D01u);
  }

  // SQLGetInfo - SQL_CONVERT_VARBINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_VARBINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA00D01u);
  }

  // SQLGetInfo - SQL_CONVERT_LONGVARBINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_LONGVARBINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xE40F01u);
  }

  // SQLGetInfo - SQL_CONVERT_DATE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_DATE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA28101u);
  }

  // SQLGetInfo - SQL_CONVERT_TIME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_TIME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA10101u);
  }

  // SQLGetInfo - SQL_CONVERT_TIMESTAMP
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_TIMESTAMP, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA38101u);
  }

  // SQLGetInfo - SQL_CONVERT_INTERVAL_DAY_TIME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CONVERT_INTERVAL_DAY_TIME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    OLD_DRIVER_ONLY("BD#convert-interval-day-time-omits-wide-char") { CHECK(numericValue == 0xF06F1Fu); }
    NEW_DRIVER_ONLY("BD#convert-interval-day-time-omits-wide-char") { CHECK(numericValue == 0x106F1Fu); }
  }

  // SQLGetInfo - SQL_SQL92_NUMERIC_VALUE_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SQL92_NUMERIC_VALUE_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x39u);
  }

  // SQLGetInfo - SQL_SQL92_STRING_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SQL92_STRING_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xEFu);
  }

  // SQLGetInfo - SQL_SQL92_DATETIME_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SQL92_DATETIME_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_OJ_CAPABILITIES
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_OJ_CAPABILITIES, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_SQL92_RELATIONAL_JOIN_OPERATORS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SQL92_RELATIONAL_JOIN_OPERATORS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15Au);
  }

  // SQLGetInfo - SQL_SQL92_PREDICATES
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SQL92_PREDICATES, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3F05u);
  }

  // SQLSetConnectAttr - SQL_ATTR_TXN_ISOLATION
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc1, SQL_ATTR_TXN_ISOLATION, (SQLPOINTER)8, -5);
    OLD_DRIVER_ONLY("BD#txn-serializable-substituted-read-committed") {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    }
    NEW_DRIVER_ONLY("BD#txn-serializable-substituted-read-committed") {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01S02"));
    }
  }

  SQLHSTMT stmt0 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt0 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt0,
                               sqlchar("CREATE LOCAL TEMPORARY TABLE "
                                       "\"XT01_E5B0BC14_436C_44F4_9582_E3784773B5DA_1_Connect_"
                                       "CheckCreateTempTableCap\" (\n\t\"COL\" INTEGER\n\t) ON COMMIT PRESERVE ROWS"),
                               SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt1 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt1 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt1, sqlchar("DROP TABLE \"XT01_E5B0BC14_436C_44F4_9582_E3784773B5DA_1_Connect_CheckCreateTempTableCap\""),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt2 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt2 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt2, sqlchar("SELECT \"COL\"\nFROM (SELECT 1 AS \"COL\") AS \"SUBQUERY\""), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt2, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
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

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
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

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt2, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
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

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_0 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_0, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_1(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_1.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_2(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_2(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt2, 1, SQL_C_SBIGINT, bind_buf_2.data(), 8, bind_ind_2.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_3(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_3(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt2, 1, SQL_C_SBIGINT, bind_buf_3.data(), 8, bind_ind_3.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt3 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt3 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret =
        SQLPrepare(stmt3, sqlchar("SELECT TOP 1 \"COL\"\nFROM (SELECT 1 AS \"COL\") AS \"CHECKTOP\""), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt3, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
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

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
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

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
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

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_4 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_4, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_5(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_5.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_6(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_6(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt3, 1, SQL_C_SBIGINT, bind_buf_6.data(), 8, bind_ind_6.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_7(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_7(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt3, 1, SQL_C_SBIGINT, bind_buf_7.data(), 8, bind_ind_7.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt4 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt4);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt4 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt4, sqlchar("SELECT \"SUBCOL\" AS \"COL\"\nFROM (\n  SELECT 1 AS \"SUBCOL\"\n) \"SUBQUERY\"\nGROUP BY 1"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt4);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt4, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
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

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
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

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
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

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_8 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_8, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_9(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_9.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_10(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_10(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt4, 1, SQL_C_SBIGINT, bind_buf_10.data(), 8, bind_ind_10.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_11(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_11(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt4, 1, SQL_C_SBIGINT, bind_buf_11.data(), 8, bind_ind_11.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt5 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt5);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt5 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt5, sqlchar("SELECT \"SUBCOL\" AS \"COL\"\nFROM (\n  SELECT 1 AS \"SUBCOL\"\n) \"SUBQUERY\"\nGROUP BY 2"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt6 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt6);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt6 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt6,
        sqlchar("SELECT *\nINTO \"XT01_E5B0BC14_436C_44F4_9582_E3784773B5DA_2_Connect_CheckSelectIntoCap\"\nFROM "
                "(SELECT 1 AS COL) AS CHECKTEMP"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt7 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt7);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt7 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt7, sqlchar("DROP TABLE \"XT01_E5B0BC14_436C_44F4_9582_E3784773B5DA_2_Connect_CheckSelectIntoCap\""),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt8 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt8);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt8 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt8, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_12(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_12(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 1, SQL_C_WCHAR, bind_buf_12.data(), 1024, bind_ind_12.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_13(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_13(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 2, SQL_C_WCHAR, bind_buf_13.data(), 1024, bind_ind_13.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_14(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_14(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 3, SQL_C_WCHAR, bind_buf_14.data(), 1024, bind_ind_14.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_15(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_15(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 4, SQL_C_WCHAR, bind_buf_15.data(), 1024, bind_ind_15.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_16(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_16(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 5, SQL_C_WCHAR, bind_buf_16.data(), 1024, bind_ind_16.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt8)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt9 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt9);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt9 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt9, sqlchar("SELECT 1"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt9);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt9, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt9, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_17 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt9, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_17, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_18(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt9, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_18.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_19(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_19(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 1, SQL_C_SBIGINT, bind_buf_19.data(), 8, bind_ind_19.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt9);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt9);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_20(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_20(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 1, SQL_C_SBIGINT, bind_buf_20.data(), 8, bind_ind_20.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt9);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  SQLHENV env4 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env4);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env4 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env4, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env4), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env4), OdbcMatchers::IsSuccess());
  }

  SQLHENV env5 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env5);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env5 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env5, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env5), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc2 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env5, &dbc2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env5), OdbcMatchers::IsSuccess());
    REQUIRE(dbc2 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc2, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)30, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc2, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::Succeeded());
  }

  // SQLSetConnectAttr - SQL_ATTR_QUIET_MODE
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc2, SQL_ATTR_QUIET_MODE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_TXN_CAPABLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_TXN_CAPABLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_CURSOR_COMMIT_BEHAVIOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CURSOR_COMMIT_BEHAVIOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
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

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_ODBC_INTERFACE_CONFORMANCE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_ODBC_INTERFACE_CONFORMANCE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_SQL_CONFORMANCE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_SQL_CONFORMANCE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_CATALOG_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CATALOG_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15u);
  }

  // SQLGetInfo - SQL_OWNER_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_OWNER_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15u);
  }

  // SQLGetInfo - SQL_COLUMN_ALIAS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_COLUMN_ALIAS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "Y");
  }

  // SQLGetInfo - SQL_MAX_IDENTIFIER_LEN
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_MAX_IDENTIFIER_LEN, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFu);
  }

  // SQLGetInfo - SQL_IDENTIFIER_QUOTE_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_IDENTIFIER_QUOTE_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\"");
  }

  // SQLGetInfo - SQL_CATALOG_NAME_SEPARATOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CATALOG_NAME_SEPARATOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == ".");
  }

  // SQLGetInfo - SQL_SPECIAL_CHARACTERS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_SPECIAL_CHARACTERS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_QUOTED_IDENTIFIER_CASE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_QUOTED_IDENTIFIER_CASE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - 169
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, 169, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_NUMERIC_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_NUMERIC_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF7FFFFu);
  }

  // SQLGetInfo - SQL_STRING_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_STRING_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFD7FFFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_TIMEDATE_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFFFFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_ADD_INTERVALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_TIMEDATE_ADD_INTERVALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFu);
  }

  // SQLGetInfo - SQL_TIMEDATE_DIFF_INTERVALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_TIMEDATE_DIFF_INTERVALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1FFu);
  }

  // SQLGetInfo - SQL_DATETIME_LITERALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_DATETIME_LITERALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_SYSTEM_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_SYSTEM_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_CONVERT_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_SQL92_VALUE_EXPRESSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_SQL92_VALUE_EXPRESSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFu);
  }

  // SQLGetInfo - SQL_CONVERT_TINYINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_TINYINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_SMALLINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_SMALLINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_INTEGER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_INTEGER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_BIGINT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_BIGINT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_REAL
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_REAL, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xF87FFFu);
  }

  // SQLGetInfo - SQL_CONVERT_FLOAT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_FLOAT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_DOUBLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_DOUBLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA3EDFFu);
  }

  // SQLGetInfo - SQL_CONVERT_VARCHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_VARCHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA38DAFu);
  }

  // SQLGetInfo - SQL_CONVERT_DECIMAL
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_DECIMAL, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_NUMERIC
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_NUMERIC, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA001AFu);
  }

  // SQLGetInfo - SQL_CONVERT_BIT
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_BIT, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xE47FFFu);
  }

  // SQLGetInfo - 173
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, 173, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_CONVERT_BINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_BINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA00D01u);
  }

  // SQLGetInfo - SQL_CONVERT_VARBINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_VARBINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA00D01u);
  }

  // SQLGetInfo - SQL_CONVERT_LONGVARBINARY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_LONGVARBINARY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xE40F01u);
  }

  // SQLGetInfo - SQL_CONVERT_DATE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_DATE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA28101u);
  }

  // SQLGetInfo - SQL_CONVERT_TIME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_TIME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA10101u);
  }

  // SQLGetInfo - SQL_CONVERT_TIMESTAMP
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_TIMESTAMP, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xA38101u);
  }

  // SQLGetInfo - SQL_CONVERT_INTERVAL_DAY_TIME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_CONVERT_INTERVAL_DAY_TIME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    OLD_DRIVER_ONLY("BD#convert-interval-day-time-omits-wide-char") { CHECK(numericValue == 0xF06F1Fu); }
    NEW_DRIVER_ONLY("BD#convert-interval-day-time-omits-wide-char") { CHECK(numericValue == 0x106F1Fu); }
  }

  // SQLGetInfo - SQL_SQL92_NUMERIC_VALUE_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_SQL92_NUMERIC_VALUE_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x39u);
  }

  // SQLGetInfo - SQL_SQL92_STRING_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_SQL92_STRING_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xEFu);
  }

  // SQLGetInfo - SQL_SQL92_DATETIME_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_SQL92_DATETIME_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_OJ_CAPABILITIES
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_OJ_CAPABILITIES, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_SQL92_RELATIONAL_JOIN_OPERATORS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_SQL92_RELATIONAL_JOIN_OPERATORS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x15Au);
  }

  // SQLGetInfo - SQL_SQL92_PREDICATES
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc2, SQL_SQL92_PREDICATES, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3F05u);
  }

  // SQLSetConnectAttr - SQL_ATTR_TXN_ISOLATION
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc2, SQL_ATTR_TXN_ISOLATION, (SQLPOINTER)8, -5);
    OLD_DRIVER_ONLY("BD#txn-serializable-substituted-read-committed") {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    }
    NEW_DRIVER_ONLY("BD#txn-serializable-substituted-read-committed") {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01S02"));
    }
  }

  SQLHSTMT stmt10 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt10);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt10 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt10, sqlchar("SELECT 1"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt10);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt10, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt10, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_21 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt10, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_21, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_22(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt10, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_22.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_23(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_23(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt10, 1, SQL_C_SBIGINT, bind_buf_23.data(), 8, bind_ind_23.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt10);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt10);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_24(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_24(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt10, 1, SQL_C_SBIGINT, bind_buf_24.data(), 8, bind_ind_24.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt10);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt11 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt11);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt11 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt11, sqlchar("%"), -3, sqlchar(""), -3, sqlchar(""), -3, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_25(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_25(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt11, 1, SQL_C_WCHAR, bind_buf_25.data(), 1024, bind_ind_25.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_26(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_26(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt11, 2, SQL_C_WCHAR, bind_buf_26.data(), 1024, bind_ind_26.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_27(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_27(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt11, 3, SQL_C_WCHAR, bind_buf_27.data(), 1024, bind_ind_27.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_28(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_28(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt11, 4, SQL_C_WCHAR, bind_buf_28.data(), 1024, bind_ind_28.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_29(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_29(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt11, 5, SQL_C_WCHAR, bind_buf_29.data(), 1024, bind_ind_29.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt11)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt11);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  SQLHENV env6 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env6);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env6 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env6, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env6), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc3 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env6, &dbc3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env6), OdbcMatchers::IsSuccess());
    REQUIRE(dbc3 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc3, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)30, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc3, nullptr, sqlchar(fixture_db.connection_string().c_str()), SQL_NTS, nullptr,
                                     0, nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
  }

  // SQLSetConnectAttr - SQL_ATTR_QUIET_MODE
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc3, SQL_ATTR_QUIET_MODE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_TXN_CAPABLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_TXN_CAPABLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_CURSOR_COMMIT_BEHAVIOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CURSOR_COMMIT_BEHAVIOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
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

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
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

  // SQLGetInfo - SQL_COLUMN_ALIAS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_COLUMN_ALIAS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "Y");
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

  // SQLGetInfo - SQL_IDENTIFIER_QUOTE_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_IDENTIFIER_QUOTE_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\"");
  }

  // SQLGetInfo - SQL_CATALOG_NAME_SEPARATOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CATALOG_NAME_SEPARATOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == ".");
  }

  // SQLGetInfo - SQL_SPECIAL_CHARACTERS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SPECIAL_CHARACTERS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_QUOTED_IDENTIFIER_CASE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_QUOTED_IDENTIFIER_CASE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - 169
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, 169, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
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

  // SQLGetInfo - SQL_DATETIME_LITERALS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_DATETIME_LITERALS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
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

  // SQLGetInfo - 173
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, 173, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
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

  // SQLGetInfo - SQL_CONVERT_INTERVAL_DAY_TIME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_CONVERT_INTERVAL_DAY_TIME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    OLD_DRIVER_ONLY("BD#convert-interval-day-time-omits-wide-char") { CHECK(numericValue == 0xF06F1Fu); }
    NEW_DRIVER_ONLY("BD#convert-interval-day-time-omits-wide-char") { CHECK(numericValue == 0x106F1Fu); }
  }

  // SQLGetInfo - SQL_SQL92_NUMERIC_VALUE_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SQL92_NUMERIC_VALUE_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x39u);
  }

  // SQLGetInfo - SQL_SQL92_STRING_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SQL92_STRING_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xEFu);
  }

  // SQLGetInfo - SQL_SQL92_DATETIME_FUNCTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_SQL92_DATETIME_FUNCTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
  }

  // SQLGetInfo - SQL_OJ_CAPABILITIES
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc3, SQL_OJ_CAPABILITIES, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x7u);
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

  // SQLSetConnectAttr - SQL_ATTR_TXN_ISOLATION
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc3, SQL_ATTR_TXN_ISOLATION, (SQLPOINTER)8, -5);
    OLD_DRIVER_ONLY("BD#txn-serializable-substituted-read-committed") {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    }
    NEW_DRIVER_ONLY("BD#txn-serializable-substituted-read-committed") {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01S02"));
    }
  }

  SQLHSTMT stmt12 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt12);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt12 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt12,
                               sqlchar("CREATE LOCAL TEMPORARY TABLE "
                                       "\"XT01_23CB0B93_C47B_407B_A962_6C3CB0EE4B07_1_Connect_"
                                       "CheckCreateTempTableCap\" (\n\t\"COL\" INTEGER\n\t) ON COMMIT PRESERVE ROWS"),
                               SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt12), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt12);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt12), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt13 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt13);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt13 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt13, sqlchar("DROP TABLE \"XT01_23CB0B93_C47B_407B_A962_6C3CB0EE4B07_1_Connect_CheckCreateTempTableCap\""),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt13), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt13);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt13), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt14 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt14);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt14 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt14, sqlchar("SELECT \"COL\"\nFROM (SELECT 1 AS \"COL\") AS \"SUBQUERY\""), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt14);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt14, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt14, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt14, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_30 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt14, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_30, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_31(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt14, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_31.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_32(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_32(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt14, 1, SQL_C_SBIGINT, bind_buf_32.data(), 8, bind_ind_32.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt14);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt14);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_33(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_33(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt14, 1, SQL_C_SBIGINT, bind_buf_33.data(), 8, bind_ind_33.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt14);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt14), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt15 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt15);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt15 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret =
        SQLPrepare(stmt15, sqlchar("SELECT TOP 1 \"COL\"\nFROM (SELECT 1 AS \"COL\") AS \"CHECKTOP\""), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt15);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt15, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt15, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt15, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_34 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt15, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_34, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_35(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt15, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_35.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_36(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_36(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt15, 1, SQL_C_SBIGINT, bind_buf_36.data(), 8, bind_ind_36.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt15);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt15);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_37(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_37(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt15, 1, SQL_C_SBIGINT, bind_buf_37.data(), 8, bind_ind_37.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt15);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt15), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt16 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt16);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt16 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt16, sqlchar("SELECT \"SUBCOL\" AS \"COL\"\nFROM (\n  SELECT 1 AS \"SUBCOL\"\n) \"SUBQUERY\"\nGROUP BY 1"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt16);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt16, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt16, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt16, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_38 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt16, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_38, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_39(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt16, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_39.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_40(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_40(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt16, 1, SQL_C_SBIGINT, bind_buf_40.data(), 8, bind_ind_40.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt16);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt16);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_41(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_41(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt16, 1, SQL_C_SBIGINT, bind_buf_41.data(), 8, bind_ind_41.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt16);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt16), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt17 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt17);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt17 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt17, sqlchar("SELECT \"SUBCOL\" AS \"COL\"\nFROM (\n  SELECT 1 AS \"SUBCOL\"\n) \"SUBQUERY\"\nGROUP BY 2"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt17), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt17);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt17), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt18 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt18);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt18 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt18,
        sqlchar("SELECT *\nINTO \"XT01_23CB0B93_C47B_407B_A962_6C3CB0EE4B07_2_Connect_CheckSelectIntoCap\"\nFROM "
                "(SELECT 1 AS COL) AS CHECKTEMP"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt18), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt18);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt18), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt19 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt19);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt19 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt19, sqlchar("DROP TABLE \"XT01_23CB0B93_C47B_407B_A962_6C3CB0EE4B07_2_Connect_CheckSelectIntoCap\""),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt19), OdbcMatchers::IsError());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt19);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt19), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt20 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt20);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt20 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt20, sqlchar("ODBCMETADATATESTDB"), -3, nullptr, 0, nullptr, 0, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt20), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_42(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_42(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt20, 1, SQL_C_WCHAR, bind_buf_42.data(), 1024, bind_ind_42.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt20), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_43(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_43(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt20, 2, SQL_C_WCHAR, bind_buf_43.data(), 1024, bind_ind_43.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt20), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_44(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_44(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt20, 3, SQL_C_WCHAR, bind_buf_44.data(), 1024, bind_ind_44.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt20), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_45(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_45(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt20, 4, SQL_C_WCHAR, bind_buf_45.data(), 1024, bind_ind_45.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt20), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_46(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_46(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt20, 5, SQL_C_WCHAR, bind_buf_46.data(), 1024, bind_ind_46.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt20), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt20)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt20), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt20);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt20), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt21 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt21);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt21 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret =
        SQLTables(stmt21, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3, sqlchar(""), -3, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt21), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_47(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_47(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt21, 1, SQL_C_WCHAR, bind_buf_47.data(), 1024, bind_ind_47.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt21), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_48(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_48(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt21, 2, SQL_C_WCHAR, bind_buf_48.data(), 1024, bind_ind_48.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt21), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_49(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_49(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt21, 3, SQL_C_WCHAR, bind_buf_49.data(), 1024, bind_ind_49.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt21), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_50(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_50(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt21, 4, SQL_C_WCHAR, bind_buf_50.data(), 1024, bind_ind_50.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt21), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_51(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_51(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt21, 5, SQL_C_WCHAR, bind_buf_51.data(), 1024, bind_ind_51.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt21), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt21);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt21), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt21);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt21), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt22 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt22);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt22 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt22, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3, sqlchar("%"), -3,
                              nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_52(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_52(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt22, 1, SQL_C_WCHAR, bind_buf_52.data(), 1024, bind_ind_52.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_53(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_53(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt22, 2, SQL_C_WCHAR, bind_buf_53.data(), 1024, bind_ind_53.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_54(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_54(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt22, 3, SQL_C_WCHAR, bind_buf_54.data(), 1024, bind_ind_54.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_55(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_55(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt22, 4, SQL_C_WCHAR, bind_buf_55.data(), 1024, bind_ind_55.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_56(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_56(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt22, 5, SQL_C_WCHAR, bind_buf_56.data(), 1024, bind_ind_56.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt22);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt22);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt22);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt22);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt22);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt22), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt23 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt23);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt23 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret =
        SQLTables(stmt23, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3, sqlchar(""), -3, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt23), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_57(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_57(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt23, 1, SQL_C_WCHAR, bind_buf_57.data(), 1024, bind_ind_57.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt23), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_58(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_58(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt23, 2, SQL_C_WCHAR, bind_buf_58.data(), 1024, bind_ind_58.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt23), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_59(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_59(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt23, 3, SQL_C_WCHAR, bind_buf_59.data(), 1024, bind_ind_59.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt23), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_60(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_60(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt23, 4, SQL_C_WCHAR, bind_buf_60.data(), 1024, bind_ind_60.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt23), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_61(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_61(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt23, 5, SQL_C_WCHAR, bind_buf_61.data(), 1024, bind_ind_61.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt23), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt23);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt23), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt23);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt23), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt24 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt24);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt24 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt24, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3, sqlchar("%"), -3,
                              nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_62(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_62(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt24, 1, SQL_C_WCHAR, bind_buf_62.data(), 1024, bind_ind_62.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_63(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_63(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt24, 2, SQL_C_WCHAR, bind_buf_63.data(), 1024, bind_ind_63.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_64(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_64(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt24, 3, SQL_C_WCHAR, bind_buf_64.data(), 1024, bind_ind_64.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_65(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_65(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt24, 4, SQL_C_WCHAR, bind_buf_65.data(), 1024, bind_ind_65.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_66(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_66(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt24, 5, SQL_C_WCHAR, bind_buf_66.data(), 1024, bind_ind_66.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt24);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt24);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt24);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt24);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt24);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt24), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt25 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt25);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt25 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret =
        SQLTables(stmt25, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3, nullptr, 0, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_67(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_67(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt25, 1, SQL_C_WCHAR, bind_buf_67.data(), 1024, bind_ind_67.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_68(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_68(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt25, 2, SQL_C_WCHAR, bind_buf_68.data(), 1024, bind_ind_68.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_69(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_69(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt25, 3, SQL_C_WCHAR, bind_buf_69.data(), 1024, bind_ind_69.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_70(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_70(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt25, 4, SQL_C_WCHAR, bind_buf_70.data(), 1024, bind_ind_70.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_71(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_71(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt25, 5, SQL_C_WCHAR, bind_buf_71.data(), 1024, bind_ind_71.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt25);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt25);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt25);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt25);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt25);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt25), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt26 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt26);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt26 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt26, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3,
                              sqlchar("ALLDATATYPESNAV"), -3, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt26), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_72(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_72(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt26, 1, SQL_C_WCHAR, bind_buf_72.data(), 1024, bind_ind_72.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt26), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_73(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_73(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt26, 2, SQL_C_WCHAR, bind_buf_73.data(), 1024, bind_ind_73.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt26), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_74(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_74(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt26, 3, SQL_C_WCHAR, bind_buf_74.data(), 1024, bind_ind_74.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt26), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_75(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_75(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt26, 4, SQL_C_WCHAR, bind_buf_75.data(), 1024, bind_ind_75.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt26), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_76(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_76(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt26, 5, SQL_C_WCHAR, bind_buf_76.data(), 1024, bind_ind_76.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt26), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt26);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt26), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt26);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt26), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt26);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt26), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt27 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt27);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt27 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt27, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3,
                              sqlchar("ALLDATATYPESNAV"), -3, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt27), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_77(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_77(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt27, 1, SQL_C_WCHAR, bind_buf_77.data(), 1024, bind_ind_77.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt27), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_78(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_78(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt27, 2, SQL_C_WCHAR, bind_buf_78.data(), 1024, bind_ind_78.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt27), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_79(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_79(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt27, 3, SQL_C_WCHAR, bind_buf_79.data(), 1024, bind_ind_79.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt27), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_80(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_80(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt27, 4, SQL_C_WCHAR, bind_buf_80.data(), 1024, bind_ind_80.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt27), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_81(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_81(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt27, 5, SQL_C_WCHAR, bind_buf_81.data(), 1024, bind_ind_81.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt27), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt27);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt27), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt27);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt27), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt27);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt27), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt28 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt28);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt28 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt28, sqlchar("SELECT *\nFROM \"DATATYPETESTS\".\"ALLDATATYPESNAV\""), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt28, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numCols == 25);
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt28, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numCols == 25);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 24);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 24);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 24);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 256);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16777216);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 10);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -2);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 32);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -2);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16777216);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -7);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 1);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_DATE);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 10);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIME);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 18);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 29);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 29);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 29);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt28, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt28);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt28), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt29 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt29);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt29 != SQL_NULL_HSTMT);
  }

  // SQLForeignKeys
  {
    SQLRETURN ret = SQLForeignKeys(stmt29, nullptr, 0, nullptr, 0, nullptr, 0, sqlchar("ODBCMETADATATESTDB"), -3,
                                   sqlchar("DATATYPETESTS"), -3, sqlchar("ALLDATATYPESNAV"), -3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt29, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numCols == 14);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt29, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt29, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)102, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_82 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt29, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_82, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_83(102, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt29, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_83.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_84(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_84(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 1, SQL_C_WCHAR, bind_buf_84.data(), 1022, bind_ind_84.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_85(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_85(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 2, SQL_C_WCHAR, bind_buf_85.data(), 1022, bind_ind_85.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_86(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_86(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 3, SQL_C_WCHAR, bind_buf_86.data(), 1022, bind_ind_86.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_87(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_87(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 4, SQL_C_WCHAR, bind_buf_87.data(), 1022, bind_ind_87.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_88(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_88(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 5, SQL_C_WCHAR, bind_buf_88.data(), 1022, bind_ind_88.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 6
  std::vector<char> bind_buf_89(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_89(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 6, SQL_C_WCHAR, bind_buf_89.data(), 1022, bind_ind_89.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 7
  std::vector<char> bind_buf_90(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_90(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 7, SQL_C_WCHAR, bind_buf_90.data(), 1022, bind_ind_90.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 8
  std::vector<char> bind_buf_91(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_91(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 8, SQL_C_WCHAR, bind_buf_91.data(), 1022, bind_ind_91.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 9
  std::vector<char> bind_buf_92(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_92(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 9, SQL_C_SSHORT, bind_buf_92.data(), 2, bind_ind_92.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 10
  std::vector<char> bind_buf_93(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_93(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 10, SQL_C_SSHORT, bind_buf_93.data(), 2, bind_ind_93.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 11
  std::vector<char> bind_buf_94(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_94(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 11, SQL_C_SSHORT, bind_buf_94.data(), 2, bind_ind_94.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 12
  std::vector<char> bind_buf_95(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_95(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 12, SQL_C_WCHAR, bind_buf_95.data(), 1022, bind_ind_95.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 13
  std::vector<char> bind_buf_96(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_96(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 13, SQL_C_WCHAR, bind_buf_96.data(), 1022, bind_ind_96.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 14
  std::vector<char> bind_buf_97(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_97(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 14, SQL_C_SSHORT, bind_buf_97.data(), 2, bind_ind_97.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt29);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_98(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_98(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 1, SQL_C_WCHAR, bind_buf_98.data(), 1022, bind_ind_98.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_99(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_99(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 2, SQL_C_WCHAR, bind_buf_99.data(), 1022, bind_ind_99.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_100(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_100(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 3, SQL_C_WCHAR, bind_buf_100.data(), 1022, bind_ind_100.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_101(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_101(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 4, SQL_C_WCHAR, bind_buf_101.data(), 1022, bind_ind_101.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_102(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_102(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 5, SQL_C_WCHAR, bind_buf_102.data(), 1022, bind_ind_102.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 6
  std::vector<char> bind_buf_103(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_103(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 6, SQL_C_WCHAR, bind_buf_103.data(), 1022, bind_ind_103.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 7
  std::vector<char> bind_buf_104(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_104(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 7, SQL_C_WCHAR, bind_buf_104.data(), 1022, bind_ind_104.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 8
  std::vector<char> bind_buf_105(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_105(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 8, SQL_C_WCHAR, bind_buf_105.data(), 1022, bind_ind_105.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 9
  std::vector<char> bind_buf_106(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_106(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 9, SQL_C_SSHORT, bind_buf_106.data(), 2, bind_ind_106.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 10
  std::vector<char> bind_buf_107(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_107(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 10, SQL_C_SSHORT, bind_buf_107.data(), 2, bind_ind_107.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 11
  std::vector<char> bind_buf_108(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_108(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 11, SQL_C_SSHORT, bind_buf_108.data(), 2, bind_ind_108.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 12
  std::vector<char> bind_buf_109(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_109(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 12, SQL_C_WCHAR, bind_buf_109.data(), 1022, bind_ind_109.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 13
  std::vector<char> bind_buf_110(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_110(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 13, SQL_C_WCHAR, bind_buf_110.data(), 1022, bind_ind_110.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 14
  std::vector<char> bind_buf_111(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_111(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt29, 14, SQL_C_SSHORT, bind_buf_111.data(), 2, bind_ind_111.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt29);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt29), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt30 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt30);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt30 != SQL_NULL_HSTMT);
  }

  // SQLPrimaryKeys
  {
    SQLRETURN ret = SQLPrimaryKeys(stmt30, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3,
                                   sqlchar("ALLDATATYPESNAV"), -3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt30, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numCols == 6);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt30, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt30, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)205, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_112 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt30, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_112, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_113(205, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt30, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_113.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_114(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_114(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 1, SQL_C_WCHAR, bind_buf_114.data(), 1022, bind_ind_114.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_115(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_115(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 2, SQL_C_WCHAR, bind_buf_115.data(), 1022, bind_ind_115.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_116(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_116(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 3, SQL_C_WCHAR, bind_buf_116.data(), 1022, bind_ind_116.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_117(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_117(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 4, SQL_C_WCHAR, bind_buf_117.data(), 1022, bind_ind_117.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_118(205 * 2, 0);
  std::vector<SQLLEN> bind_ind_118(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 5, SQL_C_SSHORT, bind_buf_118.data(), 2, bind_ind_118.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 6
  std::vector<char> bind_buf_119(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_119(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 6, SQL_C_WCHAR, bind_buf_119.data(), 1022, bind_ind_119.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt30);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_120(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_120(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 1, SQL_C_WCHAR, bind_buf_120.data(), 1022, bind_ind_120.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_121(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_121(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 2, SQL_C_WCHAR, bind_buf_121.data(), 1022, bind_ind_121.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_122(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_122(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 3, SQL_C_WCHAR, bind_buf_122.data(), 1022, bind_ind_122.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_123(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_123(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 4, SQL_C_WCHAR, bind_buf_123.data(), 1022, bind_ind_123.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_124(205 * 2, 0);
  std::vector<SQLLEN> bind_ind_124(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 5, SQL_C_SSHORT, bind_buf_124.data(), 2, bind_ind_124.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 6
  std::vector<char> bind_buf_125(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_125(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt30, 6, SQL_C_WCHAR, bind_buf_125.data(), 1022, bind_ind_125.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt30);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt30), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt31 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt31);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt31 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt31, sqlchar("SELECT 1"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt31);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt31, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt31, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt31, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_126 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt31, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_126, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_127(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt31, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_127.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_128(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_128(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt31, 1, SQL_C_SBIGINT, bind_buf_128.data(), 8, bind_ind_128.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt31);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt31);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_129(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_129(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt31, 1, SQL_C_SBIGINT, bind_buf_129.data(), 8, bind_ind_129.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt31);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt31), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt32 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt32);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt32 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt32,
                               sqlchar("SELECT SUM(1) AS \"COL\"\nFROM \"DATATYPETESTS\".\"ALLDATATYPESNAV\" "
                                       "\"ALLDATATYPESNAV\"\nHAVING COUNT(1)>0"),
                               SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt32);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt32, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt32, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt32, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)131072, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_130 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt32, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_130, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_131(131072, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt32, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_131.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_132(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_132(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt32, 1, SQL_C_SBIGINT, bind_buf_132.data(), 8, bind_ind_132.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt32);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt32);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_133(131072 * 8, 0);
  std::vector<SQLLEN> bind_ind_133(131072, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt32, 1, SQL_C_SBIGINT, bind_buf_133.data(), 8, bind_ind_133.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt32);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt32), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt33 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt33);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt33 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt33, sqlchar("ODBCMETADATATESTDB"), -3, nullptr, 0, nullptr, 0, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt33), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_134(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_134(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt33, 1, SQL_C_WCHAR, bind_buf_134.data(), 1024, bind_ind_134.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt33), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_135(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_135(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt33, 2, SQL_C_WCHAR, bind_buf_135.data(), 1024, bind_ind_135.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt33), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_136(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_136(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt33, 3, SQL_C_WCHAR, bind_buf_136.data(), 1024, bind_ind_136.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt33), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_137(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_137(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt33, 4, SQL_C_WCHAR, bind_buf_137.data(), 1024, bind_ind_137.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt33), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_138(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_138(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt33, 5, SQL_C_WCHAR, bind_buf_138.data(), 1024, bind_ind_138.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt33), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt33)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt33), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt33);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt33), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt34 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt34);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt34 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt34,
                               sqlchar("SELECT \"ALLDATATYPESNAV\".\"ROWKIND\" AS \"ROWKIND\"\nFROM "
                                       "\"DATATYPETESTS\".\"ALLDATATYPESNAV\" \"ALLDATATYPESNAV\"\nGROUP BY 1"),
                               SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt34);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt34, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt34, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt34, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)30840, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_139 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt34, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_139, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_140(30840, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt34, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_140.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_141(30840 * 34, 0);
  std::vector<SQLLEN> bind_ind_141(30840, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt34, 1, SQL_C_WCHAR, bind_buf_141.data(), 34, bind_ind_141.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt34);
    // BufferLength 34 holds 16 characters plus a NUL when SQLWCHAR is 2 bytes,
    // and 7 characters plus a NUL under iODBC, where SQLWCHAR is 4 bytes.
    // BOUNDARY is 8 characters, so that row truncates.
    NON_IODBC { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess()); }
    IODBC_ONLY {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt34);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_142(30840 * 34, 0);
  std::vector<SQLLEN> bind_ind_142(30840, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt34, 1, SQL_C_WCHAR, bind_buf_142.data(), 34, bind_ind_142.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt34);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt34), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt35 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt35);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt35 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt35,
        sqlchar("SELECT \"ALLDATATYPESNAV\".\"ROWKIND\" AS \"ROWKIND\",\n  SUM(\"ALLDATATYPESNAV\".\"INTVAL\") AS "
                "\"sum_INTVAL_ok\"\nFROM \"DATATYPETESTS\".\"ALLDATATYPESNAV\" \"ALLDATATYPESNAV\"\nGROUP BY 1"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt35);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt35, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
    CHECK(numCols == 2);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt35, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt35, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)12945, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_143 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt35, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_143, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_144(12945, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt35, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_144.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_145(12945 * 34, 0);
  std::vector<SQLLEN> bind_ind_145(12945, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt35, 1, SQL_C_WCHAR, bind_buf_145.data(), 34, bind_ind_145.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_146(12945 * 47, 0);
  std::vector<SQLLEN> bind_ind_146(12945, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt35, 2, SQL_C_CHAR, bind_buf_146.data(), 47, bind_ind_146.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt35);
    // BufferLength 34 holds 16 characters plus a NUL when SQLWCHAR is 2 bytes,
    // and 7 characters plus a NUL under iODBC, where SQLWCHAR is 4 bytes.
    // BOUNDARY is 8 characters, so that row truncates.
    NON_IODBC { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess()); }
    IODBC_ONLY {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt35);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_147(12945 * 34, 0);
  std::vector<SQLLEN> bind_ind_147(12945, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt35, 1, SQL_C_WCHAR, bind_buf_147.data(), 34, bind_ind_147.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_148(12945 * 47, 0);
  std::vector<SQLLEN> bind_ind_148(12945, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt35, 2, SQL_C_CHAR, bind_buf_148.data(), 47, bind_ind_148.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt35);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt35), OdbcMatchers::IsSuccess());
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

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env6), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
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

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
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

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env5), OdbcMatchers::IsSuccess());
  }

  // skipped 58 SQLColAttribute call(s) with undocumented field id
}
