#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: excel vba_ado concurrent_recordsets", "[excel][vba_ado][concurrent_recordsets]") {
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
    SQLRETURN ret = SQLExecDirect(stmt0,
                                  sqlchar("SELECT ROWKIND, INTVAL, VARCHARVAL\nFROM "
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
    CHECK(numCols == 3);
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 3);
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

  SQLHSTMT stmt1 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt1 != SQL_NULL_HSTMT);
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
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_QUERY_TIMEOUT, (SQLPOINTER)30, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_PARAMSET_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)1, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt1,
                                  sqlchar("SELECT ROWKIND, DATEVAL, DATEADD('day', -1, TSLTZ) AS TSLTZ FROM "
                                          "ODBCMETADATATESTDB.DATATYPETESTS.ALLDATATYPES ORDER BY ROWKIND DESC"),
                                  SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLRowCount
  {
    SQLLEN rowCount = 0;
    SQLRETURN ret = SQLRowCount(stmt1, &rowCount);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(rowCount == 4);
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt1, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numCols == 3);
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
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt1, 1, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 2
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt1, 2, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(dataType == 9);
    CHECK(colSize == 10);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 2);
  }

  // SQLDescribeCol col 3
  {
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt1, 3, nullptr, 0, nullptr, &dataType, &colSize, &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(dataType == 11);
    CHECK(colSize == 29);
    CHECK(scale == 9);
    CHECK(nullable == 1);
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_UPDATABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_DESC_UPDATABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)496, -6);
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
  std::vector<char> row_buf_stmt0(2 * 496, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 1, SQL_C_CHAR, row_buf_stmt0.data() + 152, 17,
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

  // SQLSetStmtAttr - SQL_ROWSET_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ROWSET_SIZE, (SQLPOINTER)1, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_3 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_3(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_3, extfetch_status_3.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_3 == 1);
    CHECK(extfetch_status_3[0] == SQL_ROW_SUCCESS);
  }

  // SQLSetStmtAttr - SQL_ROWSET_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ROWSET_SIZE, (SQLPOINTER)2, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)240, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_4 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_4, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_ROW_BIND_OFFSET_PTR, nullptr, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_RETRIEVE_DATA
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_RETRIEVE_DATA, nullptr, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_RETRIEVE_DATA
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_RETRIEVE_DATA, (SQLPOINTER)1, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_5 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_5, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> row_buf_stmt1(2 * 240, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 1, SQL_C_CHAR, row_buf_stmt1.data() + 152, 17,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt1.data() + 144));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  {
    SQLRETURN ret = SQLBindCol(stmt1, 2, SQL_C_DATE, row_buf_stmt1.data() + 192, 16,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt1.data() + 184));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  {
    SQLRETURN ret = SQLBindCol(stmt1, 3, SQL_C_TIMESTAMP, row_buf_stmt1.data() + 224, 16,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt1.data() + 216));
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ROWSET_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt1, SQL_ROWSET_SIZE, (SQLPOINTER)1, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_6 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_6(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt1, SQL_FETCH_NEXT, 0, &extfetch_rows_6, extfetch_status_6.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_6 == 1);
    CHECK(extfetch_status_6[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_7 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_7(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_7, extfetch_status_7.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_7 == 1);
    CHECK(extfetch_status_7[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_8 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_8(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt1, SQL_FETCH_NEXT, 0, &extfetch_rows_8, extfetch_status_8.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_8 == 1);
    CHECK(extfetch_status_8[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_9 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_9(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_9, extfetch_status_9.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_9 == 1);
    CHECK(extfetch_status_9[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_10 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_10(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt1, SQL_FETCH_NEXT, 0, &extfetch_rows_10, extfetch_status_10.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_10 == 1);
    CHECK(extfetch_status_10[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_11 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_11(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_11, extfetch_status_11.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_11 == 1);
    CHECK(extfetch_status_11[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_12 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_12(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt1, SQL_FETCH_NEXT, 0, &extfetch_rows_12, extfetch_status_12.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(extfetch_rows_12 == 1);
    CHECK(extfetch_status_12[0] == SQL_ROW_SUCCESS);
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_13 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_13(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt0, SQL_FETCH_NEXT, 0, &extfetch_rows_13, extfetch_status_13.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLExtendedFetch
  SQLULEN extfetch_rows_14 = 0;
  std::vector<SQLUSMALLINT> extfetch_status_14(2, 0);
  {
    SQLRETURN ret = SQLExtendedFetch(stmt1, SQL_FETCH_NEXT, 0, &extfetch_rows_14, extfetch_status_14.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsNoData());
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
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
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
