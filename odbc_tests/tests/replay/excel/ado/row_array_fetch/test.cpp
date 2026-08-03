#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: excel vba_ado row_array_fetch", "[excel][vba_ado][row_array_fetch]") {
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

  // SQLSetConnectAttr - SQL_ATTR_MAX_ROWS (BD#107)
  // Windows DM and iODBC both forward this call to the driver (do not intercept).
  // unixODBC also forwards and remaps HY092→S1092 (ODBC 2.x mode).
  // Old driver swallowed silently (SQL_SUCCESS); new driver rejects with HY092.
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_MAX_ROWS, nullptr, -6);
    WINDOWS_ONLY {
      OLD_DRIVER_ONLY("BD#107") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess()); }
      NEW_DRIVER_ONLY("BD#107") {
        CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0),
                   OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY092"));
      }
    }
    UNIX_ONLY {
      IODBC_ONLY {
        OLD_DRIVER_ONLY("BD#107") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess()); }
        NEW_DRIVER_ONLY("BD#107") {
          CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0),
                     OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY092"));
        }
      }
      NON_IODBC {
        CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0),
                   OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("S1092"));
      }
    }
  }

  // SQLSetConnectAttr - SQL_ATTR_QUERY_TIMEOUT (BD#107; same as SQL_ATTR_MAX_ROWS above)
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_QUERY_TIMEOUT, nullptr, -6);
    WINDOWS_ONLY {
      OLD_DRIVER_ONLY("BD#107") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess()); }
      NEW_DRIVER_ONLY("BD#107") {
        CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0),
                   OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY092"));
      }
    }
    UNIX_ONLY {
      IODBC_ONLY {
        OLD_DRIVER_ONLY("BD#107") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess()); }
        NEW_DRIVER_ONLY("BD#107") {
          CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0),
                     OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY092"));
        }
      }
      NON_IODBC {
        CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0),
                   OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("S1092"));
      }
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
  // SQL_DESC_BIND_TYPE = 10 is accepted by the Windows DM but
  // rejected by the reference driver under 64-bit unixODBC
  // (value must be 0 or divisible by alignof(SQLLEN) = 8), surfaced as 11700.
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_PARAM_BIND_TYPE, (SQLPOINTER)10, 0);
    WINDOWS_ONLY { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess()); }
    UNIX_ONLY {
      // BD#100: misaligned bind type (10) behaviour differs by driver and platform.
      // Old driver: accepted on Linux x86_64, rejected (S1000) on aarch64/macOS.
      // New driver: accepted on all platforms.
      if (get_platform() == PLATFORM::PLATFORM_LINUX && get_arch() == ARCH::ARCH_X86_64) {
        CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
      } else {
        OLD_DRIVER_ONLY("BD#100") {
          CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0),
                     OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("S1000"));
        }
        NEW_DRIVER_ONLY("BD#100") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess()); }
      }
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
    SQLRETURN ret =
        SQLExecDirect(stmt0,
                      sqlchar("SELECT a.INTVAL,\n       a.BIGINTVAL,\n       a.VARCHARVAL,\n       a.DATEVAL,\n       "
                              "a.TSNTZ,\n       SEQ4() AS DUP_ID\nFROM ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPES "
                              "a\nCROSS JOIN TABLE(GENERATOR(ROWCOUNT => 12500))\nORDER BY a.ROWKIND, DUP_ID;"),
                      SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLRowCount
  {
    SQLLEN rowCount = 0;
    SQLRETURN ret = SQLRowCount(stmt0, &rowCount);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(rowCount == 50000);
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 6);
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 6);
  }

  // SQLDescribeCol col 1
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 1, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(dataType == 3);
    CHECK(colSize == 38);
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
    CHECK(dataType == 12);
    CHECK(colSize == 256);
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
    CHECK(dataType == 9);
    CHECK(colSize == 10);
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
    CHECK(dataType == 11);
    CHECK(colSize == 29);
    CHECK(scale == 9);
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
    CHECK(dataType == 3);
    CHECK(colSize == 10);
    CHECK(scale == 0);
    CHECK(nullable == 0);
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

  // SQLSetStmtAttr - SQL_ROWSET_SIZE: the new driver ships SQLExtendedFetch but rejects
  // SQL_ROWSET_SIZE (the ODBC 2.x attribute that configures it) with S1092, and its
  // SQLExtendedFetch reads SQL_ATTR_ROW_ARRAY_SIZE instead. The reference driver accepts it.
  // Remove this skip once SNOW-3779798 implements SQL_ROWSET_SIZE for SQLExtendedFetch.
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ROWSET_SIZE, (SQLPOINTER)2, -6);
    SKIP_NEW_DRIVER("SNOW-3779798",
                    "SQL_ROWSET_SIZE (ODBC 2.x, attr 9) rejected with S1092; new driver's "
                    "SQLExtendedFetch uses SQL_ATTR_ROW_ARRAY_SIZE instead of SQL_ROWSET_SIZE");
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)600, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
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

  SQLULEN attr_ptr_2 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_2, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> row_buf_stmt0(256 * 600, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 1, SQL_C_NUMERIC, row_buf_stmt0.data() + 152, 19,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt0.data() + 144));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  {
    SQLRETURN ret = SQLBindCol(stmt0, 2, SQL_C_NUMERIC, row_buf_stmt0.data() + 192, 19,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt0.data() + 184));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  {
    SQLRETURN ret = SQLBindCol(stmt0, 3, SQL_C_CHAR, row_buf_stmt0.data() + 232, 257,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt0.data() + 224));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  {
    SQLRETURN ret = SQLBindCol(stmt0, 4, SQL_C_DATE, row_buf_stmt0.data() + 512, 16,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt0.data() + 504));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  {
    SQLRETURN ret = SQLBindCol(stmt0, 5, SQL_C_TIMESTAMP, row_buf_stmt0.data() + 544, 16,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt0.data() + 536));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 6
  {
    SQLRETURN ret = SQLBindCol(stmt0, 6, SQL_C_NUMERIC, row_buf_stmt0.data() + 576, 19,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt0.data() + 568));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ROWSET_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ROWSET_SIZE, (SQLPOINTER)256, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_3 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_3(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_3, extfetch_status_3.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_3 == 256);
    CHECK(extfetch_status_3[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_4 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_4(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_4, extfetch_status_4.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_4 == 256);
    CHECK(extfetch_status_4[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_5 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_5(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_5, extfetch_status_5.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_5 == 256);
    CHECK(extfetch_status_5[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_6 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_6(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_6, extfetch_status_6.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_6 == 256);
    CHECK(extfetch_status_6[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_7 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_7(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_7, extfetch_status_7.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_7 == 256);
    CHECK(extfetch_status_7[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_8 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_8(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_8, extfetch_status_8.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_8 == 256);
    CHECK(extfetch_status_8[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_9 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_9(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_9, extfetch_status_9.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_9 == 256);
    CHECK(extfetch_status_9[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_10 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_10(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_10, extfetch_status_10.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_10 == 256);
    CHECK(extfetch_status_10[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_11 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_11(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_11, extfetch_status_11.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_11 == 256);
    CHECK(extfetch_status_11[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_12 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_12(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_12, extfetch_status_12.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_12 == 256);
    CHECK(extfetch_status_12[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_13 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_13(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_13, extfetch_status_13.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_13 == 256);
    CHECK(extfetch_status_13[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_14 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_14(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_14, extfetch_status_14.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_14 == 256);
    CHECK(extfetch_status_14[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_15 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_15(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_15, extfetch_status_15.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_15 == 256);
    CHECK(extfetch_status_15[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_16 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_16(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_16, extfetch_status_16.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_16 == 256);
    CHECK(extfetch_status_16[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_17 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_17(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_17, extfetch_status_17.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_17 == 256);
    CHECK(extfetch_status_17[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_18 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_18(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_18, extfetch_status_18.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_18 == 256);
    CHECK(extfetch_status_18[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_19 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_19(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_19, extfetch_status_19.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_19 == 256);
    CHECK(extfetch_status_19[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_20 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_20(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_20, extfetch_status_20.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_20 == 256);
    CHECK(extfetch_status_20[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_21 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_21(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_21, extfetch_status_21.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_21 == 256);
    CHECK(extfetch_status_21[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_22 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_22(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_22, extfetch_status_22.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_22 == 256);
    CHECK(extfetch_status_22[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_23 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_23(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_23, extfetch_status_23.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_23 == 256);
    CHECK(extfetch_status_23[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_24 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_24(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_24, extfetch_status_24.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_24 == 256);
    CHECK(extfetch_status_24[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_25 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_25(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_25, extfetch_status_25.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_25 == 256);
    CHECK(extfetch_status_25[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_26 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_26(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_26, extfetch_status_26.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_26 == 256);
    CHECK(extfetch_status_26[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_27 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_27(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_27, extfetch_status_27.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_27 == 256);
    CHECK(extfetch_status_27[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_28 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_28(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_28, extfetch_status_28.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_28 == 256);
    CHECK(extfetch_status_28[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_29 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_29(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_29, extfetch_status_29.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_29 == 256);
    CHECK(extfetch_status_29[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_30 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_30(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_30, extfetch_status_30.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_30 == 256);
    CHECK(extfetch_status_30[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_31 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_31(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_31, extfetch_status_31.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_31 == 256);
    CHECK(extfetch_status_31[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_32 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_32(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_32, extfetch_status_32.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_32 == 256);
    CHECK(extfetch_status_32[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_33 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_33(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_33, extfetch_status_33.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_33 == 256);
    CHECK(extfetch_status_33[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_34 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_34(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_34, extfetch_status_34.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_34 == 256);
    CHECK(extfetch_status_34[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_35 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_35(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_35, extfetch_status_35.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_35 == 256);
    CHECK(extfetch_status_35[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_36 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_36(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_36, extfetch_status_36.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_36 == 256);
    CHECK(extfetch_status_36[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_37 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_37(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_37, extfetch_status_37.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_37 == 256);
    CHECK(extfetch_status_37[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_38 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_38(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_38, extfetch_status_38.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_38 == 256);
    CHECK(extfetch_status_38[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_39 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_39(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_39, extfetch_status_39.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_39 == 256);
    CHECK(extfetch_status_39[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_40 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_40(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_40, extfetch_status_40.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_40 == 256);
    CHECK(extfetch_status_40[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_41 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_41(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_41, extfetch_status_41.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_41 == 256);
    CHECK(extfetch_status_41[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_42 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_42(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_42, extfetch_status_42.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_42 == 256);
    CHECK(extfetch_status_42[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_43 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_43(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_43, extfetch_status_43.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_43 == 256);
    CHECK(extfetch_status_43[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_44 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_44(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_44, extfetch_status_44.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_44 == 256);
    CHECK(extfetch_status_44[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_45 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_45(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_45, extfetch_status_45.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_45 == 256);
    CHECK(extfetch_status_45[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_46 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_46(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_46, extfetch_status_46.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_46 == 256);
    CHECK(extfetch_status_46[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_47 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_47(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_47, extfetch_status_47.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_47 == 256);
    CHECK(extfetch_status_47[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_48 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_48(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_48, extfetch_status_48.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_48 == 256);
    CHECK(extfetch_status_48[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_49 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_49(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_49, extfetch_status_49.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_49 == 256);
    CHECK(extfetch_status_49[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_50 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_50(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_50, extfetch_status_50.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_50 == 256);
    CHECK(extfetch_status_50[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_51 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_51(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_51, extfetch_status_51.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_51 == 256);
    CHECK(extfetch_status_51[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_52 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_52(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_52, extfetch_status_52.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_52 == 256);
    CHECK(extfetch_status_52[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_53 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_53(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_53, extfetch_status_53.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_53 == 256);
    CHECK(extfetch_status_53[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_54 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_54(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_54, extfetch_status_54.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_54 == 256);
    CHECK(extfetch_status_54[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_55 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_55(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_55, extfetch_status_55.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_55 == 256);
    CHECK(extfetch_status_55[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_56 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_56(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_56, extfetch_status_56.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_56 == 256);
    CHECK(extfetch_status_56[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_57 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_57(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_57, extfetch_status_57.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_57 == 256);
    CHECK(extfetch_status_57[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_58 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_58(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_58, extfetch_status_58.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_58 == 256);
    CHECK(extfetch_status_58[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_59 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_59(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_59, extfetch_status_59.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_59 == 256);
    CHECK(extfetch_status_59[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_60 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_60(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_60, extfetch_status_60.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_60 == 256);
    CHECK(extfetch_status_60[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_61 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_61(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_61, extfetch_status_61.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_61 == 256);
    CHECK(extfetch_status_61[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_62 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_62(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_62, extfetch_status_62.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_62 == 256);
    CHECK(extfetch_status_62[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_63 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_63(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_63, extfetch_status_63.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_63 == 256);
    CHECK(extfetch_status_63[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_64 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_64(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_64, extfetch_status_64.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_64 == 256);
    CHECK(extfetch_status_64[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_65 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_65(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_65, extfetch_status_65.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_65 == 256);
    CHECK(extfetch_status_65[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_66 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_66(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_66, extfetch_status_66.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_66 == 256);
    CHECK(extfetch_status_66[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_67 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_67(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_67, extfetch_status_67.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_67 == 256);
    CHECK(extfetch_status_67[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_68 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_68(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_68, extfetch_status_68.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_68 == 256);
    CHECK(extfetch_status_68[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_69 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_69(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_69, extfetch_status_69.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_69 == 256);
    CHECK(extfetch_status_69[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_70 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_70(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_70, extfetch_status_70.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_70 == 256);
    CHECK(extfetch_status_70[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_71 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_71(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_71, extfetch_status_71.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_71 == 256);
    CHECK(extfetch_status_71[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_72 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_72(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_72, extfetch_status_72.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_72 == 256);
    CHECK(extfetch_status_72[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_73 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_73(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_73, extfetch_status_73.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_73 == 256);
    CHECK(extfetch_status_73[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_74 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_74(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_74, extfetch_status_74.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_74 == 256);
    CHECK(extfetch_status_74[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_75 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_75(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_75, extfetch_status_75.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_75 == 256);
    CHECK(extfetch_status_75[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_76 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_76(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_76, extfetch_status_76.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_76 == 256);
    CHECK(extfetch_status_76[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_77 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_77(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_77, extfetch_status_77.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_77 == 256);
    CHECK(extfetch_status_77[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_78 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_78(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_78, extfetch_status_78.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_78 == 256);
    CHECK(extfetch_status_78[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_79 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_79(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_79, extfetch_status_79.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_79 == 256);
    CHECK(extfetch_status_79[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_80 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_80(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_80, extfetch_status_80.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_80 == 256);
    CHECK(extfetch_status_80[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_81 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_81(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_81, extfetch_status_81.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_81 == 256);
    CHECK(extfetch_status_81[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_82 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_82(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_82, extfetch_status_82.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_82 == 256);
    CHECK(extfetch_status_82[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_83 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_83(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_83, extfetch_status_83.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_83 == 256);
    CHECK(extfetch_status_83[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_84 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_84(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_84, extfetch_status_84.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_84 == 256);
    CHECK(extfetch_status_84[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_85 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_85(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_85, extfetch_status_85.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_85 == 256);
    CHECK(extfetch_status_85[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_86 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_86(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_86, extfetch_status_86.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_86 == 256);
    CHECK(extfetch_status_86[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_87 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_87(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_87, extfetch_status_87.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_87 == 256);
    CHECK(extfetch_status_87[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_88 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_88(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_88, extfetch_status_88.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_88 == 256);
    CHECK(extfetch_status_88[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_89 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_89(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_89, extfetch_status_89.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_89 == 256);
    CHECK(extfetch_status_89[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_90 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_90(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_90, extfetch_status_90.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_90 == 256);
    CHECK(extfetch_status_90[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_91 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_91(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_91, extfetch_status_91.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_91 == 256);
    CHECK(extfetch_status_91[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_92 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_92(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_92, extfetch_status_92.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_92 == 256);
    CHECK(extfetch_status_92[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_93 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_93(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_93, extfetch_status_93.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_93 == 256);
    CHECK(extfetch_status_93[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_94 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_94(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_94, extfetch_status_94.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_94 == 256);
    CHECK(extfetch_status_94[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_95 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_95(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_95, extfetch_status_95.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_95 == 256);
    CHECK(extfetch_status_95[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_96 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_96(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_96, extfetch_status_96.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_96 == 256);
    CHECK(extfetch_status_96[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_97 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_97(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_97, extfetch_status_97.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_97 == 256);
    CHECK(extfetch_status_97[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_98 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_98(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_98, extfetch_status_98.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_98 == 256);
    CHECK(extfetch_status_98[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_99 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_99(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_99, extfetch_status_99.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_99 == 256);
    CHECK(extfetch_status_99[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_100 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_100(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_100, extfetch_status_100.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_100 == 256);
    CHECK(extfetch_status_100[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_101 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_101(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_101, extfetch_status_101.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_101 == 256);
    CHECK(extfetch_status_101[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_102 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_102(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_102, extfetch_status_102.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_102 == 256);
    CHECK(extfetch_status_102[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_103 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_103(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_103, extfetch_status_103.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_103 == 256);
    CHECK(extfetch_status_103[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_104 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_104(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_104, extfetch_status_104.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_104 == 256);
    CHECK(extfetch_status_104[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_105 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_105(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_105, extfetch_status_105.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_105 == 256);
    CHECK(extfetch_status_105[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_106 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_106(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_106, extfetch_status_106.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_106 == 256);
    CHECK(extfetch_status_106[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_107 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_107(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_107, extfetch_status_107.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_107 == 256);
    CHECK(extfetch_status_107[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_108 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_108(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_108, extfetch_status_108.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_108 == 256);
    CHECK(extfetch_status_108[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_109 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_109(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_109, extfetch_status_109.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_109 == 256);
    CHECK(extfetch_status_109[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_110 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_110(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_110, extfetch_status_110.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_110 == 256);
    CHECK(extfetch_status_110[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_111 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_111(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_111, extfetch_status_111.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_111 == 256);
    CHECK(extfetch_status_111[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_112 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_112(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_112, extfetch_status_112.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_112 == 256);
    CHECK(extfetch_status_112[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_113 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_113(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_113, extfetch_status_113.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_113 == 256);
    CHECK(extfetch_status_113[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_114 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_114(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_114, extfetch_status_114.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_114 == 256);
    CHECK(extfetch_status_114[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_115 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_115(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_115, extfetch_status_115.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_115 == 256);
    CHECK(extfetch_status_115[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_116 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_116(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_116, extfetch_status_116.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_116 == 256);
    CHECK(extfetch_status_116[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_117 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_117(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_117, extfetch_status_117.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_117 == 256);
    CHECK(extfetch_status_117[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_118 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_118(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_118, extfetch_status_118.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_118 == 256);
    CHECK(extfetch_status_118[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_119 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_119(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_119, extfetch_status_119.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_119 == 256);
    CHECK(extfetch_status_119[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_120 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_120(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_120, extfetch_status_120.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_120 == 256);
    CHECK(extfetch_status_120[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_121 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_121(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_121, extfetch_status_121.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_121 == 256);
    CHECK(extfetch_status_121[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_122 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_122(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_122, extfetch_status_122.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_122 == 256);
    CHECK(extfetch_status_122[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_123 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_123(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_123, extfetch_status_123.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_123 == 256);
    CHECK(extfetch_status_123[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_124 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_124(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_124, extfetch_status_124.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_124 == 256);
    CHECK(extfetch_status_124[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_125 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_125(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_125, extfetch_status_125.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_125 == 256);
    CHECK(extfetch_status_125[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_126 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_126(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_126, extfetch_status_126.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_126 == 256);
    CHECK(extfetch_status_126[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_127 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_127(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_127, extfetch_status_127.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_127 == 256);
    CHECK(extfetch_status_127[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_128 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_128(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_128, extfetch_status_128.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_128 == 256);
    CHECK(extfetch_status_128[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_129 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_129(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_129, extfetch_status_129.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_129 == 256);
    CHECK(extfetch_status_129[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_130 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_130(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_130, extfetch_status_130.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_130 == 256);
    CHECK(extfetch_status_130[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_131 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_131(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_131, extfetch_status_131.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_131 == 256);
    CHECK(extfetch_status_131[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_132 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_132(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_132, extfetch_status_132.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_132 == 256);
    CHECK(extfetch_status_132[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_133 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_133(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_133, extfetch_status_133.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_133 == 256);
    CHECK(extfetch_status_133[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_134 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_134(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_134, extfetch_status_134.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_134 == 256);
    CHECK(extfetch_status_134[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_135 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_135(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_135, extfetch_status_135.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_135 == 256);
    CHECK(extfetch_status_135[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_136 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_136(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_136, extfetch_status_136.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_136 == 256);
    CHECK(extfetch_status_136[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_137 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_137(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_137, extfetch_status_137.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_137 == 256);
    CHECK(extfetch_status_137[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_138 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_138(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_138, extfetch_status_138.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_138 == 256);
    CHECK(extfetch_status_138[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_139 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_139(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_139, extfetch_status_139.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_139 == 256);
    CHECK(extfetch_status_139[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_140 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_140(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_140, extfetch_status_140.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_140 == 256);
    CHECK(extfetch_status_140[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_141 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_141(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_141, extfetch_status_141.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_141 == 256);
    CHECK(extfetch_status_141[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_142 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_142(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_142, extfetch_status_142.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_142 == 256);
    CHECK(extfetch_status_142[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_143 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_143(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_143, extfetch_status_143.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_143 == 256);
    CHECK(extfetch_status_143[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_144 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_144(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_144, extfetch_status_144.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_144 == 256);
    CHECK(extfetch_status_144[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_145 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_145(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_145, extfetch_status_145.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_145 == 256);
    CHECK(extfetch_status_145[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_146 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_146(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_146, extfetch_status_146.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_146 == 256);
    CHECK(extfetch_status_146[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_147 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_147(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_147, extfetch_status_147.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_147 == 256);
    CHECK(extfetch_status_147[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_148 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_148(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_148, extfetch_status_148.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_148 == 256);
    CHECK(extfetch_status_148[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_149 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_149(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_149, extfetch_status_149.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_149 == 256);
    CHECK(extfetch_status_149[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_150 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_150(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_150, extfetch_status_150.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_150 == 256);
    CHECK(extfetch_status_150[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_151 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_151(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_151, extfetch_status_151.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_151 == 256);
    CHECK(extfetch_status_151[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_152 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_152(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_152, extfetch_status_152.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_152 == 256);
    CHECK(extfetch_status_152[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_153 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_153(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_153, extfetch_status_153.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_153 == 256);
    CHECK(extfetch_status_153[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_154 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_154(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_154, extfetch_status_154.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_154 == 256);
    CHECK(extfetch_status_154[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_155 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_155(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_155, extfetch_status_155.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_155 == 256);
    CHECK(extfetch_status_155[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_156 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_156(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_156, extfetch_status_156.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_156 == 256);
    CHECK(extfetch_status_156[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_157 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_157(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_157, extfetch_status_157.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_157 == 256);
    CHECK(extfetch_status_157[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_158 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_158(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_158, extfetch_status_158.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_158 == 256);
    CHECK(extfetch_status_158[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_159 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_159(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_159, extfetch_status_159.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_159 == 256);
    CHECK(extfetch_status_159[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_160 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_160(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_160, extfetch_status_160.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_160 == 256);
    CHECK(extfetch_status_160[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_161 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_161(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_161, extfetch_status_161.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_161 == 256);
    CHECK(extfetch_status_161[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_162 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_162(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_162, extfetch_status_162.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_162 == 256);
    CHECK(extfetch_status_162[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_163 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_163(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_163, extfetch_status_163.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_163 == 256);
    CHECK(extfetch_status_163[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_164 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_164(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_164, extfetch_status_164.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_164 == 256);
    CHECK(extfetch_status_164[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_165 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_165(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_165, extfetch_status_165.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_165 == 256);
    CHECK(extfetch_status_165[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_166 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_166(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_166, extfetch_status_166.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_166 == 256);
    CHECK(extfetch_status_166[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_167 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_167(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_167, extfetch_status_167.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_167 == 256);
    CHECK(extfetch_status_167[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_168 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_168(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_168, extfetch_status_168.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_168 == 256);
    CHECK(extfetch_status_168[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_169 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_169(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_169, extfetch_status_169.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_169 == 256);
    CHECK(extfetch_status_169[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_170 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_170(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_170, extfetch_status_170.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_170 == 256);
    CHECK(extfetch_status_170[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_171 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_171(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_171, extfetch_status_171.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_171 == 256);
    CHECK(extfetch_status_171[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_172 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_172(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_172, extfetch_status_172.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_172 == 256);
    CHECK(extfetch_status_172[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_173 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_173(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_173, extfetch_status_173.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_173 == 256);
    CHECK(extfetch_status_173[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_174 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_174(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_174, extfetch_status_174.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_174 == 256);
    CHECK(extfetch_status_174[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_175 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_175(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_175, extfetch_status_175.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_175 == 256);
    CHECK(extfetch_status_175[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_176 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_176(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_176, extfetch_status_176.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_176 == 256);
    CHECK(extfetch_status_176[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_177 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_177(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_177, extfetch_status_177.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_177 == 256);
    CHECK(extfetch_status_177[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_178 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_178(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_178, extfetch_status_178.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_178 == 256);
    CHECK(extfetch_status_178[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_179 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_179(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_179, extfetch_status_179.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_179 == 256);
    CHECK(extfetch_status_179[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_180 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_180(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_180, extfetch_status_180.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_180 == 256);
    CHECK(extfetch_status_180[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_181 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_181(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_181, extfetch_status_181.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_181 == 256);
    CHECK(extfetch_status_181[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_182 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_182(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_182, extfetch_status_182.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_182 == 256);
    CHECK(extfetch_status_182[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_183 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_183(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_183, extfetch_status_183.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_183 == 256);
    CHECK(extfetch_status_183[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_184 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_184(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_184, extfetch_status_184.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_184 == 256);
    CHECK(extfetch_status_184[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_185 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_185(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_185, extfetch_status_185.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_185 == 256);
    CHECK(extfetch_status_185[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_186 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_186(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_186, extfetch_status_186.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_186 == 256);
    CHECK(extfetch_status_186[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_187 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_187(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_187, extfetch_status_187.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_187 == 256);
    CHECK(extfetch_status_187[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_188 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_188(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_188, extfetch_status_188.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_188 == 256);
    CHECK(extfetch_status_188[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_189 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_189(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_189, extfetch_status_189.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_189 == 256);
    CHECK(extfetch_status_189[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_190 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_190(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_190, extfetch_status_190.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_190 == 256);
    CHECK(extfetch_status_190[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_191 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_191(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_191, extfetch_status_191.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_191 == 256);
    CHECK(extfetch_status_191[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_192 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_192(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_192, extfetch_status_192.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_192 == 256);
    CHECK(extfetch_status_192[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_193 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_193(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_193, extfetch_status_193.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_193 == 256);
    CHECK(extfetch_status_193[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_194 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_194(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_194, extfetch_status_194.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_194 == 256);
    CHECK(extfetch_status_194[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_195 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_195(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_195, extfetch_status_195.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_195 == 256);
    CHECK(extfetch_status_195[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_196 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_196(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_196, extfetch_status_196.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_196 == 256);
    CHECK(extfetch_status_196[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_197 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_197(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_197, extfetch_status_197.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_197 == 256);
    CHECK(extfetch_status_197[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_198 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_198(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_198, extfetch_status_198.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_198 == 80);
    CHECK(extfetch_status_198[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_199 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_199(256, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_199, extfetch_status_199.data());
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
