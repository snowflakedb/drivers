#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "compatibility.hpp"
#include "ODBCFixtures.hpp"
#include "get_diag_rec.hpp"
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
    {SQL_API_SQLFREEHANDLE, "SQLFreeHandle", false},

    // Statement Termination Functions
    {SQL_API_SQLCANCEL, "SQLCancel", true},
    {SQL_API_SQLCLOSECURSOR, "SQLCloseCursor", false},
    {SQL_API_SQLENDTRAN, "SQLEndTran", false},
    {SQL_API_SQLFREESTMT, "SQLFreeStmt", true},
};

// ============================================================================
// SQLGetFunctions - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Returns all supported functions with SQL_API_ODBC3_ALL_FUNCTIONS",
                 "[odbc-api][getfunctions][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported[SQL_API_ODBC3_ALL_FUNCTIONS_SIZE] = {};

  ret = SQLGetFunctions(dbc_handle(), SQL_API_ODBC3_ALL_FUNCTIONS, supported);
  REQUIRE(ret == SQL_SUCCESS);

  for (const auto& func : ALL_ODBC_FUNCTIONS) {
    REQUIRE(SQL_FUNC_EXISTS(supported, func.functionId));
  }

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Returns all supported functions with SQL_API_ALL_FUNCTIONS (ODBC 2.x)",
                 "[odbc-api][getfunctions][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported[100] = {}; // Size must be at least the largest function ID in ALL_ODBC_FUNCTIONS

  ret = SQLGetFunctions(dbc_handle(), SQL_API_ALL_FUNCTIONS, supported);
  REQUIRE(ret == SQL_SUCCESS);

  for (const auto& func : ALL_ODBC_FUNCTIONS) {
    if (func.odbc2) {
      REQUIRE(supported[func.functionId] == SQL_TRUE);
    }
  }

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Correctly reports unsupported optional functions",
                 "[odbc-api][getfunctions][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
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
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns SUCCESS for NULL pointer (differs from ODBC spec)
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLCONNECT, nullptr);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: HY095 - Invalid FunctionId",
                 "[odbc-api][getfunctions][driver_info][error]") {
  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported = SQL_FALSE;
  ret = SQLGetFunctions(dbc_handle(), 9999, &supported);

  // Note: Reference driver validates function ID and returns error
  REQUIRE(ret == SQL_ERROR);
  const auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  REQUIRE(std::string(records[0].sqlState) == "HY095");

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLGetFunctions - State Transition Tests
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLGetFunctions: Requires active connection",
                 "[odbc-api][getfunctions][driver_info][error]") {
  SQLUSMALLINT supported = SQL_FALSE;
  const SQLRETURN ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLCONNECT, &supported);

  // Note: Reference driver requires active connection (differs from ODBC spec)
  REQUIRE_EXPECTED_ERROR(ret, "HY010", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: Can be called after connection established",
                 "[odbc-api][getfunctions][driver_info]") {
  const std::string dsn = config.value().dsn_name();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS,
                             nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT supported = SQL_FALSE;
  ret = SQLGetFunctions(dbc_handle(), SQL_API_SQLEXECDIRECT, &supported);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(supported == SQL_TRUE);

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLGetFunctions - Comprehensive Function Coverage Test
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetFunctions: All known supported functions",
                 "[odbc-api][getfunctions][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: Reference driver requires an active connection
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  for (const auto& func : ALL_ODBC_FUNCTIONS) {
    SQLUSMALLINT supported = SQL_FALSE;
    ret = SQLGetFunctions(dbc_handle(), func.functionId, &supported);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(supported == SQL_TRUE);
  }

  SQLDisconnect(dbc_handle());
}
