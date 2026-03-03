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
// SQLSetDescRec - Setting Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Bind column via ARD and fetch",
                 "[odbc-api][setdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLINTEGER col_val = 0;
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret = SQLSetDescRec(ard, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &col_val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Verify ARD fields after setting",
                 "[odbc-api][setdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLINTEGER col_val = 0;
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret = SQLSetDescRec(ard, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &col_val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_SLONG);

  SQLPOINTER dptr = nullptr;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &dptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dptr == &col_val);

  SQLPOINTER iptr = nullptr;
  ret = SQLGetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &iptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(iptr == &ind);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Set APD record for parameter binding",
                 "[odbc-api][setdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC apd = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);
  const SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLINTEGER param_val = 55;
  SQLLEN param_ind = 0, param_olen = sizeof(SQLINTEGER);
  SQLRETURN ret = SQLSetDescRec(apd, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &param_val, &param_olen, &param_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescRec(ipd, 1, SQL_INTEGER, 0, 4, 10, 0, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, sizeof(result), &result_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 55);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Set explicit descriptor record",
                 "[odbc-api][setdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLLEN ind = 0, olen = 0;
  ret = SQLSetDescRec(explicit_desc, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Set IPD record for consistency check",
                 "[odbc-api][setdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  const SQLRETURN ret = SQLSetDescRec(ipd, 1, SQL_INTEGER, 0, 4, 10, 0, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: RecNumber beyond count increases DESC_COUNT",
                 "[odbc-api][setdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLLEN ind = 0, olen = 0;
  ret = SQLSetDescRec(explicit_desc, 5, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 5);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: DataPtr NULL unbinds ARD column",
                 "[odbc-api][setdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLINTEGER col_val = 0;
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret = SQLSetDescRec(ard, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &col_val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLPOINTER dptr = nullptr;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &dptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dptr == &col_val);

  ret = SQLSetDescRec(ard, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &dptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dptr == nullptr);
}

// ============================================================================
// SQLSetDescRec - Error Cases
// ============================================================================

TEST_CASE("SQLSetDescRec: SQL_INVALID_HANDLE for null descriptor", "[odbc-api][setdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLSetDescRec(SQL_NULL_HDESC, 1, SQL_C_SLONG, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: HY016 - Cannot modify IRD",
                 "[odbc-api][setdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLRETURN ret = SQLSetDescRec(ird, 1, SQL_INTEGER, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY016", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: 07009 - RecNumber 0 on IPD",
                 "[odbc-api][setdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescRec(ipd, 0, SQL_INTEGER, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: 07009 - Negative RecNumber",
                 "[odbc-api][setdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescRec(ard, -1, SQL_C_SLONG, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: HY021 - Invalid descriptor type",
                 "[odbc-api][setdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescRec(explicit_desc, 1, 9999, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY021", explicit_desc, SQL_HANDLE_DESC);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][setdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLINTEGER val = 0;
  SQLLEN ind = 0, olen = 0;
  ret = SQLSetDescRec(ard, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &val, &olen, &ind);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", ard, SQL_HANDLE_DESC);

  SQLCancel(stmt_handle());
}
