#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_descriptor.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLCopyDesc - Application Descriptor Copies
// ============================================================================

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "SQLCopyDesc: Copy ARD between statements",
                 "[odbc-api][copydesc][descriptor]") {
  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard1 = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLHDESC ard2 = get_descriptor(stmt2_handle(), SQL_ATTR_APP_ROW_DESC);

  ret = SQLCopyDesc(ard1, ard2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt2_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt2_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_val == 42);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "SQLCopyDesc: Copy APD between statements",
                 "[odbc-api][copydesc][descriptor]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 77;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt2_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC apd1 = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);
  SQLHDESC apd2 = get_descriptor(stmt2_handle(), SQL_ATTR_APP_PARAM_DESC);

  ret = SQLCopyDesc(apd1, apd2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt2_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt2_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt2_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 77);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy ARD to itself preserves bindings",
                 "[odbc-api][copydesc][descriptor]") {
  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  ret = SQLCopyDesc(ard, ard);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy ARD to explicit descriptor",
                 "[odbc-api][copydesc][descriptor]") {
  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  ret = SQLCopyDesc(ard, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = 0;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy between explicit descriptors",
                 "[odbc-api][copydesc][descriptor]") {
  HandleWrapper desc1_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc1 = desc1_guard.getHandle();

  HandleWrapper desc2_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc2 = desc2_guard.getHandle();

  SQLRETURN ret = SQLSetDescField(desc1, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(desc1, desc2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = 0;
  ret = SQLGetDescField(desc2, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy explicit descriptor to ARD",
                 "[odbc-api][copydesc][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLCopyDesc(explicit_desc, ard);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy overwrites existing bindings",
                 "[odbc-api][copydesc][descriptor]") {
  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT count_before = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count_before, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count_before == 1);

  HandleWrapper empty_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC empty_desc = empty_desc_guard.getHandle();

  ret = SQLCopyDesc(empty_desc, ard);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count_after = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count_after, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count_after == 0);
}

// ============================================================================
// SQLCopyDesc - Implementation Descriptor Copies
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY021 - Copy IRD to explicit descriptor",
                 "[odbc-api][copydesc][descriptor]") {
  // Note: Per ODBC spec, copying from IRD after execution should succeed.
  // The reference driver returns HY021 (Inconsistent descriptor information)
  // with "Illegal descriptor concise type" for any copy from an IRD.
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 AS COL1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  ret = SQLCopyDesc(ird, explicit_desc);
  REQUIRE_EXPECTED_ERROR(ret, "HY021", explicit_desc, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY021 - Copy IRD to ARD on same statement",
                 "[odbc-api][copydesc][descriptor]") {
  // Note: Same HY021 failure as IRD-to-explicit copy.
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  ret = SQLCopyDesc(ird, ard);
  REQUIRE_EXPECTED_ERROR(ret, "HY021", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY021 - Copy IPD to explicit descriptor",
                 "[odbc-api][copydesc][descriptor]") {
  // Note: Reference driver rejects all copies from implementation descriptors.
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  ret = SQLCopyDesc(ipd, explicit_desc);
  REQUIRE_EXPECTED_ERROR(ret, "HY021", explicit_desc, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY021 - Copy APD to IPD", "[odbc-api][copydesc][descriptor]") {
  // Note: Per ODBC spec, IPD is a valid copy target. The reference driver
  // returns HY021 for any copy involving implementation descriptors.
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC apd = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);
  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  ret = SQLCopyDesc(apd, ipd);
  REQUIRE_EXPECTED_ERROR(ret, "HY021", ipd, SQL_HANDLE_DESC);
}

// ============================================================================
// SQLCopyDesc - Error Cases
// ============================================================================

TEST_CASE("SQLCopyDesc: SQL_INVALID_HANDLE for null source and target", "[odbc-api][copydesc][descriptor][error]") {
  const SQLRETURN ret = SQLCopyDesc(SQL_NULL_HDESC, SQL_NULL_HDESC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: SQL_INVALID_HANDLE for null source",
                 "[odbc-api][copydesc][descriptor][error]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  const SQLRETURN ret = SQLCopyDesc(SQL_NULL_HDESC, explicit_desc);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: SQL_INVALID_HANDLE for null target",
                 "[odbc-api][copydesc][descriptor][error]") {
  // Under iODBC the DriverManager services SQLCopyDesc inside its own
  // handle-dispatch layer and SIGSEGVs when the target is a null handle
  // (BD#59 / BD#932) — the crash happens in the DM before either driver's
  // SQLCopyDesc runs, so it is identical for the new and old driver. Skip for
  // both drivers under iODBC (a SIGSEGV cannot be asserted around anyway); the
  // null-handle contract is still covered on unixODBC and Windows below.
  SKIP_IODBC("BD#59 - iODBC DM SegFaults on SQLCopyDesc with a null target (both drivers)");

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  const SQLRETURN ret = SQLCopyDesc(ard, SQL_NULL_HDESC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY016 - Cannot copy into IRD",
                 "[odbc-api][copydesc][descriptor][error]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  ret = SQLCopyDesc(ard, ird);
  REQUIRE_EXPECTED_ERROR(ret, "HY016", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY007 - IRD source from unprepared statement",
                 "[odbc-api][copydesc][descriptor][error]") {
  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret = SQLCopyDesc(ird, explicit_desc);
  REQUIRE_EXPECTED_ERROR(ret, "HY007", explicit_desc, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY010 - Called during SQL_NEED_DATA (explicit desc)",
                 "[odbc-api][copydesc][descriptor][error]") {
  // Under iODBC the DriverManager implements SQLCopyDesc itself (copying the
  // descriptor field-by-field via SQLGetDescField/SQLSetDescField) rather than
  // dispatching to the driver, so the driver's SQL_NEED_DATA state-gate never
  // runs and the outcome reflects the DM, not the driver (old driver: silent
  // SQL_SUCCESS per BD#70; new driver: an opaque SQL_ERROR from the DM's field
  // copy). Neither driver's SQLCopyDesc logic is exercised here under iODBC, so
  // skip for both drivers; the HY010 contract stays asserted on unixODBC/Windows.
  SKIP_IODBC("BD#70 - iODBC DM implements SQLCopyDesc; the driver NEED_DATA gate is not exercised (both drivers)");

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCopyDesc(ard, explicit_desc);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", ard, SQL_HANDLE_DESC);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][copydesc][descriptor][error]") {
  // Under iODBC the DriverManager implements SQLCopyDesc itself (copying the
  // descriptor field-by-field via SQLGetDescField/SQLSetDescField) rather than
  // dispatching to the driver, so the driver's SQL_NEED_DATA state-gate never
  // runs and the outcome reflects the DM, not the driver (old driver: silent
  // SQL_SUCCESS per BD#69; new driver: an opaque SQL_ERROR from the DM's field
  // copy). Neither driver's SQLCopyDesc logic is exercised here under iODBC, so
  // skip for both drivers; the HY010 contract stays asserted on unixODBC/Windows.
  SKIP_IODBC("BD#69 - iODBC DM implements SQLCopyDesc; the driver NEED_DATA gate is not exercised (both drivers)");

  // Given the implicit ARD of the default statement (the source descriptor)
  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  // And a second statement on the same connection whose implicit ARD serves as the
  // copy target
  SQLHSTMT stmt2 = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt2);
  REQUIRE(ret == SQL_SUCCESS);
  const SQLHDESC target_ard = get_descriptor(stmt2, SQL_ATTR_APP_ROW_DESC);

  // And the source statement is driven into SQL_NEED_DATA via a SQL_DATA_AT_EXEC bind
  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  // When SQLCopyDesc is called while the source statement is in SQL_NEED_DATA
  ret = SQLCopyDesc(ard, target_ard);
  // Then the driver surfaces HY010 on the source ARD. (Under iODBC the DM
  //   services SQLCopyDesc itself and this case is skipped above for both
  //   drivers.)
  REQUIRE_EXPECTED_ERROR(ret, "HY010", ard, SQL_HANDLE_DESC);

  // And the statement is cancelled to release any pending state and the helper stmt
  // is freed
  SQLCancel(stmt_handle());
  SQLFreeHandle(SQL_HANDLE_STMT, stmt2);
}

// ============================================================================
// SQLCopyDesc - Preservation of Record Field Values
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy preserves TYPE and OCTET_LENGTH",
                 "[odbc-api][copydesc][descriptor]") {
  HandleWrapper desc1_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc1 = desc1_guard.getHandle();

  HandleWrapper desc2_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc2 = desc2_guard.getHandle();

  SQLRETURN ret = SQLSetDescField(desc1, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_CHAR), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(desc1, 1, SQL_DESC_OCTET_LENGTH, reinterpret_cast<SQLPOINTER>(128), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(desc1, desc2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(desc2, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_CHAR);

  SQLLEN octet_len = 0;
  ret = SQLGetDescField(desc2, 1, SQL_DESC_OCTET_LENGTH, &octet_len, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(octet_len == 128);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy preserves multiple record bindings",
                 "[odbc-api][copydesc][descriptor]") {
  HandleWrapper desc1_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc1 = desc1_guard.getHandle();

  HandleWrapper desc2_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc2 = desc2_guard.getHandle();

  SQLRETURN ret = SQLSetDescField(desc1, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_SLONG), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(desc1, 2, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_CHAR), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(desc1, 3, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_DOUBLE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(desc1, desc2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(desc2, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 3);

  SQLSMALLINT type1 = -1, type2 = -1, type3 = -1;
  ret = SQLGetDescField(desc2, 1, SQL_DESC_TYPE, &type1, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type1 == SQL_C_SLONG);

  ret = SQLGetDescField(desc2, 2, SQL_DESC_TYPE, &type2, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type2 == SQL_C_CHAR);

  ret = SQLGetDescField(desc2, 3, SQL_DESC_TYPE, &type3, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type3 == SQL_C_DOUBLE);
}

// ============================================================================
// SQLCopyDesc - Copy Does Not Share State (Independent After Copy)
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Modifying source after copy does not affect target",
                 "[odbc-api][copydesc][descriptor]") {
  HandleWrapper desc1_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc1 = desc1_guard.getHandle();

  HandleWrapper desc2_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc2 = desc2_guard.getHandle();

  SQLRETURN ret = SQLSetDescField(desc1, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(desc1, desc2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(desc1, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(7), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count1 = -1, count2 = -1;
  ret = SQLGetDescField(desc1, 0, SQL_DESC_COUNT, &count1, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count1 == 7);

  ret = SQLGetDescField(desc2, 0, SQL_DESC_COUNT, &count2, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count2 == 3);
}

// ============================================================================
// SQLCopyDesc - Multi-Column ARD Copy and Fetch
// ============================================================================

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "SQLCopyDesc: Copy multi-column ARD to second statement and fetch",
                 "[odbc-api][copydesc][descriptor]") {
  SQLINTEGER col1 = 0, col2 = 0;
  SQLLEN ind1 = 0, ind2 = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col1, 0, &ind1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLBindCol(stmt_handle(), 2, SQL_C_SLONG, &col2, 0, &ind2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard1 = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLHDESC ard2 = get_descriptor(stmt2_handle(), SQL_ATTR_APP_ROW_DESC);

  ret = SQLCopyDesc(ard1, ard2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt2_handle(), sqlchar("SELECT 100, 200"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt2_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col1 == 100);
  REQUIRE(col2 == 200);
}

// ============================================================================
// SQLCopyDesc - Header Field Preservation
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy preserves ARRAY_SIZE header",
                 "[odbc-api][copydesc][descriptor]") {
  HandleWrapper desc1_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc1 = desc1_guard.getHandle();

  HandleWrapper desc2_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC desc2 = desc2_guard.getHandle();

  SQLRETURN ret = SQLSetDescField(desc1, 0, SQL_DESC_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(10), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(desc1, desc2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN arr_sz = 0;
  ret = SQLGetDescField(desc2, 0, SQL_DESC_ARRAY_SIZE, &arr_sz, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(arr_sz == 10);
}
