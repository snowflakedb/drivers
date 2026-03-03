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
// SQLGetDescField - Header Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: Implicit descriptor has ALLOC_AUTO",
                 "[odbc-api][getdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT alloc_type = -1;
  SQLRETURN ret = SQLGetDescField(ard, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_AUTO);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: Explicit descriptor has ALLOC_USER",
                 "[odbc-api][getdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_USER);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: DESC_COUNT reflects bound columns",
                 "[odbc-api][getdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT count = -1;
  SQLRETURN ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 0);

  SQLINTEGER col_val = 0;
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: IRD fields available after prepare",
                 "[odbc-api][getdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42 AS PREP_COL"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(ird, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ird, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_DECIMAL);
}

// ============================================================================
// SQLGetDescField - ARD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: ARD record fields after binding",
                 "[odbc-api][getdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER col_val = 0;
  SQLLEN ind = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_SLONG);

  SQLPOINTER data_ptr = nullptr;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &data_ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(data_ptr == &col_val);
}

// ============================================================================
// SQLGetDescField - IRD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: IRD fields after execution",
                 "[odbc-api][getdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42 AS MY_COL"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(ird, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ird, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_DECIMAL);

  char name[128] = {};
  SQLINTEGER name_len = 0;
  ret = SQLGetDescField(ird, 1, SQL_DESC_NAME, name, sizeof(name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(name) == "MY_COL");
  REQUIRE(name_len == 6);

  SQLSMALLINT nullable = -1;
  ret = SQLGetDescField(ird, 1, SQL_DESC_NULLABLE, &nullable, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nullable == SQL_NO_NULLS);
}

// ============================================================================
// SQLGetDescField - APD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: APD fields after parameter binding",
                 "[odbc-api][getdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 55;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC apd = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(apd, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_SLONG);
}

// ============================================================================
// SQLGetDescField - SQL_NO_DATA
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: SQL_NO_DATA for RecNumber beyond count",
                 "[odbc-api][getdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ird, 99, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLGetDescField - String Truncation
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: 01004 - String truncation on small buffer",
                 "[odbc-api][getdescfield][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42 AS MY_COL"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char tiny[3] = {};
  SQLINTEGER full_len = 0;
  ret = SQLGetDescField(ird, 1, SQL_DESC_NAME, tiny, sizeof(tiny), &full_len);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(std::string(tiny) == "MY");
  REQUIRE(full_len == 6);
}

// ============================================================================
// SQLGetDescField - Error Cases
// ============================================================================

TEST_CASE("SQLGetDescField: SQL_INVALID_HANDLE for null descriptor", "[odbc-api][getdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLSMALLINT val = -1;
  const SQLRETURN ret = SQLGetDescField(SQL_NULL_HDESC, 0, SQL_DESC_COUNT, &val, 0, nullptr);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY091 - Invalid field identifier",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT val = -1;
  SQLRETURN ret = SQLGetDescField(ard, 0, 9999, &val, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY091", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: 07009 - Negative RecNumber",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT val = -1;
  SQLRETURN ret = SQLGetDescField(ard, -1, SQL_DESC_TYPE, &val, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY007 - IRD record from unprepared statement",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLSMALLINT type = -1;
  SQLRETURN ret = SQLGetDescField(ird, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY007", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY007 - IRD header from unprepared statement",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: The reference driver returns HY007 for any access to an IRD whose
  // associated statement has not been prepared or executed, including header
  // fields. The ODBC spec only explicitly lists HY007 for record fields.
  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLSMALLINT count = -1;
  SQLRETURN ret = SQLGetDescField(ird, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY007", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY007 - IRD after cursor closed",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ird, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  // Note: The ODBC spec says SQL_NO_DATA for IRD with no open cursor in
  // prepared/executed state. The reference driver returns HY007 instead.
  REQUIRE_EXPECTED_ERROR(ret, "HY007", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: 07009 - RecNumber 0 on IPD for record field",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ipd, 0, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY090 - Negative BufferLength",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 AS X"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char buf[32] = {};
  SQLINTEGER slen = 0;
  ret = SQLGetDescField(ird, 1, SQL_DESC_NAME, buf, -1, &slen);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY091 - Undefined field for ARD",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: The ODBC spec says getting a field undefined for a descriptor type
  // returns SQL_SUCCESS with undefined value. The reference driver returns HY091 instead.
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  char name[64] = {};
  SQLINTEGER name_len = 0;
  SQLRETURN ret = SQLGetDescField(ard, 1, SQL_DESC_NAME, name, sizeof(name), &name_len);
  REQUIRE_EXPECTED_ERROR(ret, "HY091", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][getdescfield][descriptor][error]") {
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

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", ard, SQL_HANDLE_DESC);

  SQLCancel(stmt_handle());
}
