#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

// ============================================================================
// Global Function List - Comprehensive ODBC Function Coverage
// ============================================================================

struct FunctionTest {
  SQLUSMALLINT functionId;
  const char* name;
  bool odbc2;
};

static const FunctionTest ALL_ODBC_FUNCTIONS[] = {
    // Connection Functions
    {SQL_API_SQLALLOCHANDLE, "SQLAllocHandle", false},
    {SQL_API_SQLBROWSECONNECT, "SQLBrowseConnect", true},
    {SQL_API_SQLCONNECT, "SQLConnect", true},
    {SQL_API_SQLDRIVERCONNECT, "SQLDriverConnect", true},

    // Driver Information Functions
    {SQL_API_SQLGETFUNCTIONS, "SQLGetFunctions", true},
    {SQL_API_SQLGETINFO, "SQLGetInfo", true},
    {SQL_API_SQLGETTYPEINFO, "SQLGetTypeInfo", true},

    // Catalog Functions
    {SQL_API_SQLCOLUMNPRIVILEGES, "SQLColumnPrivileges", true},
    {SQL_API_SQLCOLUMNS, "SQLColumns", true},
    {SQL_API_SQLFOREIGNKEYS, "SQLForeignKeys", true},
    {SQL_API_SQLPRIMARYKEYS, "SQLPrimaryKeys", true},
    {SQL_API_SQLPROCEDURECOLUMNS, "SQLProcedureColumns", true},
    {SQL_API_SQLPROCEDURES, "SQLProcedures", true},
    {SQL_API_SQLSPECIALCOLUMNS, "SQLSpecialColumns", true},
    {SQL_API_SQLSTATISTICS, "SQLStatistics", true},
    {SQL_API_SQLTABLEPRIVILEGES, "SQLTablePrivileges", true},
    {SQL_API_SQLTABLES, "SQLTables", true},

    // Statement Preparation Functions
    {SQL_API_SQLBINDPARAMETER, "SQLBindParameter", true},
    {SQL_API_SQLGETCURSORNAME, "SQLGetCursorName", true},
    {SQL_API_SQLPREPARE, "SQLPrepare", true},
    {SQL_API_SQLSETCURSORNAME, "SQLSetCursorName", true},
    {SQL_API_SQLSETSCROLLOPTIONS, "SQLSetScrollOptions", true},

    // Result Retrieval Functions
    {SQL_API_SQLBINDCOL, "SQLBindCol", true},
    // Note: SQLBulkOperations not supported by reference driver
    {SQL_API_SQLCOLATTRIBUTE, "SQLColAttribute", false},
    {SQL_API_SQLCOLATTRIBUTES, "SQLColAttributes", true},
    {SQL_API_SQLDESCRIBECOL, "SQLDescribeCol", true},
    {SQL_API_SQLFETCH, "SQLFetch", true},
    // Note: SQLFetchScroll not supported by reference driver
    {SQL_API_SQLGETDATA, "SQLGetData", true},
    {SQL_API_SQLGETDIAGFIELD, "SQLGetDiagField", false},
    {SQL_API_SQLGETDIAGREC, "SQLGetDiagRec", false},
    {SQL_API_SQLMORERESULTS, "SQLMoreResults", true},
    {SQL_API_SQLNUMRESULTCOLS, "SQLNumResultCols", true},
    {SQL_API_SQLROWCOUNT, "SQLRowCount", true},
    // Note: SQLSetPos not supported by reference driver

    // Descriptor Functions
    {SQL_API_SQLCOPYDESC, "SQLCopyDesc", false},
    {SQL_API_SQLGETDESCFIELD, "SQLGetDescField", false},
    {SQL_API_SQLGETDESCREC, "SQLGetDescRec", false},
    {SQL_API_SQLSETDESCFIELD, "SQLSetDescField", false},
    {SQL_API_SQLSETDESCREC, "SQLSetDescRec", false},

    // Attribute Functions
    {SQL_API_SQLGETCONNECTATTR, "SQLGetConnectAttr", false},
    {SQL_API_SQLGETENVATTR, "SQLGetEnvAttr", false},
    {SQL_API_SQLGETSTMTATTR, "SQLGetStmtAttr", false},
    {SQL_API_SQLPARAMOPTIONS, "SQLParamOptions", true},
    {SQL_API_SQLSETCONNECTATTR, "SQLSetConnectAttr", false},
    {SQL_API_SQLSETENVATTR, "SQLSetEnvAttr", false},
    {SQL_API_SQLSETSTMTATTR, "SQLSetStmtAttr", false},

    // Execution Functions
    {SQL_API_SQLDESCRIBEPARAM, "SQLDescribeParam", true},
    {SQL_API_SQLEXECDIRECT, "SQLExecDirect", true},
    {SQL_API_SQLEXECUTE, "SQLExecute", true},
    {SQL_API_SQLNATIVESQL, "SQLNativeSql", true},
    {SQL_API_SQLNUMPARAMS, "SQLNumParams", true},
    {SQL_API_SQLPARAMDATA, "SQLParamData", true},
    {SQL_API_SQLPUTDATA, "SQLPutData", true},

    // Disconnection Functions
    {SQL_API_SQLDISCONNECT, "SQLDisconnect", true},
    {SQL_API_SQLFREECONNECT, "SQLFreeConnect", true},
    {SQL_API_SQLFREEENV, "SQLFreeEnv", true},
    {SQL_API_SQLFREEHANDLE, "SQLFreeHandle", false},

    // Statement Termination Functions
    {SQL_API_SQLCANCEL, "SQLCancel", true},
    {SQL_API_SQLCANCELHANDLE, "SQLCancelHandle", false},
    {SQL_API_SQLCLOSECURSOR, "SQLCloseCursor", false},
    {SQL_API_SQLENDTRAN, "SQLEndTran", false},
    {SQL_API_SQLFREESTMT, "SQLFreeStmt", true},
};

// ============================================================================
// SQLGetFunctions - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture,
                 "SQLGetFunctions: Returns all supported functions with SQL_API_ODBC3_ALL_FUNCTIONS",
                 "[odbc-api][getfunctions][driver_info]") {
  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported[SQL_API_ODBC3_ALL_FUNCTIONS_SIZE] = {};

  ret = SQLGetFunctions(dbc_handle(), SQL_API_ODBC3_ALL_FUNCTIONS, supported);
  REQUIRE(ret == SQL_SUCCESS);

  for (const auto& func : ALL_ODBC_FUNCTIONS) {
    // Windows DM does not report deprecated SQLSetScrollOptions
    WINDOWS_ONLY {
      if (func.functionId == SQL_API_SQLSETSCROLLOPTIONS) continue;
    }
    // The old driver under iODBC also omits SQL_API_SQLSETSCROLLOPTIONS and
    //   SQL_API_SQLPARAMOPTIONS from its bitmap (the entry points aren't
    //   exported via iODBC's dispatch), matching the Windows DM exclusion
    //   for SetScrollOptions above.
    OLD_IODBC_ONLY("BD#65") {
      if (func.functionId == SQL_API_SQLSETSCROLLOPTIONS) continue;
      if (func.functionId == SQL_API_SQLPARAMOPTIONS) continue;
    }
    // BD#127: old driver under iODBC omits SQLFreeConnect / SQLFreeEnv from the
    //   ODBC3 bitmap (it does not export the symbols; DM maps them to
    //   SQLFreeHandle). Per-function and SQL_API_ALL_FUNCTIONS probes still
    //   report them supported.
    OLD_IODBC_ONLY("BD#127") {
      if (func.functionId == SQL_API_SQLFREECONNECT) continue;
      if (func.functionId == SQL_API_SQLFREEENV) continue;
    }
    INFO("Function: " << func.name << " (ID=" << func.functionId << ")");
    REQUIRE(SQL_FUNC_EXISTS(supported, func.functionId));
  }

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture,
                 "SQLGetFunctions: Returns all supported functions with SQL_API_ALL_FUNCTIONS (ODBC 2.x)",
                 "[odbc-api][getfunctions][driver_info]") {
  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported[100] = {};  // Size must be at least the largest function ID in ALL_ODBC_FUNCTIONS

  ret = SQLGetFunctions(dbc_handle(), SQL_API_ALL_FUNCTIONS, supported);
  REQUIRE(ret == SQL_SUCCESS);

  for (const auto& func : ALL_ODBC_FUNCTIONS) {
    if (func.odbc2) {
      WINDOWS_ONLY {
        if (func.functionId == SQL_API_SQLSETSCROLLOPTIONS) continue;
      }
      // Same exclusions apply to iODBC + old driver for the
      //   SQL_API_ALL_FUNCTIONS (ODBC 2.x) bitmap as for the 3.x bitmap above.
      OLD_IODBC_ONLY("BD#65") {
        if (func.functionId == SQL_API_SQLSETSCROLLOPTIONS) continue;
        if (func.functionId == SQL_API_SQLPARAMOPTIONS) continue;
      }
      REQUIRE(supported[func.functionId] == SQL_TRUE);
    }
  }

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Correctly reports unsupported optional functions",
                 "[odbc-api][getfunctions][driver_info]") {
  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported = SQL_TRUE;

  // Note: Reference driver supports SQLBrowseConnect but not iteratively
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLBROWSECONNECT, &supported);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(supported == SQL_TRUE);

  // Note: Reference driver does not support SQLBulkOperations
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLBULKOPERATIONS, &supported);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(supported == SQL_FALSE);

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLGetFunctions - Error Cases: Invalid Handle
// ============================================================================

TEST_CASE("SQLGetFunctions: SQL_INVALID_HANDLE - NULL connection handle",
          "[odbc-api][getfunctions][driver_info][error]") {
  SQLUSMALLINT supported = SQL_FALSE;
  const SQLRETURN ret = SQLGetFunctions(SQL_NULL_HDBC, SQL_API_SQLCONNECT, &supported);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(EnvFixture, "SQLGetFunctions: SQL_INVALID_HANDLE - Invalid handle type",
                 "[odbc-api][getfunctions][driver_info][error]") {
  SQLUSMALLINT supported = SQL_FALSE;
  const SQLRETURN ret = SQLGetFunctions(env_handle(), SQL_API_SQLCONNECT, &supported);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// ============================================================================
// SQLGetFunctions - Error Cases: Invalid Parameters
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Accepts NULL output pointer",
                 "[odbc-api][getfunctions][driver_info]") {
  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns SUCCESS for NULL pointer (differs from ODBC spec)
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLCONNECT, nullptr);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

// Tagged [flaky] so it is excluded from the gating run (`ctest -LE flaky`) while
// still running in the separate non-blocking flaky step. It fails intermittently
// only under the iODBC driver manager, whose dispatch of an out-of-range
// SQLGetFunctions FunctionId diverges from unixODBC/Windows (see BD#62), not the
// driver's own function-table behavior that the assertions below cover.
TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: HY095 - Invalid FunctionId",
                 "[odbc-api][getfunctions][driver_info][error][flaky]") {
  // Given an active connection to the default DSN
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLGetFunctions is called with an out-of-range FunctionId (9999)
  SQLUSMALLINT supported = SQL_FALSE;
  ret = SQLGetFunctions(dbc_handle(), 9999, &supported);

  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()),
               OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY095"));

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLGetFunctions - State Transition Tests
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLGetFunctions: Requires active connection",
                 "[odbc-api][getfunctions][driver_info][error]") {
  // Given an allocated but unconnected DBC handle
  // When SQLGetFunctions is called on the unconnected handle for SQL_API_SQLCONNECT
  SQLUSMALLINT supported = SQL_FALSE;
  const SQLRETURN ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLCONNECT, &supported);

  NON_IODBC {
    // And the reference driver requires an active connection
    //   (differs from ODBC spec) and the call surfaces HY010 (function sequence error)
    REQUIRE_EXPECTED_ERROR(ret, "HY010", dbc_handle(), SQL_HANDLE_DBC);
  }
  IODBC_ONLY {
    // And the iODBC DM also requires an active connection for SQLGetFunctions
    //   and rejects the call with SQL_ERROR before consulting its static
    //   table
    REQUIRE(ret == SQL_ERROR);
  }
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Can be called after connection established",
                 "[odbc-api][getfunctions][driver_info]") {
  // iODBC short-circuits SQLGetFunctions from its static table without contacting
  // the driver, so the round-trip behaviour this test asserts cannot be exercised
  // there. The static-table behaviour is covered separately by the "Invalid
  // function ID" and "Requires active connection" cases above.
  SKIP_IODBC(
      "iODBC DM reports SQL_API_SQLEXECDIRECT as supported in its static table regardless of driver answer, but the "
      "assertion order assumes a driver round-trip");

  // Given a DBC handle that has been connected to the default DSN
  const std::string dsn = dsn_name();
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn.c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLGetFunctions is called for SQL_API_SQLEXECDIRECT
  SQLUSMALLINT supported = SQL_FALSE;
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLEXECDIRECT, &supported);

  // Then the driver round-trip reports SQL_TRUE for the function id
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(supported == SQL_TRUE);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Reports SQLEndTran as supported",
                 "[odbc-api][getfunctions][driver_info]") {
  // Regression guard: SQLEndTran is implemented and exported by the driver, so
  // SQLGetFunctions must report it supported. When SQLEndTran was left out of
  // the driver's supported-function bitmap, unixODBC (which gates dispatch on
  // that bitmap) refused to call the driver's SQLEndTran and returned
  // SQL_ERROR, while iODBC/Windows dispatched regardless - so the regression
  // only surfaced under unixODBC. The comprehensive coverage tests above are
  // gated behind SKIP_NEW_DRIVER_NOT_IMPLEMENTED and so do not cover this on
  // the new driver; this focused case does.
  //
  // iODBC answers SQLGetFunctions from its own static table without a driver
  // round-trip, so the driver-answer assertion only holds off iODBC (matching
  // the SQL_API_SQLEXECDIRECT round-trip case above).
  SKIP_IODBC("iODBC DM answers SQLGetFunctions from its static table regardless of the driver's bitmap");

  const std::string dsn = dsn_name();
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn.c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported = SQL_FALSE;
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLENDTRAN, &supported);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(supported == SQL_TRUE);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Reports SQLTransact as supported",
                 "[odbc-api][getfunctions][driver_info]") {
  SKIP_IODBC("iODBC DM answers SQLGetFunctions from its static table regardless of the driver's bitmap");

  const std::string dsn = dsn_name();
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn.c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported = SQL_FALSE;
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLTRANSACT, &supported);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(supported == SQL_TRUE);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Reports SQLFreeConnect as supported",
                 "[odbc-api][getfunctions][driver_info]") {
  const std::string dsn = dsn_name();
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn.c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported = SQL_FALSE;
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLFREECONNECT, &supported);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(supported == SQL_TRUE);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Reports SQLFreeEnv as supported",
                 "[odbc-api][getfunctions][driver_info]") {
  const std::string dsn = dsn_name();
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn.c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported = SQL_FALSE;
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLFREEENV, &supported);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(supported == SQL_TRUE);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Reports SQLCloseCursor as supported",
                 "[odbc-api][getfunctions][driver_info]") {
  // Regression guard: SQLCloseCursor is implemented and exported by both drivers, so
  // SQLGetFunctions must report it supported. SQL_API_SQLCLOSECURSOR (1003) is within
  // unixODBC's accepted single-function id range, so the DM answers it from the ODBC3
  // bitmap for both drivers, which return SQL_SUCCESS + SQL_TRUE (confirmed in CI). This
  // is unlike SQL_API_SQLCANCELHANDLE (1022), which the DM range-rejects — see BD#120.
  SKIP_IODBC("iODBC DM answers SQLGetFunctions from its static table regardless of the driver's bitmap");

  const std::string dsn = dsn_name();
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn.c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported = SQL_FALSE;
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLCLOSECURSOR, &supported);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(supported == SQL_TRUE);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: ODBC3 bitmap reports SQLCancelHandle",
                 "[odbc-api][getfunctions][driver_info]") {
  SKIP_IODBC("iODBC DM answers SQLGetFunctions from its static table regardless of the driver's bitmap");

  const std::string dsn = dsn_name();
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn.c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported[SQL_API_ODBC3_ALL_FUNCTIONS_SIZE] = {};
  ret = SQLGetFunctions(dbc_handle(), SQL_API_ODBC3_ALL_FUNCTIONS, supported);
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE(SQL_FUNC_EXISTS(supported, SQL_API_SQLCANCELHANDLE));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLGetFunctions - Comprehensive Function Coverage Test
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: All known supported functions",
                 "[odbc-api][getfunctions][driver_info]") {
  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  for (const auto& func : ALL_ODBC_FUNCTIONS) {
    WINDOWS_ONLY {
      if (func.functionId == SQL_API_SQLSETSCROLLOPTIONS) continue;
    }
    // The old driver under iODBC also omits SQL_API_SQLSETSCROLLOPTIONS from
    //   its per-function inventory (matches the bitmap exclusion in the
    //   ODBC3_ALL_FUNCTIONS / ALL_FUNCTIONS tests above).
    OLD_IODBC_ONLY("BD#65") {
      if (func.functionId == SQL_API_SQLSETSCROLLOPTIONS) continue;
    }
    // BD#120: unixODBC range-rejects the per-function SQLGetFunctions(id) probe of
    //   SQL_API_SQLCANCELHANDLE (id 1022, past the DM's SQL_API_SQLFETCHSCROLL/1021
    //   guard) with SQL_ERROR, even though the ODBC3 bitmap reports it supported.
    //   iODBC and Windows still answer the per-function probe.
    //   SQL_API_SQLCLOSECURSOR (1003) is in range and returns SQL_SUCCESS on both
    //   drivers, so it is asserted normally here.
    UNIX_ONLY {
      NON_IODBC {
        if (func.functionId == SQL_API_SQLCANCELHANDLE) continue;
      }
    }
    INFO("Testing function: " << func.name << " (ID=" << func.functionId << ")");
    SQLUSMALLINT supported = SQL_FALSE;
    ret = SQLGetFunctions(dbc_handle(), func.functionId, &supported);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(supported == SQL_TRUE);
  }

  SQLDisconnect(dbc_handle());
}
