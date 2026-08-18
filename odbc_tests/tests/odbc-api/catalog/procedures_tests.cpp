#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <algorithm>
#include <cctype>
#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "ReadOnlyDbFixture.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

namespace {
std::string to_lower_copy(const std::string& s) {
  std::string out = s;
  std::transform(out.begin(), out.end(), out.begin(), [](unsigned char c) { return std::tolower(c); });
  return out;
}
}  // namespace

// ============================================================================
// SQLProcedures - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Result set has correct number of columns",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // ODBC 3.x spec defines 8 columns
  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 8);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Result set column names match ODBC 3.x spec",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const char* expectedColNames[] = {"PROCEDURE_CAT",     "PROCEDURE_SCHEM", "PROCEDURE_NAME", "NUM_INPUT_PARAMS",
                                    "NUM_OUTPUT_PARAMS", "NUM_RESULT_SETS", "REMARKS",        "PROCEDURE_TYPE"};

  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);

  for (SQLSMALLINT col = 1; col <= static_cast<SQLSMALLINT>(std::size(expectedColNames)); col++) {
    char colName[256] = {};
    SQLSMALLINT nameLen = 0;
    SQLSMALLINT dataType = 0;
    SQLULEN colSize = 0;
    SQLSMALLINT decDigits = 0;
    SQLSMALLINT nullable = 0;

    ret = SQLDescribeCol(stmt_handle(), col, reinterpret_cast<SQLCHAR*>(colName), sizeof(colName), &nameLen, &dataType,
                         &colSize, &decDigits, &nullable);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(colName) == expectedColNames[col - 1]);
  }
}

// ============================================================================
// SQLProcedures - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Returns known procedure with correct metadata",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char procCat[256] = {};
  char procSchem[256] = {};
  char procName[256] = {};
  SQLSMALLINT procType = 0;
  SQLLEN procCatInd = 0;
  SQLLEN procSchemInd = 0;
  SQLLEN procNameInd = 0;
  SQLLEN procTypeInd = 0;

  REQUIRE(SQLGetData(stmt_handle(), 1, SQL_C_CHAR, procCat, sizeof(procCat), &procCatInd) == SQL_SUCCESS);
  REQUIRE(procCatInd != SQL_NULL_DATA);
  REQUIRE(SQLGetData(stmt_handle(), 2, SQL_C_CHAR, procSchem, sizeof(procSchem), &procSchemInd) == SQL_SUCCESS);
  REQUIRE(procSchemInd != SQL_NULL_DATA);
  REQUIRE(SQLGetData(stmt_handle(), 3, SQL_C_CHAR, procName, sizeof(procName), &procNameInd) == SQL_SUCCESS);
  REQUIRE(procNameInd != SQL_NULL_DATA);
  REQUIRE(SQLGetData(stmt_handle(), 8, SQL_C_SSHORT, &procType, 0, &procTypeInd) == SQL_SUCCESS);
  REQUIRE(procTypeInd != SQL_NULL_DATA);

  REQUIRE(std::string(procCat) == database_name());
  REQUIRE(std::string(procSchem) == schema_name());
  REQUIRE(std::string(procName) == readonly_db::BASIC_PROC);
  // SQL_PT_FUNCTION since it has RETURNS
  REQUIRE(procType == SQL_PT_FUNCTION);

  // NUM_OUTPUT_PARAMS (col 5) is reserved and always NULL.
  SQLINTEGER numOutputParams = 0;
  SQLLEN outputInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 5, SQL_C_SLONG, &numOutputParams, 0, &outputInd) == SQL_SUCCESS);
  REQUIRE(outputInd == SQL_NULL_DATA);

  // NUM_RESULT_SETS (col 6) is 0 for a scalar (non-table-valued) procedure.
  SQLINTEGER numResultSets = -1;
  SQLLEN numResultSetsInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 6, SQL_C_SLONG, &numResultSets, 0, &numResultSetsInd) == SQL_SUCCESS);
  REQUIRE(numResultSetsInd != SQL_NULL_DATA);
  REQUIRE(numResultSets == 0);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: NUM_RESULT_SETS is 1 for a table-valued procedure",
                 "[odbc-api][procedures][catalog]") {
  // TABLE_PROC(pid INTEGER) RETURNS TABLE(id, name): a table-valued return sets
  // NUM_RESULT_SETS (col 6) to 1, in contrast to the scalar case above.
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::TABLE_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char procName[256] = {};
  SQLLEN procNameInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 3, SQL_C_CHAR, procName, sizeof(procName), &procNameInd) == SQL_SUCCESS);
  REQUIRE(procNameInd != SQL_NULL_DATA);
  REQUIRE(std::string(procName) == readonly_db::TABLE_PROC);

  SQLINTEGER numResultSets = -1;
  SQLLEN numResultSetsInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 6, SQL_C_SLONG, &numResultSets, 0, &numResultSetsInd) == SQL_SUCCESS);
  REQUIRE(numResultSetsInd != SQL_NULL_DATA);
  REQUIRE(numResultSets == 1);

  // A table-valued procedure still has a RETURNS clause, so PROCEDURE_TYPE stays
  // SQL_PT_FUNCTION.
  SQLSMALLINT procType = 0;
  SQLLEN procTypeInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 8, SQL_C_SSHORT, &procType, 0, &procTypeInd) == SQL_SUCCESS);
  REQUIRE(procTypeInd != SQL_NULL_DATA);
  REQUIRE(procType == SQL_PT_FUNCTION);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// NUM_INPUT_PARAMS (col 4) reports the parameter count parsed from
// argument_signature. Both the new driver and the reference driver populate this
// column with the argument count (reference: getNumArguments()). Each proc is
// checked in its own single-round-trip test to avoid the multiple-catalog-call
// timeout that keeps the "multiple times on same statement" cases skipped.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: NUM_INPUT_PARAMS reports single-param count",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(SQLFetch(stmt_handle()) == SQL_SUCCESS);

  // BASICPROC(p1 VARCHAR) has one input parameter.
  SQLINTEGER numInputParams = -1;
  SQLLEN numInputParamsInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_SLONG, &numInputParams, 0, &numInputParamsInd) == SQL_SUCCESS);
  REQUIRE(numInputParamsInd != SQL_NULL_DATA);
  REQUIRE(numInputParams == 1);
  REQUIRE(SQLFetch(stmt_handle()) == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: NUM_INPUT_PARAMS reports multi-param count",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::MULTI_PARAM_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(SQLFetch(stmt_handle()) == SQL_SUCCESS);

  // MULTIPARAMPROC(pname VARCHAR, page FLOAT) has two input parameters.
  SQLINTEGER numInputParams = -1;
  SQLLEN numInputParamsInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_SLONG, &numInputParams, 0, &numInputParamsInd) == SQL_SUCCESS);
  REQUIRE(numInputParamsInd != SQL_NULL_DATA);
  REQUIRE(numInputParams == 2);
  REQUIRE(SQLFetch(stmt_handle()) == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Wildcard search finds procedure",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar("BASICPR%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char name[256] = {};
  SQLLEN nameInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name, sizeof(name), &nameInd) == SQL_SUCCESS);
  REQUIRE(nameInd != SQL_NULL_DATA);
  REQUIRE(std::string(name) == readonly_db::BASIC_PROC);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Multiple VARCHAR-returning procs are all returned",
                 "[odbc-api][procedures][catalog][known-bug]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar("PROCMULTI%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    char name[256] = {};
    SQLLEN nameInd = 0;
    REQUIRE(SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name, sizeof(name), &nameInd) == SQL_SUCCESS);
    REQUIRE(nameInd != SQL_NULL_DATA);
    INFO("Row " << (rowCount + 1) << ": " << name);
    rowCount++;
  }
  // Both procedures should be returned
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: NUMBER-returning proc is returned alongside VARCHAR proc",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar("PROCDTYPE%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    char name[256] = {};
    SQLLEN nameInd = 0;
    REQUIRE(SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name, sizeof(name), &nameInd) == SQL_SUCCESS);
    REQUIRE(nameInd != SQL_NULL_DATA);
    INFO("Row " << (rowCount + 1) << ": " << name);
    rowCount++;
  }
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Multiple NUMBER-returning procs are all returned",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar("PROCNUM%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    char name[256] = {};
    SQLLEN nameInd = 0;
    REQUIRE(SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name, sizeof(name), &nameInd) == SQL_SUCCESS);
    REQUIRE(nameInd != SQL_NULL_DATA);
    INFO("Row " << (rowCount + 1) << ": " << name);
    rowCount++;
  }
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Non-existent procedure returns empty result set",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar("NONEXISTENTPROCXYZ99999"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLProcedures - Parameter Variations
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Various parameter combinations are accepted",
                 "[odbc-api][procedures][catalog][long_running]") {
  // Explicit catalog, schema, proc with SQL_NTS
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Explicit string lengths instead of SQL_NTS
  const std::string proc = readonly_db::BASIC_PROC;
  const std::string db = database_name();
  const std::string schema = schema_name();
  ret = SQLProcedures(stmt_handle(), sqlchar(db.c_str()), static_cast<SQLSMALLINT>(db.length()),
                      sqlchar(schema.c_str()), static_cast<SQLSMALLINT>(schema.length()), sqlchar(proc.c_str()),
                      static_cast<SQLSMALLINT>(proc.length()));
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

// ============================================================================
// SQLProcedures - Statement Reuse
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Can call multiple times on same statement after close cursor",
                 "[odbc-api][procedures][catalog][long_running]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: SQLRowCount after catalog function call",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLProcedures - Error Cases
// ============================================================================

TEST_CASE("SQLProcedures: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][procedures][catalog][error]") {
  const SQLRETURN ret = SQLProcedures(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, sqlchar("PROC"), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: HY090 - Negative CatalogName length",
                 "[odbc-api][procedures][catalog][error]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("PROC"), SQL_NTS);
  IODBC_ONLY {
    // iODBC's DM-side length validator rejects the negative length with the
    //   ODBC 2.x form of HY090 ("S1090") before the call reaches the driver.
    //   Exactly one record is posted on the SQL_HANDLE_STMT handle.
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: HY090 - Negative SchemaName length",
                 "[odbc-api][procedures][catalog][error]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("PROC"), SQL_NTS);
  IODBC_ONLY {
    // iODBC's DM-side length validator rejects the negative length with the
    //   ODBC 2.x form of HY090 ("S1090") before the call reaches the driver.
    //   Exactly one record is posted on the SQL_HANDLE_STMT handle.
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: HY090 - Negative ProcName length",
                 "[odbc-api][procedures][catalog][error]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("PROC"), -999);
  IODBC_ONLY {
    // iODBC's DM-side length validator rejects the negative length with the
    //   ODBC 2.x form of HY090 ("S1090") before the call reaches the driver.
    //   Exactly one record is posted on the SQL_HANDLE_STMT handle.
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: 24000 - Cursor already open",
                 "[odbc-api][procedures][catalog][error]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLProcedures: Requires active connection", "[odbc-api][procedures][catalog][error]") {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: metadata_id=TRUE with NULL CatalogName returns HY009",
                 "[odbc-api][procedures][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLProcedures(stmt_handle(), nullptr, 0, sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::BASIC_PROC),
                      SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: metadata_id=TRUE with NULL SchemaName returns HY009",
                 "[odbc-api][procedures][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, nullptr, 0, sqlchar(readonly_db::BASIC_PROC),
                      SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: metadata_id=TRUE with NULL ProcName returns HY009",
                 "[odbc-api][procedures][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

// ============================================================================
// SQLProcedures - SQL_ATTR_METADATA_ID identifier matching (BD#91)
// ============================================================================

// In identifier mode, unquoted identifiers are case-insensitive (folded to
// uppercase), so a lowercase catalog/schema/procedure name must still match the
// uppercase names Snowflake stores. The new driver folds unquoted identifiers
// (ODBC-spec compliant) so the row matches; the legacy driver filters
// information_schema case-sensitively and drops every row (BD#91).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLProcedures: metadata_id=TRUE matches unquoted identifiers case-insensitively",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  const std::string cat = to_lower_copy(database_name());
  const std::string sch = to_lower_copy(schema_name());
  const std::string proc = to_lower_copy(readonly_db::BASIC_PROC);

  ret = SQLProcedures(stmt_handle(), sqlchar(cat.c_str()), SQL_NTS, sqlchar(sch.c_str()), SQL_NTS,
                      sqlchar(proc.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  NEW_DRIVER_ONLY("BD#91") {
    REQUIRE(ret == SQL_SUCCESS);

    char procName[256] = {};
    SQLLEN procNameInd = 0;
    REQUIRE(SQLGetData(stmt_handle(), 3, SQL_C_CHAR, procName, sizeof(procName), &procNameInd) == SQL_SUCCESS);
    REQUIRE(procNameInd != SQL_NULL_DATA);
    REQUIRE(std::string(procName) == readonly_db::BASIC_PROC);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_NO_DATA);
  }
  OLD_DRIVER_ONLY("BD#91") { REQUIRE(ret == SQL_NO_DATA); }
}

// In pattern mode (default), the arguments are ordinary case-sensitive search
// values, so a lowercase procedure name must NOT match the uppercase stored
// name. Guards the identifier-mode fold from over-reaching into pattern mode.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: metadata_id=FALSE treats identifiers case-sensitively",
                 "[odbc-api][procedures][catalog]") {
  const std::string proc = to_lower_copy(readonly_db::BASIC_PROC);

  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(proc.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// SNOW-3899630 / BD#121: when the procedure has no comment, the new driver
// returns SQL_NULL_DATA for REMARKS (col 7). The legacy driver maps a null
// comment to a non-null empty string.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: REMARKS is SQL_NULL_DATA when procedure has no comment",
                 "[odbc-api][procedures][catalog]") {
  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char remarks[256];
  std::memset(remarks, 0xFF, sizeof(remarks));
  SQLLEN remarksInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 7, SQL_C_CHAR, remarks, sizeof(remarks), &remarksInd) == SQL_SUCCESS);

  NEW_DRIVER_ONLY("BD#121") { CHECK(remarksInd == SQL_NULL_DATA); }
  OLD_DRIVER_ONLY("BD#121") {
    CHECK(remarksInd != SQL_NULL_DATA);
    CHECK(std::string(remarks).empty());
  }

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}
