#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_descriptor.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLSetDescRec - Setting Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Bind column via ARD and fetch",
                 "[odbc-api][setdescrec][descriptor]") {
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
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLINTEGER val = 0;
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret = SQLSetDescRec(explicit_desc, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Set IPD record for consistency check",
                 "[odbc-api][setdescrec][descriptor]") {
  const SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  const SQLRETURN ret = SQLSetDescRec(ipd, 1, SQL_INTEGER, 0, 4, 10, 0, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: RecNumber beyond count increases DESC_COUNT",
                 "[odbc-api][setdescrec][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLINTEGER val = 0;
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret = SQLSetDescRec(explicit_desc, 5, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 5);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: DataPtr NULL unbinds ARD column",
                 "[odbc-api][setdescrec][descriptor]") {
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
  const SQLRETURN ret = SQLSetDescRec(SQL_NULL_HDESC, 1, SQL_C_SLONG, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: HY016 - Cannot modify IRD",
                 "[odbc-api][setdescrec][descriptor][error]") {
  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLRETURN ret = SQLSetDescRec(ird, 1, SQL_INTEGER, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY016", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: 07009 - RecNumber 0 on IPD",
                 "[odbc-api][setdescrec][descriptor][error]") {
  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescRec(ipd, 0, SQL_INTEGER, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: 07009 - Negative RecNumber",
                 "[odbc-api][setdescrec][descriptor][error]") {
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescRec(ard, -1, SQL_C_SLONG, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: HY021 - Invalid descriptor type",
                 "[odbc-api][setdescrec][descriptor][error]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret = SQLSetDescRec(explicit_desc, 1, 9999, 0, 4, 0, 0, nullptr, nullptr, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY021", explicit_desc, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][setdescrec][descriptor][error]") {
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
  OLD_IODBC_ONLY("BD#70") {
    // The old driver doesn't gate descriptor mutations on SQL_NEED_DATA and
    //   silently accepts SQLSetDescRec mid-DAE; the new driver enforces
    //   "HY010" itself before reaching the descriptor.
    REQUIRE(ret == SQL_SUCCESS);
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY010", ard, SQL_HANDLE_DESC);
  }

  SQLCancel(stmt_handle());
}

// ============================================================================
// SQLSetDescRec - Multiple Record Bindings
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Multiple ARD records and fetch",
                 "[odbc-api][setdescrec][descriptor]") {
  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLINTEGER col1 = 0, col2 = 0;
  SQLLEN ind1 = 0, ind2 = 0, olen1 = 0, olen2 = 0;

  SQLRETURN ret = SQLSetDescRec(ard, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &col1, &olen1, &ind1);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescRec(ard, 2, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &col2, &olen2, &ind2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 10, 20"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col1 == 10);
  REQUIRE(col2 == 20);
}

// ============================================================================
// SQLSetDescRec - CHAR Type with Length
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Bind CHAR column via ARD and fetch",
                 "[odbc-api][setdescrec][descriptor]") {
  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret = SQLSetDescRec(ard, 1, SQL_C_CHAR, 0, sizeof(buf), 0, 0, buf, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 'hello'"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(reinterpret_cast<char*>(buf)) == "hello");
}

// ============================================================================
// SQLSetDescRec - Overwrite Existing Record
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Overwrite existing binding with new type",
                 "[odbc-api][setdescrec][descriptor]") {
  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLINTEGER int_val = 0;
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret = SQLSetDescRec(ard, 1, SQL_C_SLONG, 0, sizeof(SQLINTEGER), 0, 0, &int_val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_SLONG);

  SQLCHAR char_val[32] = {};
  ret = SQLSetDescRec(ard, 1, SQL_C_CHAR, 0, sizeof(char_val), 0, 0, char_val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_CHAR);

  SQLPOINTER dptr = nullptr;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &dptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dptr == char_val);
}

// ============================================================================
// SQLSetDescRec - Datetime Type
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: Set SQL_TYPE_TIMESTAMP on explicit descriptor",
                 "[odbc-api][setdescrec][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQL_TIMESTAMP_STRUCT ts_val = {};
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret = SQLSetDescRec(explicit_desc, 1, SQL_C_TYPE_TIMESTAMP, 0, sizeof(SQL_TIMESTAMP_STRUCT), 6, 0, &ts_val,
                                &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_DATETIME);

  SQLSMALLINT code = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_DATETIME_INTERVAL_CODE, &code, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(code == SQL_CODE_TIMESTAMP);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLSetDescRec: SQL_DATETIME with SubType sets concise TYPE and DATETIME_INTERVAL_CODE",
                 "[odbc-api][setdescrec][descriptor]") {
  // Per the ODBC spec, when Type is SQL_DATETIME the SubType argument carries
  // the SQL_DESC_DATETIME_INTERVAL_CODE and selects the concise datetime type.
  // SQLSetDescRec(SQL_DATETIME, SQL_CODE_TIMESTAMP) must therefore be equivalent
  // to setting the TIMESTAMP concise type (regression for the previously-dropped
  // SubType argument).
  HandleWrapper desc(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = desc.getHandle();

  SQL_TIMESTAMP_STRUCT ts_val = {};
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret = SQLSetDescRec(explicit_desc, 1, SQL_DATETIME, SQL_CODE_TIMESTAMP, sizeof(SQL_TIMESTAMP_STRUCT), 6, 0,
                                &ts_val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_DATETIME);

  SQLSMALLINT concise = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, &concise, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(concise == SQL_C_TYPE_TIMESTAMP);

  SQLSMALLINT code = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_DATETIME_INTERVAL_CODE, &code, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(code == SQL_CODE_TIMESTAMP);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescRec: precision and scale of 0 overwrite prior values",
                 "[odbc-api][setdescrec][descriptor]") {
  // SQLSetDescRec sets SQL_DESC_PRECISION / SQL_DESC_SCALE directly, so a second
  // call with 0 must overwrite the previously-set non-zero values (regression for
  // the `if precision != 0` / `if scale != 0` guard that silently skipped 0).
  HandleWrapper desc(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = desc.getHandle();

  SQL_NUMERIC_STRUCT num_val = {};
  SQLLEN ind = 0, olen = 0;
  SQLRETURN ret =
      SQLSetDescRec(explicit_desc, 1, SQL_C_NUMERIC, 0, sizeof(SQL_NUMERIC_STRUCT), 18, 4, &num_val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT precision = -1, scale = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_PRECISION, &precision, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(precision == 18);
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_SCALE, &scale, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(scale == 4);

  // Re-set the same record with 0/0 — must overwrite, not skip.
  ret = SQLSetDescRec(explicit_desc, 1, SQL_C_NUMERIC, 0, sizeof(SQL_NUMERIC_STRUCT), 0, 0, &num_val, &olen, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_PRECISION, &precision, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(precision == 0);
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_SCALE, &scale, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(scale == 0);
}
