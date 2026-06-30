#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: excel vba_ado sqlbindcol", "[excel][vba_ado][sqlbindcol]") {
  // Blocked on the new driver. Required to unblock:
  //   - SQLGetInfo info types (S1C00 - unknown): SQL_ACTIVE_STATEMENTS,
  //     SQL_DATABASE_NAME, SQL_DEFAULT_TXN_ISOLATION, SQL_TXN_CAPABLE,
  //     SQL_TXN_ISOLATION_OPTION, SQL_SCROLL_OPTIONS, SQL_SCROLL_CONCURRENCY,
  //     SQL_POS_OPERATIONS, SQL_LOCK_TYPES, SQL_STATIC_SENSITIVITY,
  //     SQL_BOOKMARK_PERSISTENCE, SQL_MULT_RESULT_SETS, SQL_NEED_LONG_DATA_LEN,
  //     SQL_FORWARD_ONLY_CURSOR_ATTRIBUTES1, SQL_STATIC_CURSOR_ATTRIBUTES1,
  //     SQL_STATIC_CURSOR_ATTRIBUTES2, SQL_KEYSET_CURSOR_ATTRIBUTES1,
  //     SQL_KEYSET_CURSOR_ATTRIBUTES2.
  //   - SQLColAttribute fields (S1092 - unknown): SQL_DESC_LABEL, SQL_DESC_UPDATABLE.
  // Re-run replay_excel against the new driver and remove this skip once implemented.
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

  // SQLSetEnvAttr - SQL_ATTR_CONNECTION_POOLING
  {
    SQLRETURN ret = SQLSetEnvAttr(env1, SQL_ATTR_CONNECTION_POOLING, nullptr, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc0 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env1, &dbc0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
    REQUIRE(dbc0 != SQL_NULL_HDBC);
  }

  // SQLGetInfo - SQL_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)15, -6);
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

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_POS_OPERATIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_POS_OPERATIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_STATIC_SENSITIVITY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_STATIC_SENSITIVITY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_LOCK_TYPES
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_LOCK_TYPES, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x2u);
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLGetInfo - SQL_TXN_ISOLATION_OPTION
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_TXN_ISOLATION_OPTION, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x2u);
  }

  // SQLGetInfo - SQL_BOOKMARK_PERSISTENCE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_BOOKMARK_PERSISTENCE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_SCROLL_OPTIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SCROLL_OPTIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_SCROLL_CONCURRENCY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SCROLL_CONCURRENCY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_DYNAMIC_CURSOR_ATTRIBUTES1
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DYNAMIC_CURSOR_ATTRIBUTES1, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_KEYSET_CURSOR_ATTRIBUTES1
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_KEYSET_CURSOR_ATTRIBUTES1, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_STATIC_CURSOR_ATTRIBUTES1
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_STATIC_CURSOR_ATTRIBUTES1, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_FORWARD_ONLY_CURSOR_ATTRIBUTES1
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_FORWARD_ONLY_CURSOR_ATTRIBUTES1, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_KEYSET_CURSOR_ATTRIBUTES2
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_KEYSET_CURSOR_ATTRIBUTES2, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_STATIC_CURSOR_ATTRIBUTES2
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_STATIC_CURSOR_ATTRIBUTES2, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_NEED_LONG_DATA_LEN
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_NEED_LONG_DATA_LEN, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "N");
  }

  // SQLGetInfo - SQL_DATABASE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATABASE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
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

  // SQLGetInfo - SQL_CURSOR_ROLLBACK_BEHAVIOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CURSOR_ROLLBACK_BEHAVIOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
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

  // SQLSetConnectAttr - SQL_ATTR_MAX_ROWS
  // Class A divergence: the Windows DM accepts this ODBC 2.x statement attribute
  // set on a connection handle; unixODBC forwards it and the Snowflake driver
  // rejects it (S1092). Assert the platform-correct outcome on each side.
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_MAX_ROWS, nullptr, -6);
    WINDOWS_ONLY { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess()); }
    UNIX_ONLY {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("S1092"));
    }
  }

  // SQLSetConnectAttr - SQL_ATTR_QUERY_TIMEOUT
  // Class A divergence: same as SQL_ATTR_MAX_ROWS above - accepted by the Windows
  // DM, rejected by the driver under unixODBC (S1092).
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_QUERY_TIMEOUT, nullptr, -6);
    WINDOWS_ONLY { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess()); }
    UNIX_ONLY {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("S1092"));
    }
  }

  // SQLGetInfo - SQL_DBMS_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DBMS_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
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

  // SQLGetInfo - SQL_DEFAULT_TXN_ISOLATION
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DEFAULT_TXN_ISOLATION, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x2u);
  }

  SQLHSTMT stmt0 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt0 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_PARAM_BIND_TYPE
  // Class A divergence: SQL_DESC_BIND_TYPE = 10 is accepted by the Windows DM but
  // rejected by the SimbaEngine SDK descriptor validator under 64-bit unixODBC
  // (value must be 0 or divisible by alignof(SQLLEN) = 8), surfaced as 11700.
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_PARAM_BIND_TYPE, (SQLPOINTER)10, 0);
    WINDOWS_ONLY { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess()); }
    UNIX_ONLY {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0),
                 OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("S1000"));
    }
  }

  // SQLSetStmtAttr - SQL_ATTR_PARAM_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_PARAM_BIND_TYPE, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_0 = 0;
  // SQLSetStmtAttr - SQL_ATTR_PARAM_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_PARAM_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_0, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_PARAM_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_PARAM_BIND_OFFSET_PTR, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_MULT_RESULT_SETS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_MULT_RESULT_SETS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "N");
  }

  // SQLSetStmtAttr - SQL_ATTR_QUERY_TIMEOUT
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_QUERY_TIMEOUT, (SQLPOINTER)30, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_PARAMSET_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)1, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt0,
                                  sqlchar("SELECT * REPLACE(\n  DATEADD('day', -1, TSLTZ) AS TSLTZ,\n  DATEADD('day', "
                                          "-1, TSTZ)  AS TSTZ,\n  TO_VARCHAR(NUM38)         AS NUM38\n)\nFROM "
                                          "ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPES\nORDER BY ROWKIND;"),
                                  SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLRowCount
  {
    SQLLEN rowCount = 0;
    SQLRETURN ret = SQLRowCount(stmt0, &rowCount);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(rowCount == 4);
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 25);
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 25);
  }

  // SQLDescribeCol col 1
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 1, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 12);
    CHECK(colSize == 16);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 2
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 2, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 3
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 3, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 4
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 4, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 5
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 5, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 3);
    CHECK(colSize == 38);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 6
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 6, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 12);
    CHECK(colSize == 134217728);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 7
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 7, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 3);
    CHECK(colSize == 18);
    CHECK(scale == 6);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 8
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 8, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 8);
    CHECK(colSize == 53);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 9
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 9, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 8);
    CHECK(colSize == 53);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 10
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 10, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 8);
    CHECK(colSize == 53);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 11
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 11, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 12);
    CHECK(colSize == 256);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 12
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 12, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 13
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 13, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 12);
    CHECK(colSize == 10);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 14
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 14, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == -2);
    CHECK(colSize == 16);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 15
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 15, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == -2);
    CHECK(colSize == 8388608);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 16
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 16, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == -7);
    CHECK(colSize == 1);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 17
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 17, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 9);
    CHECK(colSize == 10);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 18
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 18, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 10);
    CHECK(colSize == 18);
    CHECK(scale == 9);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 19
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 19, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 11);
    CHECK(colSize == 29);
    CHECK(scale == 9);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 20
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 20, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 11);
    CHECK(colSize == 29);
    CHECK(scale == 9);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 21
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 21, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 11);
    CHECK(colSize == 29);
    CHECK(scale == 9);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 22
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 22, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 12);
    CHECK(colSize == 134217728);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 23
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 23, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 12);
    CHECK(colSize == 134217728);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 24
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 24, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 12);
    CHECK(colSize == 134217728);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 25
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 25, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 12);
    CHECK(colSize == 134217728);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt0, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  SQLULEN attr_ptr_1 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_1, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_ROW_BIND_OFFSET_PTR, nullptr, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_RETRIEVE_DATA
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_RETRIEVE_DATA, nullptr, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_RETRIEVE_DATA
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_RETRIEVE_DATA, (SQLPOINTER)1, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)1, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_2 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_2, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_3(1 * 17, 0);
  std::vector<SQLLEN> bind_ind_3(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 1, SQL_C_CHAR, bind_buf_3.data(), 17, bind_ind_3.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_4(1 * 19, 0);
  std::vector<SQLLEN> bind_ind_4(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 2, SQL_C_NUMERIC, bind_buf_4.data(), 19, bind_ind_4.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_5(1 * 19, 0);
  std::vector<SQLLEN> bind_ind_5(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 3, SQL_C_NUMERIC, bind_buf_5.data(), 19, bind_ind_5.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_6(1 * 19, 0);
  std::vector<SQLLEN> bind_ind_6(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 4, SQL_C_NUMERIC, bind_buf_6.data(), 19, bind_ind_6.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_7(1 * 19, 0);
  std::vector<SQLLEN> bind_ind_7(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 5, SQL_C_NUMERIC, bind_buf_7.data(), 19, bind_ind_7.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 7
  std::vector<char> bind_buf_8(1 * 19, 0);
  std::vector<SQLLEN> bind_ind_8(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 7, SQL_C_NUMERIC, bind_buf_8.data(), 19, bind_ind_8.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 8
  std::vector<char> bind_buf_9(1 * 8, 0);
  std::vector<SQLLEN> bind_ind_9(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 8, SQL_C_DOUBLE, bind_buf_9.data(), 8, bind_ind_9.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 9
  std::vector<char> bind_buf_10(1 * 8, 0);
  std::vector<SQLLEN> bind_ind_10(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 9, SQL_C_DOUBLE, bind_buf_10.data(), 8, bind_ind_10.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 10
  std::vector<char> bind_buf_11(1 * 8, 0);
  std::vector<SQLLEN> bind_ind_11(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 10, SQL_C_DOUBLE, bind_buf_11.data(), 8, bind_ind_11.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 11
  std::vector<char> bind_buf_12(1 * 257, 0);
  std::vector<SQLLEN> bind_ind_12(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 11, SQL_C_CHAR, bind_buf_12.data(), 257, bind_ind_12.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 13
  std::vector<char> bind_buf_13(1 * 11, 0);
  std::vector<SQLLEN> bind_ind_13(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 13, SQL_C_CHAR, bind_buf_13.data(), 11, bind_ind_13.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 14
  std::vector<char> bind_buf_14(1 * 16, 0);
  std::vector<SQLLEN> bind_ind_14(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 14, SQL_C_BINARY, bind_buf_14.data(), 16, bind_ind_14.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 16
  std::vector<char> bind_buf_15(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_15(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 16, SQL_C_BIT, bind_buf_15.data(), 2, bind_ind_15.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 17
  std::vector<char> bind_buf_16(1 * 16, 0);
  std::vector<SQLLEN> bind_ind_16(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 17, SQL_C_DATE, bind_buf_16.data(), 16, bind_ind_16.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 18
  std::vector<char> bind_buf_17(1 * 16, 0);
  std::vector<SQLLEN> bind_ind_17(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 18, SQL_C_TIME, bind_buf_17.data(), 16, bind_ind_17.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 19
  std::vector<char> bind_buf_18(1 * 16, 0);
  std::vector<SQLLEN> bind_ind_18(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 19, SQL_C_TIMESTAMP, bind_buf_18.data(), 16, bind_ind_18.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 20
  std::vector<char> bind_buf_19(1 * 16, 0);
  std::vector<SQLLEN> bind_ind_19(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 20, SQL_C_TIMESTAMP, bind_buf_19.data(), 16, bind_ind_19.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 21
  std::vector<char> bind_buf_20(1 * 16, 0);
  std::vector<SQLLEN> bind_ind_20(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 21, SQL_C_TIMESTAMP, bind_buf_20.data(), 16, bind_ind_20.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "99999999999999999999999999999999999999");
    CHECK(ind == 38);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "boundary text payload");
    CHECK(ind == 21);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt0, 15, SQL_C_BINARY, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == 2);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt0, 15, SQL_C_BINARY, buf.data(), 2, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
  }

  // SQLGetData col 22
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 22, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 22
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 22, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "null");
    CHECK(ind == 4);
  }

  // SQLGetData col 23
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 23, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 23
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 23, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "{}");
    CHECK(ind == 2);
  }

  // SQLGetData col 24
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 24, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 24
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 24, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "[]");
    CHECK(ind == 2);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == ": \"Point\"\n}");
    CHECK(ind == 11);
  }

  // SQLFetch
  // Class A divergence: converting this row's high-scale numeric column into the
  // bound buffer truncates fractional digits. The Windows DM surfaces plain
  // success; the driver under unixODBC returns a warning (01000 / native 40460).
  {
    SQLRETURN ret = SQLFetch(stmt0);
    WINDOWS_ONLY { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess()); }
    UNIX_ONLY {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01000"));
    }
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "123456789012345678901234567890");
    CHECK(ind == 30);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "representative text payload");
    CHECK(ind == 27);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt0, 15, SQL_C_BINARY, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == 4);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(4, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt0, 15, SQL_C_BINARY, buf.data(), 4, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == 4);
  }

  // SQLGetData col 22
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 22, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 22
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 22, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "{\n  \"a\": 1\n}");
    CHECK(ind == 12);
  }

  // SQLGetData col 23
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 23, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 23
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 23, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "{\n  \"k\": \"v\"\n}");
    CHECK(ind == 14);
  }

  // SQLGetData col 24
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 24, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 24
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 24, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "[\n  1,\n  2,\n  3\n]");
    CHECK(ind == 17);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "\": \"Point\"\n}");
    CHECK(ind == 12);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt0, 15, SQL_C_BINARY, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 22
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 22, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 23
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 23, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 24
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 24, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  // Class A divergence: as above, fetching this row truncates fractional digits
  // of a numeric column. Windows DM reports success; unixODBC surfaces the
  // driver's warning (01000 / native 40460).
  {
    SQLRETURN ret = SQLFetch(stmt0);
    WINDOWS_ONLY { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess()); }
    UNIX_ONLY {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01000"));
    }
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "42");
    CHECK(ind == 2);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 12
  // Non-ASCII source text fetched as narrow SQL_C_CHAR is substituted byte-for-byte
  // with 0x1A (ASCII SUB) by the driver's Unicode->ANSI conversion, on both the
  // Windows capture and the Linux reference driver.
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "\x1a\x1a \x1a\x1a\x1a\x1a\x1a\x1a\x1a \x1a\x1a\x1a \x1a mixed scripts");
    CHECK(ind == 30);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt0, 15, SQL_C_BINARY, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == 2);
  }

  // SQLGetData col 15
  {
    SQLLEN ind = 0;
    std::vector<char> buf(2, static_cast<char>(0xFF));
    SQLRETURN ret = SQLGetData(stmt0, 15, SQL_C_BINARY, buf.data(), 2, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == 2);
  }

  // SQLGetData col 22
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 22, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 22
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 22, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "{\n  \"cjk\": \"\x1a\x1a\",\n  \"emoji\": \"\x1a\"\n}");
    CHECK(ind == 33);
  }

  // SQLGetData col 23
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 23, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 23
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 23, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "{\n  \"lang\": \"\x1a\x1a\"\n}");
    CHECK(ind == 18);
  }

  // SQLGetData col 24
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 24, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 24
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 24, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == "[\n  \"\x1a\",\n  \"\x1a\",\n  \"\x1a\"\n]");
    CHECK(ind == 23);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(1, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 0, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccessWithInfo());
    CHECK(ind == SQL_NO_TOTAL);
  }

  // SQLGetData col 25
  {
    std::vector<char> buf(52, static_cast<char>(0xFF));
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 25, SQL_C_CHAR, buf.data(), 51, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());
    CHECK(std::string(buf.data(), n) == ",\n  \"type\": \"Point\"\n}");
    CHECK(ind == 21);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
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

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }
}
