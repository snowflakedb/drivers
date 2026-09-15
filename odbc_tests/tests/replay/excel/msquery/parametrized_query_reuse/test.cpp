#include <algorithm>
#include <chrono>
#include <cstring>
#include <string>
#include <thread>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "WideString.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: excel msquery parametrized_query_reuse", "[excel][msquery]") {
  SKIP_IODBC("Excel MS Query replays use the Windows ODBC Driver Manager");
  // SNOW-4082442: SQLFetch returns 22003 while writing SQLGetTypeInfo's 2-byte and
  // 8-byte SQL_C_DEFAULT binds for values such as 134217728 and −5. SQLColumns has
  // the same failure on a 2-byte SQL_C_DEFAULT DATA_TYPE such as 12 or −7.
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  auto config = DataSourceConfig::Snowflake().install();

  SQLHENV env0 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env0);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env0 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env0, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc0 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env0, &dbc0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
    REQUIRE(dbc0 != SQL_NULL_HDBC);
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc0, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DATA_SOURCE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATA_SOURCE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt0 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt0 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_ACTIVE_STATEMENTS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_ACTIVE_STATEMENTS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_DATA_SOURCE_READ_ONLY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATA_SOURCE_READ_ONLY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "N");
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_SEARCH_PATTERN_ESCAPE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SEARCH_PATTERN_ESCAPE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\\");
  }

  // SQLGetInfo - SQL_CORRELATION_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CORRELATION_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x2u);
  }

  // SQLGetInfo - SQL_NON_NULLABLE_COLUMNS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_NON_NULLABLE_COLUMNS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_CATALOG_NAME_SEPARATOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_NAME_SEPARATOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == ".");
  }

  // SQLGetInfo - SQL_FILE_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_FILE_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_SEARCH_PATTERN_ESCAPE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SEARCH_PATTERN_ESCAPE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\\");
  }

  // SQLGetInfo - SQL_CATALOG_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "database");
  }

  // SQLGetInfo - SQL_DATABASE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATABASE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt0, sqlchar("%"), -3, sqlchar(""), 0, sqlchar(""), 0, sqlchar(""), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_0(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_0(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 1, SQL_C_CHAR, bind_buf_0.data(), 65, bind_ind_0.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt0)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_MAX_SCHEMA_NAME_LEN
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_MAX_SCHEMA_NAME_LEN, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFu);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt0, sqlchar(""), 0, sqlchar("%"), -3, sqlchar(""), 0, sqlchar(""), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_1(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_1(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 2, SQL_C_CHAR, bind_buf_1.data(), 65, bind_ind_1.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt0)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt0, sqlchar("ODBCMETADATATESTDB"), 18, nullptr, -3, nullptr, -3,
                              sqlchar("'TABLE','VIEW','SYNONYM'"), 24);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_2(1 * 129, 0);
  std::vector<SQLLEN> bind_ind_2(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 3, SQL_C_CHAR, bind_buf_2.data(), 129, bind_ind_2.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_3(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_3(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 4, SQL_C_CHAR, bind_buf_3.data(), 65, bind_ind_3.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_4(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_4(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 2, SQL_C_CHAR, bind_buf_4.data(), 65, bind_ind_4.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt0)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_IDENTIFIER_QUOTE_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_IDENTIFIER_QUOTE_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\"");
  }

  // SQLGetTypeInfo
  {
    SQLRETURN ret = SQLGetTypeInfo(stmt0, SQL_ALL_TYPES);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_5(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_5(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 2, SQL_C_DEFAULT, bind_buf_5.data(), 2, bind_ind_5.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_6(1 * 128, 0);
  std::vector<SQLLEN> bind_ind_6(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 4, SQL_C_CHAR, bind_buf_6.data(), 128, bind_ind_6.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_7(1 * 128, 0);
  std::vector<SQLLEN> bind_ind_7(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 5, SQL_C_CHAR, bind_buf_7.data(), 128, bind_ind_7.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 9
  std::vector<char> bind_buf_8(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_8(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 9, SQL_C_DEFAULT, bind_buf_8.data(), 2, bind_ind_8.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_9(1 * 8, 0);
  std::vector<SQLLEN> bind_ind_9(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 3, SQL_C_DEFAULT, bind_buf_9.data(), 8, bind_ind_9.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 15
  std::vector<char> bind_buf_10(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_10(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 15, SQL_C_DEFAULT, bind_buf_10.data(), 2, bind_ind_10.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 7
  std::vector<char> bind_buf_11(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_11(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 7, SQL_C_DEFAULT, bind_buf_11.data(), 2, bind_ind_11.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_ACTIVE_STATEMENTS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_ACTIVE_STATEMENTS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  SQLHSTMT stmt1 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt1 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DATABASE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATABASE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_TABLE_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_TABLE_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "table");
  }

  // SQLGetInfo - SQL_OWNER_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_OWNER_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "schema");
  }

  // SQLGetInfo - SQL_CATALOG_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "database");
  }

  // SQLColumns
  {
    SQLRETURN ret = SQLColumns(stmt0, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3,
                               sqlchar("ALLDATATYPESNAV"), -3, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_12(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_12(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 4, SQL_C_CHAR, bind_buf_12.data(), 65, bind_ind_12.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_13(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_13(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 5, SQL_C_DEFAULT, bind_buf_13.data(), 2, bind_ind_13.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_RESET_PARAMS);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSpecialColumns
  {
    SQLRETURN ret =
        SQLSpecialColumns(stmt0, SQL_BEST_ROWID, sqlchar("ODBCMETADATATESTDB"), 18, sqlchar("DATATYPETESTS"), 13,
                          sqlchar("ALLDATATYPESNAV"), 15, SQL_SCOPE_CURROW, SQL_NULLABLE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_14(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_14(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 2, SQL_C_CHAR, bind_buf_14.data(), 65, bind_ind_14.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_RESET_PARAMS);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DATABASE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATABASE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_RESET_PARAMS);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt1,
        sqlchar(
            "SELECT ALLDATATYPESNAV.ROWKIND, ALLDATATYPESNAV.INTVAL, ALLDATATYPESNAV.VARCHARVAL\r\nFROM "
            "ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPESNAV ALLDATATYPESNAV\r\nWHERE (ALLDATATYPESNAV.ROWKIND=?)"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindParameter 1 (manual: ROWKIND = 'BOUNDARY'. The WinODBC trace records
  // the parameter buffer pointer but never its bytes, so the bound value is
  // unrecoverable from the LOG. The query is `WHERE ALLDATATYPESNAV.ROWKIND = ?`,
  // an exact match, so a zero-filled buffer matches no rows and the recorded
  // SQL_SUCCESS fetch can't be reproduced. Bind a real ROWKIND value from the
  // fixture (scripts/odbc/setup_readonly_metadata_db.sql) — each ROWKIND yields
  // exactly one row. The 8-byte CHAR buffer in the trace fits 'BOUNDARY'.)
  std::string param_buf_15 = "BOUNDARY";
  SQLLEN param_ind_15 = SQL_NTS;
  {
    SQLRETURN ret = SQLBindParameter(stmt1, 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 134217728, 0,
                                     static_cast<SQLPOINTER>(param_buf_15.data()),
                                     static_cast<SQLLEN>(param_buf_15.size()), &param_ind_15);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt1, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numCols == 3);
  }

  // SQLDescribeCol col 1
  {
    char colName[257] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt1, 1, reinterpret_cast<SQLCHAR*>(colName), 256, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "ROWKIND");
    CHECK(dataType == 12);
    CHECK(colSize == 16);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_COLUMN_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_COLUMN_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16);
  }

  // SQLDescribeCol col 2
  {
    char colName[257] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt1, 2, reinterpret_cast<SQLCHAR*>(colName), 256, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "INTVAL");
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_COLUMN_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_COLUMN_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLDescribeCol col 3
  {
    char colName[257] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt1, 3, reinterpret_cast<SQLCHAR*>(colName), 256, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "VARCHARVAL");
    CHECK(dataType == 12);
    CHECK(colSize == 256);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_COLUMN_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_COLUMN_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 256);
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_16(1 * 17, 0);
  std::vector<SQLLEN> bind_ind_16(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 1, SQL_C_CHAR, bind_buf_16.data(), 17, bind_ind_16.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_17(1 * 137, 0);
  std::vector<SQLLEN> bind_ind_17(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 2, SQL_C_CHAR, bind_buf_17.data(), 137, bind_ind_17.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_18(1 * 256, 0);
  std::vector<SQLLEN> bind_ind_18(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 3, SQL_C_CHAR, bind_buf_18.data(), 256, bind_ind_18.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt1);
    // The bound 'BOUNDARY' row's VARCHARVAL is RPAD('X',256,'X') (256 chars), one
    // byte too long for the trace's 256-byte SQL_C_CHAR buffer once the NUL
    // terminator is counted, so the reference driver truncates and reports
    // SQL_SUCCESS_WITH_INFO (01004) under every driver manager (Windows DM and
    // unixODBC alike).
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1),
               OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_19(1 * 17, 0);
  std::vector<SQLLEN> bind_ind_19(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 1, SQL_C_CHAR, bind_buf_19.data(), 17, bind_ind_19.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_20(1 * 137, 0);
  std::vector<SQLLEN> bind_ind_20(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 2, SQL_C_CHAR, bind_buf_20.data(), 137, bind_ind_20.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_21(1 * 256, 0);
  std::vector<SQLLEN> bind_ind_21(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 3, SQL_C_CHAR, bind_buf_21.data(), 256, bind_ind_21.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_RESET_PARAMS);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
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

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env1, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLSetConnectAttr(dbc1, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)45, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc1, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::Succeeded());
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
    SQLRETURN ret = SQLPrepare(
        stmt2,
        sqlchar(
            "SELECT ALLDATATYPESNAV.ROWKIND, ALLDATATYPESNAV.INTVAL, ALLDATATYPESNAV.VARCHARVAL\r\nFROM "
            "ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPESNAV ALLDATATYPESNAV\r\nWHERE (ALLDATATYPESNAV.ROWKIND=?)"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLBindParameter 1 (manual: ROWKIND = 'NORMAL'; see the note on the first
  // bind. SQL_C_WCHAR here, so encode the fixture value as UTF-16. The trace's
  // 12-byte buffer fits the 6-char 'NORMAL'.)
  std::vector<SQLWCHAR> param_buf_22 = sf::wide::encode_wide(U"NORMAL");
  SQLLEN param_ind_22 = SQL_NTS;
  {
    SQLRETURN ret =
        SQLBindParameter(stmt2, 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 134217728, 0, param_buf_22.data(),
                         static_cast<SQLLEN>(param_buf_22.size() * sizeof(SQLWCHAR)), &param_ind_22);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_ASYNC_ENABLE, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // async execution: the host re-issues the same statement until the
  // server stops reporting SQL_STILL_EXECUTING. The poll count is timing-
  // dependent, so loop rather than pin the trace's count, and sleep between
  // polls: hammering the execute in a tight loop starves the driver's
  // in-flight async work and it never reaches a terminal code. A statement
  // that still has not settled after the bound fails the test rather than
  // looping forever, so a regression in async completion is caught.
  {
    SQLRETURN ret;
    int async_polls = 0;
    do {
      if (async_polls++ >= 300) {
        FAIL("async execute did not reach a terminal code within the poll bound");
      }
      if (async_polls > 1) {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
      }
      ret = SQLExecute(stmt2);
    } while (ret == SQL_STILL_EXECUTING);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_ASYNC_ENABLE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt2, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(numCols == 3);
  }

  // SQLDescribeCol col 1
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt2, 1, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "ROWKIND");
    CHECK(dataType == 12);
    CHECK(colSize == 16);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 2
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt2, 2, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "INTVAL");
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 3
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt2, 3, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "VARCHARVAL");
    CHECK(dataType == 12);
    CHECK(colSize == 256);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)584, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_23 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_23, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> row_buf_stmt2(1 * 584, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt2, 1, SQL_C_WCHAR, row_buf_stmt2.data() + 8, 34,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt2.data() + 0));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  {
    SQLRETURN ret = SQLBindCol(stmt2, 2, SQL_C_DOUBLE, row_buf_stmt2.data() + 52, 8,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt2.data() + 44));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  {
    SQLRETURN ret = SQLBindCol(stmt2, 3, SQL_C_WCHAR, row_buf_stmt2.data() + 68, 514,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt2.data() + 60));
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

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt2, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
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

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env3, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc2 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc2 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc2, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)45, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc2, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::Succeeded());
  }

  SQLHSTMT stmt3 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc2, &stmt3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    REQUIRE(stmt3 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt3,
        sqlchar(
            "SELECT ALLDATATYPESNAV.ROWKIND, ALLDATATYPESNAV.INTVAL, ALLDATATYPESNAV.VARCHARVAL\r\nFROM "
            "ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPESNAV ALLDATATYPESNAV\r\nWHERE (ALLDATATYPESNAV.ROWKIND=?)"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLBindParameter 1 (manual: ROWKIND = 'UNICODE'; see the note on the first
  // bind. The trace's 14-byte WCHAR buffer fits the 7-char 'UNICODE'.)
  std::vector<SQLWCHAR> param_buf_24 = sf::wide::encode_wide(U"UNICODE");
  SQLLEN param_ind_24 = SQL_NTS;
  {
    SQLRETURN ret =
        SQLBindParameter(stmt3, 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 134217728, 0, param_buf_24.data(),
                         static_cast<SQLLEN>(param_buf_24.size() * sizeof(SQLWCHAR)), &param_ind_24);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ASYNC_ENABLE, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // async execution: the host re-issues the same statement until the
  // server stops reporting SQL_STILL_EXECUTING. The poll count is timing-
  // dependent, so loop rather than pin the trace's count, and sleep between
  // polls: hammering the execute in a tight loop starves the driver's
  // in-flight async work and it never reaches a terminal code. A statement
  // that still has not settled after the bound fails the test rather than
  // looping forever, so a regression in async completion is caught.
  {
    SQLRETURN ret;
    int async_polls = 0;
    do {
      if (async_polls++ >= 300) {
        FAIL("async execute did not reach a terminal code within the poll bound");
      }
      if (async_polls > 1) {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
      }
      ret = SQLExecute(stmt3);
    } while (ret == SQL_STILL_EXECUTING);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ASYNC_ENABLE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt3, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numCols == 3);
  }

  // SQLDescribeCol col 1
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt3, 1, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "ROWKIND");
    CHECK(dataType == 12);
    CHECK(colSize == 16);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 2
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt3, 2, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "INTVAL");
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 3
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt3, 3, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "VARCHARVAL");
    CHECK(dataType == 12);
    CHECK(colSize == 256);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)584, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_25 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_25, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> row_buf_stmt3(1 * 584, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt3, 1, SQL_C_WCHAR, row_buf_stmt3.data() + 8, 34,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt3.data() + 0));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  {
    SQLRETURN ret = SQLBindCol(stmt3, 2, SQL_C_DOUBLE, row_buf_stmt3.data() + 52, 8,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt3.data() + 44));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  {
    SQLRETURN ret = SQLBindCol(stmt3, 3, SQL_C_WCHAR, row_buf_stmt3.data() + 68, 514,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt3.data() + 60));
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

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt3, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
  }

  SQLHENV env4 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env4);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env4 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env4, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env4), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc3 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env4, &dbc3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env4), OdbcMatchers::IsSuccess());
    REQUIRE(dbc3 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc3, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)45, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc3, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
  }

  SQLHSTMT stmt4 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt4);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt4 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt4,
        sqlchar(
            "SELECT ALLDATATYPESNAV.ROWKIND, ALLDATATYPESNAV.INTVAL, ALLDATATYPESNAV.VARCHARVAL\r\nFROM "
            "ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPESNAV ALLDATATYPESNAV\r\nWHERE (ALLDATATYPESNAV.ROWKIND=?)"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLBindParameter 1 (manual: ROWKIND = 'MISSING'. Unlike the other executes,
  // the trace records this statement's fetch returning SQL_NO_DATA with no rows
  // (the "parameter reuse" scenario re-runs the query with a value that matches
  // nothing). Bind a 7-char value that is deliberately absent from the fixture's
  // ROWKIND domain {NORMAL,BOUNDARY,UNICODE,NULLROW} so the empty result set is
  // reproduced. The trace's 14-byte WCHAR buffer fits the 7-char 'MISSING'.)
  std::vector<SQLWCHAR> param_buf_26 = sf::wide::encode_wide(U"MISSING");
  SQLLEN param_ind_26 = SQL_NTS;
  {
    SQLRETURN ret =
        SQLBindParameter(stmt4, 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 134217728, 0, param_buf_26.data(),
                         static_cast<SQLLEN>(param_buf_26.size() * sizeof(SQLWCHAR)), &param_ind_26);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ASYNC_ENABLE, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // async execution: the host re-issues the same statement until the
  // server stops reporting SQL_STILL_EXECUTING. The poll count is timing-
  // dependent, so loop rather than pin the trace's count, and sleep between
  // polls: hammering the execute in a tight loop starves the driver's
  // in-flight async work and it never reaches a terminal code. A statement
  // that still has not settled after the bound fails the test rather than
  // looping forever, so a regression in async completion is caught.
  {
    SQLRETURN ret;
    int async_polls = 0;
    do {
      if (async_polls++ >= 300) {
        FAIL("async execute did not reach a terminal code within the poll bound");
      }
      if (async_polls > 1) {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
      }
      ret = SQLExecute(stmt4);
    } while (ret == SQL_STILL_EXECUTING);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ASYNC_ENABLE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt4, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numCols == 3);
  }

  // SQLDescribeCol col 1
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt4, 1, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "ROWKIND");
    CHECK(dataType == 12);
    CHECK(colSize == 16);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 2
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt4, 2, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "INTVAL");
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 3
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt4, 3, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "VARCHARVAL");
    CHECK(dataType == 12);
    CHECK(colSize == 256);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)584, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_27 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_27, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> row_buf_stmt4(1 * 584, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt4, 1, SQL_C_WCHAR, row_buf_stmt4.data() + 8, 34,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt4.data() + 0));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  {
    SQLRETURN ret = SQLBindCol(stmt4, 2, SQL_C_DOUBLE, row_buf_stmt4.data() + 52, 8,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt4.data() + 44));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  {
    SQLRETURN ret = SQLBindCol(stmt4, 3, SQL_C_WCHAR, row_buf_stmt4.data() + 68, 514,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt4.data() + 60));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt4, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
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

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env5, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env5), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc4 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env5, &dbc4);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env5), OdbcMatchers::IsSuccess());
    REQUIRE(dbc4 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc4, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)45, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc4, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::Succeeded());
  }

  SQLHSTMT stmt5 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc4, &stmt5);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc4), OdbcMatchers::IsSuccess());
    REQUIRE(stmt5 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt5,
        sqlchar(
            "SELECT ALLDATATYPESNAV.ROWKIND, ALLDATATYPESNAV.INTVAL, ALLDATATYPESNAV.VARCHARVAL\r\nFROM "
            "ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPESNAV ALLDATATYPESNAV\r\nWHERE (ALLDATATYPESNAV.ROWKIND=?)"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLBindParameter 1 (manual: ROWKIND = 'BOUNDARY'; see the note on the first
  // bind. The trace's 16-byte WCHAR buffer fits the 8-char 'BOUNDARY'.)
  std::vector<SQLWCHAR> param_buf_28 = sf::wide::encode_wide(U"BOUNDARY");
  SQLLEN param_ind_28 = SQL_NTS;
  {
    SQLRETURN ret =
        SQLBindParameter(stmt5, 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 134217728, 0, param_buf_28.data(),
                         static_cast<SQLLEN>(param_buf_28.size() * sizeof(SQLWCHAR)), &param_ind_28);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt5, SQL_ATTR_ASYNC_ENABLE, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // async execution: the host re-issues the same statement until the
  // server stops reporting SQL_STILL_EXECUTING. The poll count is timing-
  // dependent, so loop rather than pin the trace's count, and sleep between
  // polls: hammering the execute in a tight loop starves the driver's
  // in-flight async work and it never reaches a terminal code. A statement
  // that still has not settled after the bound fails the test rather than
  // looping forever, so a regression in async completion is caught.
  {
    SQLRETURN ret;
    int async_polls = 0;
    do {
      if (async_polls++ >= 300) {
        FAIL("async execute did not reach a terminal code within the poll bound");
      }
      if (async_polls > 1) {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
      }
      ret = SQLExecute(stmt5);
    } while (ret == SQL_STILL_EXECUTING);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt5, SQL_ATTR_ASYNC_ENABLE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt5, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(numCols == 3);
  }

  // SQLDescribeCol col 1
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt5, 1, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "ROWKIND");
    CHECK(dataType == 12);
    CHECK(colSize == 16);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 2
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt5, 2, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "INTVAL");
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 3
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt5, 3, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "VARCHARVAL");
    CHECK(dataType == 12);
    CHECK(colSize == 256);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt5, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)584, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_29 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt5, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_29, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> row_buf_stmt5(1 * 584, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt5, 1, SQL_C_WCHAR, row_buf_stmt5.data() + 8, 34,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt5.data() + 0));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  {
    SQLRETURN ret = SQLBindCol(stmt5, 2, SQL_C_DOUBLE, row_buf_stmt5.data() + 52, 8,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt5.data() + 44));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  {
    SQLRETURN ret = SQLBindCol(stmt5, 3, SQL_C_WCHAR, row_buf_stmt5.data() + 68, 514,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt5.data() + 60));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt5, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
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

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env5), OdbcMatchers::IsSuccess());
  }

  SQLHENV env6 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env6);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env6 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env6, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env6), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc5 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env6, &dbc5);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env6), OdbcMatchers::IsSuccess());
    REQUIRE(dbc5 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc5, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)45, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc5, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::Succeeded());
  }

  SQLHSTMT stmt6 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc5, &stmt6);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc5), OdbcMatchers::IsSuccess());
    REQUIRE(stmt6 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt6,
        sqlchar(
            "SELECT ALLDATATYPESNAV.ROWKIND, ALLDATATYPESNAV.INTVAL, ALLDATATYPESNAV.VARCHARVAL\r\nFROM "
            "ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPESNAV ALLDATATYPESNAV\r\nWHERE (ALLDATATYPESNAV.ROWKIND=?)"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindParameter 1 (manual: ROWKIND = 'NULLROW'; see the note on the first
  // bind. The trace's 14-byte WCHAR buffer fits the 7-char 'NULLROW'.)
  std::vector<SQLWCHAR> param_buf_30 = sf::wide::encode_wide(U"NULLROW");
  SQLLEN param_ind_30 = SQL_NTS;
  {
    SQLRETURN ret =
        SQLBindParameter(stmt6, 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 134217728, 0, param_buf_30.data(),
                         static_cast<SQLLEN>(param_buf_30.size() * sizeof(SQLWCHAR)), &param_ind_30);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt6, SQL_ATTR_ASYNC_ENABLE, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // async execution: the host re-issues the same statement until the
  // server stops reporting SQL_STILL_EXECUTING. The poll count is timing-
  // dependent, so loop rather than pin the trace's count, and sleep between
  // polls: hammering the execute in a tight loop starves the driver's
  // in-flight async work and it never reaches a terminal code. A statement
  // that still has not settled after the bound fails the test rather than
  // looping forever, so a regression in async completion is caught.
  {
    SQLRETURN ret;
    int async_polls = 0;
    do {
      if (async_polls++ >= 300) {
        FAIL("async execute did not reach a terminal code within the poll bound");
      }
      if (async_polls > 1) {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
      }
      ret = SQLExecute(stmt6);
    } while (ret == SQL_STILL_EXECUTING);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt6, SQL_ATTR_ASYNC_ENABLE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt6, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numCols == 3);
  }

  // SQLDescribeCol col 1
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt6, 1, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "ROWKIND");
    CHECK(dataType == 12);
    CHECK(colSize == 16);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 2
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt6, 2, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "INTVAL");
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 3
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt6, 3, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "VARCHARVAL");
    CHECK(dataType == 12);
    CHECK(colSize == 256);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt6, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)584, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_31 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt6, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_31, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> row_buf_stmt6(1 * 584, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt6, 1, SQL_C_WCHAR, row_buf_stmt6.data() + 8, 34,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt6.data() + 0));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  {
    SQLRETURN ret = SQLBindCol(stmt6, 2, SQL_C_DOUBLE, row_buf_stmt6.data() + 52, 8,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt6.data() + 44));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  {
    SQLRETURN ret = SQLBindCol(stmt6, 3, SQL_C_WCHAR, row_buf_stmt6.data() + 68, 514,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt6.data() + 60));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt6, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
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

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env6), OdbcMatchers::IsSuccess());
  }

  // --- Replay-only env cleanup (not present in the original trace) ---
  // ODBC-consuming hosts (Excel, Power Query, ...) deliberately leave their
  // SQL_HANDLE_ENV handles allocated at shutdown — the pool root for
  // `SQL_ATTR_CONNECTION_POOLING` is anchored on the env, and any teardown
  // done during DllMain(DLL_PROCESS_DETACH) is invisible to the trace logger.
  // Our replay binary runs many tests in one process, so we explicitly free
  // each leaked env here to avoid leaking pooled connections across tests.
  // SQLFreeHandle - SQLHENV (env2, replay-only)
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
  }
}
