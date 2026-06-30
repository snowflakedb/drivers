#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_descriptor.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// Descriptor Swap via SQLSetStmtAttr — SQL_ATTR_APP_ROW_DESC
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Set explicit ARD on statement",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Verify the active ARD is the explicit one via its state
  SQLHDESC current_ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(current_ard, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_USER);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(current_ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 3);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Explicit ARD reflects bindings independently",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC active_ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(active_ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

// ============================================================================
// Descriptor Swap via SQLSetStmtAttr — SQL_ATTR_APP_PARAM_DESC
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Set explicit APD on statement",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Verify the active APD is the explicit one via its state
  SQLHDESC current_apd = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);
  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(current_apd, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_USER);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(current_apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

// ============================================================================
// Descriptor Swap — Sharing Across Statements
// ============================================================================

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: Share explicit ARD across two statements",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(9), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Both statements should see the explicit descriptor's state (COUNT == 9)
  SQLHDESC ard1 = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT count1 = -1;
  ret = SQLGetDescField(ard1, 0, SQL_DESC_COUNT, &count1, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count1 == 9);

  SQLHDESC ard2 = get_descriptor(stmt2_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT count2 = -1;
  ret = SQLGetDescField(ard2, 0, SQL_DESC_COUNT, &count2, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count2 == 9);

  // Both report ALLOC_USER since the shared descriptor is explicit
  SQLSMALLINT alloc1 = -1, alloc2 = -1;
  ret = SQLGetDescField(ard1, 0, SQL_DESC_ALLOC_TYPE, &alloc1, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLGetDescField(ard2, 0, SQL_DESC_ALLOC_TYPE, &alloc2, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc1 == SQL_DESC_ALLOC_USER);
  REQUIRE(alloc2 == SQL_DESC_ALLOC_USER);

  // Explicitly revert both statements to implicit before freeing the descriptor.
  // The Windows DM crashes if SQLFreeHandle(SQL_HANDLE_DESC) auto-reverts multiple
  // statements — its internal per-statement descriptor tracking holds stale refs.
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_ROW_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

// ============================================================================
// Descriptor Swap — Free Reverts to Implicit
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Free explicit desc reverts stmt to implicit ARD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(7), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  // After free, the statement should revert to its implicit ARD (ALLOC_AUTO, COUNT 0)
  SQLHDESC current_ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(current_ard, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_AUTO);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(current_ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 0);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Free explicit desc reverts stmt to implicit APD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(4), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  // After free, the statement should revert to its implicit APD (ALLOC_AUTO, COUNT 0)
  SQLHDESC current_apd = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);
  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(current_apd, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_AUTO);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(current_apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 0);
}

// ============================================================================
// Descriptor Swap — Error Cases
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Cannot set IRD via SQLSetStmtAttr",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_IMP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_ERROR);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Cannot set IPD via SQLSetStmtAttr",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_ERROR);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: HY017 when setting foreign implicit ARD as APP_ROW_DESC",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC foreign_ard = get_descriptor(stmt2_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, foreign_ard, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: HY017 when setting foreign implicit APD as APP_PARAM_DESC",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC foreign_apd = get_descriptor(stmt2_handle(), SQL_ATTR_APP_PARAM_DESC);

  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, foreign_apd, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: HY017 when setting foreign implicit ARD as APP_PARAM_DESC",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC foreign_ard = get_descriptor(stmt2_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, foreign_ard, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: HY017 when setting foreign implicit APD as APP_ROW_DESC",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC foreign_apd = get_descriptor(stmt2_handle(), SQL_ATTR_APP_PARAM_DESC);

  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, foreign_apd, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Cannot use explicit descriptor from different connection",
                 "[odbc-api][descriptor][swap][error]") {
  // Allocate a second connection and connect
  SQLHDBC dbc2 = SQL_NULL_HDBC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &dbc2);
  REQUIRE(ret == SQL_SUCCESS);

  struct Cleanup {
    SQLHDBC dbc{SQL_NULL_HDBC};
    SQLHDESC desc{SQL_NULL_HDESC};
    ~Cleanup() {
      if (desc != SQL_NULL_HDESC) (void)SQLFreeHandle(SQL_HANDLE_DESC, desc);
      if (dbc != SQL_NULL_HDBC) {
        (void)SQLDisconnect(dbc);
        (void)SQLFreeHandle(SQL_HANDLE_DBC, dbc);
      }
    }
  } cleanup{dbc2, SQL_NULL_HDESC};

  ret = SQLConnect(dbc2, sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(SQL_SUCCEEDED(ret));

  // Allocate an explicit descriptor on the second connection
  SQLHDESC foreign_explicit = SQL_NULL_HDESC;
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc2, &foreign_explicit);
  REQUIRE(ret == SQL_SUCCESS);
  cleanup.desc = foreign_explicit;

  // Attempt to set it on stmt from the first connection — must fail.
  // iODBC's DM intercepts this and returns HY017 (it treats the foreign handle
  // as automatically allocated); unixODBC/Windows DM pass through to the driver
  // which returns HY024.
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, foreign_explicit, 0);
  IODBC_ONLY { REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT); }
  NON_IODBC { REQUIRE_EXPECTED_ERROR(ret, "HY024", stmt_handle(), SQL_HANDLE_STMT); }
}

// ============================================================================
// Descriptor Swap — End-to-End: Fetch through explicit ARD
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Fetch uses bindings from explicit ARD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR sql[] = "SELECT 42 AS val, 'hello' AS msg";
  ret = SQLExecDirect(stmt_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // SQLBindCol writes to the active (explicit) ARD
  SQLINTEGER int_val = 0;
  SQLLEN int_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_LONG, &int_val, sizeof(int_val), &int_ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR str_val[64] = {};
  SQLLEN str_ind = 0;
  ret = SQLBindCol(stmt_handle(), 2, SQL_C_CHAR, str_val, sizeof(str_val), &str_ind);
  REQUIRE(ret == SQL_SUCCESS);

  // The driver must read bindings from the explicit ARD during fetch
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(int_val == 42);
  REQUIRE(int_ind == sizeof(SQLINTEGER));
  REQUIRE(std::string(reinterpret_cast<char*>(str_val)) == "hello");
  REQUIRE(str_ind == 5);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture,
                 "Descriptor swap: Two statements sharing explicit ARD fetch into same buffers",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Bind a column via stmt1 — writes to the shared explicit ARD
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute and fetch on stmt2 — shares the ARD, should write to the same buffer
  SQLCHAR sql[] = "SELECT 99 AS num";
  ret = SQLExecDirect(stmt2_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt2_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 99);
  REQUIRE(indicator == sizeof(SQLINTEGER));

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_ROW_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}
