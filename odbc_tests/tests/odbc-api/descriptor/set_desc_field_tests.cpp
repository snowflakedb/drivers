#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>

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
// SQLSetDescField - Header Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set DESC_COUNT on explicit descriptor",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 3);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set DESC_ARRAY_SIZE on ARD",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 0, SQL_DESC_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(5), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN arr_sz = 0;
  ret = SQLGetDescField(ard, 0, SQL_DESC_ARRAY_SIZE, &arr_sz, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(arr_sz == 5);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Decreasing DESC_COUNT unbinds higher records",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER col1 = 0, col2 = 0;
  SQLLEN ind1 = 0, ind2 = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col1, 0, &ind1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLBindCol(stmt_handle(), 2, SQL_C_SLONG, &col2, 0, &ind2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);

  ret = SQLSetDescField(ard, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);
}

// ============================================================================
// SQLSetDescField - Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set DESC_TYPE on ARD record",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_SLONG), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_SLONG);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set CONCISE_TYPE sets TYPE implicitly",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, reinterpret_cast<SQLPOINTER>(SQL_INTEGER), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT concise = -1, dtype = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, &concise, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(concise == SQL_INTEGER);

  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_TYPE, &dtype, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dtype == SQL_INTEGER);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set DATA_PTR on ARD triggers consistency check",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_SLONG), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER data_val = 42;
  ret = SQLSetDescField(ard, 1, SQL_DESC_DATA_PTR, &data_val, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLPOINTER ptr = nullptr;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ptr == &data_val);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set NAME on IPD for named parameters",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_NAME, sqlchar("PARAM1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  char name[64] = {};
  SQLINTEGER name_len = 0;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_NAME, name, sizeof(name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(name) == "PARAM1");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set UNNAMED to SQL_UNNAMED on IPD",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_NAME, sqlchar("P1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(ipd, 1, SQL_DESC_UNNAMED, reinterpret_cast<SQLPOINTER>(SQL_UNNAMED), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLSetDescField - IRD Writable Exceptions
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: ARRAY_STATUS_PTR allowed on IRD",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLUSMALLINT status_arr[1] = {};
  const SQLRETURN ret = SQLSetDescField(ird, 0, SQL_DESC_ARRAY_STATUS_PTR, status_arr, 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: ROWS_PROCESSED_PTR allowed on IRD",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLULEN rows_proc = 0;
  const SQLRETURN ret = SQLSetDescField(ird, 0, SQL_DESC_ROWS_PROCESSED_PTR, &rows_proc, 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLSetDescField - Error Cases
// ============================================================================

TEST_CASE("SQLSetDescField: SQL_INVALID_HANDLE for null descriptor", "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLSetDescField(SQL_NULL_HDESC, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY091 - Read-only field ALLOC_TYPE",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 0, SQL_DESC_ALLOC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_DESC_ALLOC_USER), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY091", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY091 - Invalid field identifier",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 0, 9999, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY091", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY016 - Cannot modify IRD header field",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ird, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY016", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY016 - Cannot modify IRD record field",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ird, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_INTEGER), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY016", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY016 - Cannot set NAME on IRD",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 AS X"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  ret = SQLSetDescField(ird, 1, SQL_DESC_NAME, sqlchar("NEW_NAME"), SQL_NTS);
  WINDOWS_ONLY {
    // Windows DM intercepts the call and returns HY091 (descriptor type out of range)
    REQUIRE_EXPECTED_ERROR(ret, "HY091", ird, SQL_HANDLE_DESC);
  }
  UNIX_ONLY {
    // Note: The ODBC spec says HY091 for setting a read-only field on IRD.
    // The reference driver returns HY016 (cannot modify IRD) for all IRD writes.
    REQUIRE_EXPECTED_ERROR(ret, "HY016", ird, SQL_HANDLE_DESC);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: 07009 - RecNumber 0 on IPD for record field",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 0, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_INTEGER), 0);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: 07009 - Negative RecNumber",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, -1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_SLONG), 0);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY092 - Set UNNAMED to SQL_NAMED on IPD",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_UNNAMED, SQL_NAMED, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY092", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][setdescfield][descriptor][error]") {
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

  ret = SQLSetDescField(ard, 0, SQL_DESC_COUNT, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", ard, SQL_HANDLE_DESC);

  SQLCancel(stmt_handle());
}

// ============================================================================
// SQLSetDescField - IPD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set PARAMETER_TYPE on IPD",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_PARAMETER_TYPE, reinterpret_cast<SQLPOINTER>(SQL_PARAM_INPUT), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT ptype = -1;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_PARAMETER_TYPE, &ptype, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ptype == SQL_PARAM_INPUT);
}

// ============================================================================
// SQLSetDescField - Deferred Fields on Application Descriptors
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set INDICATOR_PTR on ARD",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_SLONG), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN ind_var = 0;
  ret = SQLSetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &ind_var, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLPOINTER ptr = nullptr;
  ret = SQLGetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ptr == &ind_var);
}

// ============================================================================
// SQLSetDescField - Unbinding Behavior
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Setting non-deferred field unbinds record",
                 "[odbc-api][setdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER col_val = 0;
  SQLLEN ind = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLPOINTER dptr = nullptr;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &dptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dptr == &col_val);

  ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, reinterpret_cast<SQLPOINTER>(10), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &dptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dptr == nullptr);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: 07009 - DESC_COUNT set to negative value",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(-1), 0);
  WINDOWS_ONLY {
    // Windows DM returns HY024 (Invalid argument value) for negative DESC_COUNT
    REQUIRE_EXPECTED_ERROR(ret, "HY024", ard, SQL_HANDLE_DESC);
  }
  UNIX_ONLY { REQUIRE_EXPECTED_ERROR(ret, "07009", ard, SQL_HANDLE_DESC); }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY090 - Negative BufferLength for string field",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_NAME, sqlchar("TEST"), -5);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY105 - Invalid parameter type value",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_PARAMETER_TYPE, reinterpret_cast<SQLPOINTER>(9999), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY105", ipd, SQL_HANDLE_DESC);
}
