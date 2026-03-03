#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(ard, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = 0;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy between explicit descriptors",
                 "[odbc-api][copydesc][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC desc1 = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &desc1);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC desc2 = SQL_NULL_HDESC;
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &desc2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(desc1, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(desc1, desc2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = 0;
  ret = SQLGetDescField(desc2, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);

  SQLFreeHandle(SQL_HANDLE_DESC, desc1);
  SQLFreeHandle(SQL_HANDLE_DESC, desc2);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy explicit descriptor to ARD",
                 "[odbc-api][copydesc][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  ret = SQLCopyDesc(explicit_desc, ard);
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: Copy overwrites existing bindings",
                 "[odbc-api][copydesc][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT count_before = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count_before, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count_before == 1);

  SQLHDESC empty_desc = SQL_NULL_HDESC;
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &empty_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(empty_desc, ard);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count_after = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count_after, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count_after == 0);

  SQLFreeHandle(SQL_HANDLE_DESC, empty_desc);
}

// ============================================================================
// SQLCopyDesc - Implementation Descriptor Copies
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY021 - Copy IRD to explicit descriptor",
                 "[odbc-api][copydesc][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: Per ODBC spec, copying from IRD after execution should succeed.
  // The reference driver returns HY021 (Inconsistent descriptor information)
  // with "Illegal descriptor concise type" for any copy from an IRD.
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 AS COL1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(ird, explicit_desc);
  REQUIRE_EXPECTED_ERROR(ret, "HY021", explicit_desc, SQL_HANDLE_DESC);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY021 - Copy IRD to ARD on same statement",
                 "[odbc-api][copydesc][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: Reference driver rejects all copies from implementation descriptors.
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(ipd, explicit_desc);
  REQUIRE_EXPECTED_ERROR(ret, "HY021", explicit_desc, SQL_HANDLE_DESC);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY021 - Copy APD to IPD", "[odbc-api][copydesc][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLCopyDesc(SQL_NULL_HDESC, SQL_NULL_HDESC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: SQL_INVALID_HANDLE for null source",
                 "[odbc-api][copydesc][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(SQL_NULL_HDESC, explicit_desc);
  REQUIRE(ret == SQL_INVALID_HANDLE);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: SQL_INVALID_HANDLE for null target",
                 "[odbc-api][copydesc][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  const SQLRETURN ret = SQLCopyDesc(ard, SQL_NULL_HDESC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY016 - Cannot copy into IRD",
                 "[odbc-api][copydesc][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  ret = SQLCopyDesc(ard, ird);
  REQUIRE_EXPECTED_ERROR(ret, "HY016", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY007 - IRD source from unprepared statement",
                 "[odbc-api][copydesc][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCopyDesc(ird, explicit_desc);
  REQUIRE_EXPECTED_ERROR(ret, "HY007", explicit_desc, SQL_HANDLE_DESC);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCopyDesc: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][copydesc][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Get descriptors before entering NEED_DATA state
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
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
  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}
