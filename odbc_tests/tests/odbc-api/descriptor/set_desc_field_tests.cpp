#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

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
// SQLSetDescField - Header Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set DESC_COUNT on explicit descriptor",
                 "[odbc-api][setdescfield][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 3);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set DESC_ARRAY_SIZE on ARD",
                 "[odbc-api][setdescfield][descriptor]") {
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
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret =
      SQLSetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, reinterpret_cast<SQLPOINTER>(SQL_INTEGER), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT concise = -1, dtype = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, &concise, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(concise == SQL_INTEGER);

  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_TYPE, &dtype, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dtype == SQL_INTEGER);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set DATA_PTR on ARD triggers consistency check",
                 "[odbc-api][setdescfield][descriptor]") {
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
  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLUSMALLINT status_arr[1] = {};
  const SQLRETURN ret = SQLSetDescField(ird, 0, SQL_DESC_ARRAY_STATUS_PTR, status_arr, 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: ROWS_PROCESSED_PTR allowed on IRD",
                 "[odbc-api][setdescfield][descriptor]") {
  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLULEN rows_proc = 0;
  const SQLRETURN ret = SQLSetDescField(ird, 0, SQL_DESC_ROWS_PROCESSED_PTR, &rows_proc, 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLSetDescField - Error Cases
// ============================================================================

TEST_CASE("SQLSetDescField: SQL_INVALID_HANDLE for null descriptor", "[odbc-api][setdescfield][descriptor][error]") {
  const SQLRETURN ret = SQLSetDescField(SQL_NULL_HDESC, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY091 - Read-only field ALLOC_TYPE",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 0, SQL_DESC_ALLOC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_DESC_ALLOC_USER), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY091", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY091 - Invalid field identifier",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 0, 9999, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY091", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY016 - Cannot modify IRD header field",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ird, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY016", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY016 - Cannot modify IRD record field",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ird, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_INTEGER), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY016", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY016 - Cannot set NAME on IRD",
                 "[odbc-api][setdescfield][descriptor][error]") {
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
  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 0, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_INTEGER), 0);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: 07009 - Negative RecNumber",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, -1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_SLONG), 0);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY092 - Set UNNAMED to SQL_NAMED on IPD",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_UNNAMED, SQL_NAMED, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY092", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][setdescfield][descriptor][error]") {
  // Given the implicit ARD of a statement driven into SQL_NEED_DATA via a
  // SQL_DATA_AT_EXEC bind
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  // When SQLSetDescField is called on the ARD while the parent statement is in
  // SQL_NEED_DATA
  ret = SQLSetDescField(ard, 0, SQL_DESC_COUNT, nullptr, 0);

  OLD_IODBC_ONLY("BD#69") {
    // Then driver bypasses the SQL_NEED_DATA state-check for the
    //   descriptor entry point and silently returns SQL_SUCCESS.
    REQUIRE(ret == SQL_SUCCESS);
  }
  else {
    // Then DM surfaces HY010
    REQUIRE_EXPECTED_ERROR(ret, "HY010", ard, SQL_HANDLE_DESC);
  }

  // And the statement is cancelled to release any pending state
  SQLCancel(stmt_handle());
}

// ============================================================================
// SQLSetDescField - IPD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set PARAMETER_TYPE on IPD",
                 "[odbc-api][setdescfield][descriptor]") {
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
  // Under iODBC the DriverManager owns BufferLength validation for the string
  // descriptor fields and reshapes the call before dispatch (BD#62), in a
  // libiodbc-version-dependent way: it either rejects the negative BufferLength
  // itself with S1090, or forwards the call with a nulled value pointer (so the
  // driver sees HY009, not HY090) / silently accepts it. The outcome is DM- and
  // version-dependent, not driver-controlled, so skip under iODBC for both
  // drivers; the HY090 contract stays asserted on unixODBC and Windows.
  SKIP_IODBC("BD#62 - iODBC DM owns SQLSetDescField string BufferLength validation (both drivers)");

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_NAME, sqlchar("TEST"), -5);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: HY105 - Invalid parameter type value",
                 "[odbc-api][setdescfield][descriptor][error]") {
  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_PARAMETER_TYPE, reinterpret_cast<SQLPOINTER>(9999), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY105", ipd, SQL_HANDLE_DESC);
}

// ============================================================================
// SQLSetDescField - Record Beyond Count Auto-Extends
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Setting record beyond count auto-extends DESC_COUNT",
                 "[odbc-api][setdescfield][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLSMALLINT count = -1;
  SQLRETURN ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 0);

  ret = SQLSetDescField(explicit_desc, 5, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_SLONG), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 5);
}

// ============================================================================
// SQLSetDescField - CONCISE_TYPE Datetime/Interval Mapping
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLSetDescField: CONCISE_TYPE SQL_TYPE_TIMESTAMP sets TYPE and DATETIME_INTERVAL_CODE",
                 "[odbc-api][setdescfield][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret =
      SQLSetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, reinterpret_cast<SQLPOINTER>(SQL_TYPE_TIMESTAMP), 0);
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
                 "SQLSetDescField: CONCISE_TYPE SQL_TYPE_DATE sets TYPE and DATETIME_INTERVAL_CODE",
                 "[odbc-api][setdescfield][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret =
      SQLSetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, reinterpret_cast<SQLPOINTER>(SQL_TYPE_DATE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_DATETIME);

  SQLSMALLINT code = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_DATETIME_INTERVAL_CODE, &code, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(code == SQL_CODE_DATE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLSetDescField: CONCISE_TYPE SQL_TYPE_TIME sets TYPE and DATETIME_INTERVAL_CODE",
                 "[odbc-api][setdescfield][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret =
      SQLSetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, reinterpret_cast<SQLPOINTER>(SQL_TYPE_TIME), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_DATETIME);

  SQLSMALLINT code = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_DATETIME_INTERVAL_CODE, &code, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(code == SQL_CODE_TIME);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Non-datetime CONCISE_TYPE sets DATETIME_INTERVAL_CODE to 0",
                 "[odbc-api][setdescfield][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret =
      SQLSetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, reinterpret_cast<SQLPOINTER>(SQL_TYPE_TIMESTAMP), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 1, SQL_DESC_CONCISE_TYPE, reinterpret_cast<SQLPOINTER>(SQL_INTEGER), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_INTEGER);

  SQLSMALLINT code = -1;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_DATETIME_INTERVAL_CODE, &code, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(code == 0);
}

// ============================================================================
// SQLSetDescField - OCTET_LENGTH and LENGTH Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set OCTET_LENGTH on ARD record",
                 "[odbc-api][setdescfield][descriptor]") {
  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_CHAR), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, reinterpret_cast<SQLPOINTER>(256), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN octet_len = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, &octet_len, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(octet_len == 256);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set LENGTH on explicit descriptor",
                 "[odbc-api][setdescfield][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret = SQLSetDescField(explicit_desc, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_CHAR), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 1, SQL_DESC_LENGTH, reinterpret_cast<SQLPOINTER>(100), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN length = 0;
  ret = SQLGetDescField(explicit_desc, 1, SQL_DESC_LENGTH, &length, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(length == 100);
}

// ============================================================================
// SQLSetDescField - PRECISION and SCALE Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set PRECISION on IPD",
                 "[odbc-api][setdescfield][descriptor]") {
  const SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_DECIMAL), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(ipd, 1, SQL_DESC_PRECISION, reinterpret_cast<SQLPOINTER>(18), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT precision = -1;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_PRECISION, &precision, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(precision == 18);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set SCALE on IPD", "[odbc-api][setdescfield][descriptor]") {
  const SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLRETURN ret = SQLSetDescField(ipd, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_DECIMAL), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(ipd, 1, SQL_DESC_SCALE, reinterpret_cast<SQLPOINTER>(4), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT scale = -1;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_SCALE, &scale, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(scale == 4);
}

// ============================================================================
// SQLSetDescField - BIND_OFFSET_PTR Header Field
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set BIND_OFFSET_PTR on ARD",
                 "[odbc-api][setdescfield][descriptor]") {
  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLULEN offset = 0;
  SQLRETURN ret = SQLSetDescField(ard, 0, SQL_DESC_BIND_OFFSET_PTR, &offset, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLPOINTER ptr = nullptr;
  ret = SQLGetDescField(ard, 0, SQL_DESC_BIND_OFFSET_PTR, &ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ptr == &offset);
}

// ============================================================================
// SQLSetDescField - OCTET_LENGTH_PTR on ARD
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set OCTET_LENGTH_PTR on ARD",
                 "[odbc-api][setdescfield][descriptor]") {
  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_CHAR), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN olen = 0;
  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH_PTR, &olen, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLPOINTER ptr = nullptr;
  ret = SQLGetDescField(ard, 1, SQL_DESC_OCTET_LENGTH_PTR, &ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ptr == &olen);
}

// ============================================================================
// SQLSetDescField - BIND_TYPE Header Field
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Set BIND_TYPE on ARD",
                 "[odbc-api][setdescfield][descriptor]") {
  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetDescField(ard, 0, SQL_DESC_BIND_TYPE, reinterpret_cast<SQLPOINTER>(64), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN bind_type = 0;
  ret = SQLGetDescField(ard, 0, SQL_DESC_BIND_TYPE, &bind_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(bind_type == 64);
}

// ============================================================================
// SQLSetDescField - DESC_COUNT Increasing Behavior
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: Increasing DESC_COUNT allocates new records",
                 "[odbc-api][setdescfield][descriptor]") {
  HandleWrapper explicit_desc_guard(dbc_handle(), SQL_HANDLE_DESC);
  const SQLHDESC explicit_desc = explicit_desc_guard.getHandle();

  SQLRETURN ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(5), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 5);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(10), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 10);
}

// ============================================================================
// SQLSetDescField - ARRAY_STATUS_PTR on APD
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetDescField: ARRAY_STATUS_PTR on APD",
                 "[odbc-api][setdescfield][descriptor]") {
  const SQLHDESC apd = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);

  SQLUSMALLINT status_arr[5] = {};
  SQLRETURN ret = SQLSetDescField(apd, 0, SQL_DESC_ARRAY_STATUS_PTR, status_arr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLPOINTER ptr = nullptr;
  ret = SQLGetDescField(apd, 0, SQL_DESC_ARRAY_STATUS_PTR, &ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ptr == status_arr);
}
