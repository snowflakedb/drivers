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
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT alloc_type = -1;
  SQLRETURN ret = SQLGetDescField(ard, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_AUTO);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: Explicit descriptor has ALLOC_USER",
                 "[odbc-api][getdescfield][descriptor]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_USER);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: DESC_COUNT reflects bound columns",
                 "[odbc-api][getdescfield][descriptor]") {
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: IRD remaining fields across types",
                 "[odbc-api][getdescfield][descriptor]") {
  WINDOWS_ONLY { SKIP("SNOW-3720962: Test hangs on Windows — investigating driver-level deadlock"); }
  SQLRETURN ret = SQLExecDirect(stmt_handle(),
                                sqlchar("SELECT 'hello'::VARCHAR(50) AS STR_COL, "
                                        "42::NUMBER(10,2) AS NUM_COL"),
                                SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  // --- VARCHAR column (rec 1) ---

  {
    SQLINTEGER case_sensitive = -1;
    ret = SQLGetDescField(ird, 1, SQL_DESC_CASE_SENSITIVE, &case_sensitive, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(case_sensitive == SQL_TRUE);

    SQLSMALLINT searchable = -1;
    ret = SQLGetDescField(ird, 1, SQL_DESC_SEARCHABLE, &searchable, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(searchable == SQL_SEARCHABLE);

    SQLINTEGER num_prec_radix = -1;
    ret = SQLGetDescField(ird, 1, SQL_DESC_NUM_PREC_RADIX, &num_prec_radix, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(num_prec_radix == 0);

    SQLLEN display_size = -1;
    ret = SQLGetDescField(ird, 1, SQL_DESC_DISPLAY_SIZE, &display_size, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(display_size == 50);

    char label[128] = {};
    SQLINTEGER label_len = 0;
    ret = SQLGetDescField(ird, 1, SQL_DESC_LABEL, label, sizeof(label), &label_len);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DESC, ird), OdbcMatchers::Succeeded());
    REQUIRE(std::string(label) == "STR_COL");

    char base_col[128] = {};
    SQLINTEGER base_col_len = 0;
    ret = SQLGetDescField(ird, 1, SQL_DESC_BASE_COLUMN_NAME, base_col, sizeof(base_col), &base_col_len);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(base_col) == "STR_COL");

    char table_name[128] = {};
    SQLINTEGER table_name_len = 0;
    ret = SQLGetDescField(ird, 1, SQL_DESC_TABLE_NAME, table_name, sizeof(table_name), &table_name_len);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(table_name).empty());

    char lit_prefix[32] = {};
    SQLINTEGER lit_prefix_len = 0;
    ret = SQLGetDescField(ird, 1, SQL_DESC_LITERAL_PREFIX, lit_prefix, sizeof(lit_prefix), &lit_prefix_len);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(lit_prefix) == "'");

    char lit_suffix[32] = {};
    SQLINTEGER lit_suffix_len = 0;
    ret = SQLGetDescField(ird, 1, SQL_DESC_LITERAL_SUFFIX, lit_suffix, sizeof(lit_suffix), &lit_suffix_len);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(lit_suffix) == "'");
  }

  // --- NUMBER column (rec 2) ---
  {
    SQLSMALLINT case_sensitive = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_CASE_SENSITIVE, &case_sensitive, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(case_sensitive == SQL_FALSE);

    SQLSMALLINT searchable2 = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_SEARCHABLE, &searchable2, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(searchable2 == SQL_PRED_BASIC);

    SQLINTEGER num_prec_radix = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_NUM_PREC_RADIX, &num_prec_radix, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(num_prec_radix == 10);

    SQLSMALLINT unsigned_attr = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_UNSIGNED, &unsigned_attr, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(unsigned_attr == SQL_FALSE);

    SQLSMALLINT unnamed = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_UNNAMED, &unnamed, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(unnamed == SQL_NAMED);

    SQLSMALLINT updatable = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_UPDATABLE, &updatable, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(updatable == SQL_ATTR_READWRITE_UNKNOWN);

    SQLINTEGER auto_unique = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_AUTO_UNIQUE_VALUE, &auto_unique, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(auto_unique == SQL_FALSE);

    SQLSMALLINT fixed_prec = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_FIXED_PREC_SCALE, &fixed_prec, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(fixed_prec == SQL_FALSE);

    SQLSMALLINT precision = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_PRECISION, &precision, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(precision == 10);

    SQLSMALLINT scale = -1;
    ret = SQLGetDescField(ird, 2, SQL_DESC_SCALE, &scale, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(scale == 2);

    char type_name[128] = {};
    SQLINTEGER type_name_len = 0;
    ret = SQLGetDescField(ird, 2, SQL_DESC_TYPE_NAME, type_name, sizeof(type_name), &type_name_len);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(type_name) == "DECIMAL");

    char num_lit_prefix[32] = {};
    SQLINTEGER num_lit_prefix_len = 0;
    ret = SQLGetDescField(ird, 2, SQL_DESC_LITERAL_PREFIX, num_lit_prefix, sizeof(num_lit_prefix), &num_lit_prefix_len);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(num_lit_prefix).empty());
  }
}

// ============================================================================
// SQLGetDescField - APD Record Fields
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: APD fields after parameter binding",
                 "[odbc-api][getdescfield][descriptor]") {
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
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42 AS MY_COL"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char tiny[3] = {};
  SQLINTEGER full_len = 0;
  ret = SQLGetDescField(ird, 1, SQL_DESC_NAME, tiny, sizeof(tiny), &full_len);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(std::string(tiny) == "MY");
  WINDOWS_ONLY {
    // Windows DM reports the Unicode byte length (6 chars * 2 bytes = 12)
    REQUIRE(full_len == 12);
  }
  UNIX_ONLY {
    IODBC_ONLY {
      // iODBC reports the *truncated* StringLength (matches the bytes
      // actually written) following the ODBC 2.x convention.
      REQUIRE(full_len == 2);
    }
    else {
      REQUIRE(full_len == 6);
    }
  }
}

// ============================================================================
// SQLGetDescField - Error Cases
// ============================================================================

TEST_CASE("SQLGetDescField: SQL_INVALID_HANDLE for null descriptor", "[odbc-api][getdescfield][descriptor][error]") {
  SQLSMALLINT val = -1;
  const SQLRETURN ret = SQLGetDescField(SQL_NULL_HDESC, 0, SQL_DESC_COUNT, &val, 0, nullptr);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY091 - Invalid field identifier",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT val = -1;
  SQLRETURN ret = SQLGetDescField(ard, 0, 9999, &val, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY091", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: 07009 - Negative RecNumber",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLSMALLINT val = -1;
  SQLRETURN ret = SQLGetDescField(ard, -1, SQL_DESC_TYPE, &val, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ard, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY007 - IRD record from unprepared statement",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLSMALLINT type = -1;
  SQLRETURN ret = SQLGetDescField(ird, 1, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY007", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY007 - IRD header from unprepared statement",
                 "[odbc-api][getdescfield][descriptor][error]") {
  // The ODBC spec only lists HY007 for record fields, but both the reference
  // driver and unixODBC DM return HY007 for header fields too.  iODBC and the
  // Windows DM do not intercept header access — the new driver returns
  // SQL_SUCCESS with count=0.
  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLSMALLINT count = -1;
  SQLRETURN ret = SQLGetDescField(ird, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY007", ird, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY007 - IRD after cursor closed",
                 "[odbc-api][getdescfield][descriptor][error]") {
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
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ipd = get_descriptor(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ipd, 0, SQL_DESC_TYPE, &type, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "07009", ipd, SQL_HANDLE_DESC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY090 - Negative BufferLength",
                 "[odbc-api][getdescfield][descriptor][error]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 AS X"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  char buf[32] = {};
  SQLINTEGER slen = 0;
  ret = SQLGetDescField(ird, 1, SQL_DESC_NAME, buf, -1, &slen);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DESC, ird),
               OdbcMatchers::IsError() && (OdbcMatchers::HasSqlState("HY090") || OdbcMatchers::HasSqlState("HY000")));
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY091 - Undefined field for ARD",
                 "[odbc-api][getdescfield][descriptor][error]") {
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

  // When SQLGetDescField is called on the ARD while the parent statement is in
  // SQL_NEED_DATA
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);

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
  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDescField: HY010 - IRD access during SQL_NEED_DATA",
                 "[odbc-api][getdescfield][descriptor][error]") {
  // Given the implicit IRD of a statement driven into SQL_NEED_DATA via a
  // SQL_DATA_AT_EXEC bind
  const SQLHDESC ird = get_descriptor(stmt_handle(), SQL_ATTR_IMP_ROW_DESC);

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  // When SQLGetDescField is called on the IRD while the parent statement is in
  // SQL_NEED_DATA
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(ird, 0, SQL_DESC_COUNT, &count, 0, nullptr);

  OLD_IODBC_ONLY("BD#69") {
    // Then driver bypasses the SQL_NEED_DATA state-check for the
    //   descriptor entry point and silently returns SQL_SUCCESS.
    REQUIRE(ret == SQL_SUCCESS);
  }
  else {
    // Then DM surface HY010
    REQUIRE_EXPECTED_ERROR(ret, "HY010", ird, SQL_HANDLE_DESC);
  }

  // And the statement is cancelled to release any pending state
  SQLCancel(stmt_handle());
}
