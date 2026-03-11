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
// SQLGetDescRec - IRD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: IRD record after execution",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42 AS MY_COL"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char name[128] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ird, 1, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(name) == "MY_COL");
  REQUIRE(name_len == 6);
  REQUIRE(type == SQL_DECIMAL);
  REQUIRE(sub_type == 0);
  REQUIRE(nullable == SQL_NO_NULLS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: IRD record after prepare",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1 AS PREP_COL"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char name[128] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ird, 1, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(name) == "PREP_COL");
  REQUIRE(name_len == 8);
  REQUIRE(type == SQL_DECIMAL);
}

// ============================================================================
// SQLGetDescRec - ARD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: ARD record after binding",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER col_val = 0;
  SQLLEN ind = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  constexpr char name[128] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ard, 1, sqlchar(name), sizeof(name), &name_len, &type, &sub_type, &length, &precision, &scale,
                      &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_SLONG);
}

// ============================================================================
// SQLGetDescRec - APD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: APD record after parameter binding",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 55;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC apd = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);

  char name[128] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(apd, 1, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_SLONG);
}

// ============================================================================
// SQLGetDescRec - NULL Output Pointers
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: All NULL output pointers",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  ret = SQLGetDescRec(ird, 1, nullptr, 0, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLGetDescRec - SQL_NO_DATA
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: SQL_NO_DATA for RecNumber beyond count",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  constexpr char name[32] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ird, 99, sqlchar(name), sizeof(name), &name_len, &type, &sub_type, &length, &precision, &scale,
                      &nullable);
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLGetDescRec - String Truncation
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: 01004 - Name truncation on small buffer",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42 AS MY_COL"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char tiny[3] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ird, 1, reinterpret_cast<SQLCHAR*>(tiny), sizeof(tiny), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(std::string(tiny) == "MY");
  REQUIRE(name_len == 6);
  REQUIRE(type == SQL_DECIMAL);
}

// ============================================================================
// SQLGetDescRec - Error Cases
// ============================================================================

TEST_CASE("SQLGetDescRec: SQL_INVALID_HANDLE for null descriptor", "[odbc-api][getdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  constexpr char name[32] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  const SQLRETURN ret = SQLGetDescRec(SQL_NULL_HDESC, 1, sqlchar(name), sizeof(name), &name_len, &type, &sub_type,
                                      &length, &precision, &scale, &nullable);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: 07009 - RecNumber 0 (bookmark)",
                 "[odbc-api][getdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char name[32] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ird, 0, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: 07009 - Negative RecNumber",
                 "[odbc-api][getdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char name[32] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ird, -1, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: HY007 - IRD from unprepared statement",
                 "[odbc-api][getdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char name[32] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  SQLRETURN ret = SQLGetDescRec(ird, 1, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type,
                                &length, &precision, &scale, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "HY007", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: HY007 - IRD after cursor closed",
                 "[odbc-api][getdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  char name[32] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ird, 1, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  // Note: The ODBC spec says SQL_NO_DATA for IRD with no open cursor in
  // prepared/executed state. The reference driver returns HY007 instead.
  REQUIRE_EXPECTED_ERROR(ret, "HY007", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][getdescrec][descriptor][error]") {
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

  char name[32] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ard, 1, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", ard, SQL_HANDLE_DESC);

  SQLCancel(stmt_handle());
}

// ============================================================================
// SQLGetDescRec - IPD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: IPD record after parameter binding",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 55;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  char name[128] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ipd, 1, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_INTEGER);
  REQUIRE(length == 4);
  REQUIRE(nullable == SQL_NULLABLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: 07009 - RecNumber 0 on IPD",
                 "[odbc-api][getdescrec][descriptor][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  char name[32] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ipd, 0, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type, &length,
                      &precision, &scale, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ipd, SQL_HANDLE_DESC);
}

// ============================================================================
// SQLGetDescRec - Explicit Descriptor
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: Explicit descriptor record",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 1, SQL_DESC_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_SLONG), 0);
  REQUIRE(ret == SQL_SUCCESS);

  char name[128] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(explicit_desc, 1, reinterpret_cast<SQLCHAR*>(name), sizeof(name), &name_len, &type, &sub_type,
                      &length, &precision, &scale, &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(type == SQL_C_SLONG);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: SQL_NO_DATA on empty explicit descriptor",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  constexpr char name[32] = {};
  SQLSMALLINT name_len = 0, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(explicit_desc, 1, sqlchar(name), sizeof(name), &name_len, &type, &sub_type, &length, &precision,
                      &scale, &nullable);
  REQUIRE(ret == SQL_NO_DATA);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

// ============================================================================
// SQLGetDescRec - Name NULL with StringLengthPtr
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescRec: Name NULL still returns StringLengthPtr",
                 "[odbc-api][getdescrec][descriptor]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42 AS MY_COL"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLSMALLINT name_len = -1, type = 0, sub_type = 0, precision = 0, scale = 0, nullable = 0;
  SQLLEN length = 0;

  ret = SQLGetDescRec(ird, 1, nullptr, 0, &name_len, &type, &sub_type, &length, &precision, &scale, &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 6);
  REQUIRE(type == SQL_DECIMAL);
}
