#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <algorithm>
#include <cctype>
#include <cstring>
#include <map>
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
// SQLProcedureColumns - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Result set has correct number of columns",
                 "[odbc-api][procedurecolumns][catalog]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns 21 columns (ODBC 3.x spec defines 19, driver adds 2 extra)
  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 21);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Result set column names match ODBC 3.x spec",
                 "[odbc-api][procedurecolumns][catalog]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns 21 columns (19 spec + 2 driver-specific)
  const char* expectedColNames[] = {"PROCEDURE_CAT",     "PROCEDURE_SCHEM",  "PROCEDURE_NAME", "COLUMN_NAME",
                                    "COLUMN_TYPE",       "DATA_TYPE",        "TYPE_NAME",      "COLUMN_SIZE",
                                    "BUFFER_LENGTH",     "DECIMAL_DIGITS",   "NUM_PREC_RADIX", "NULLABLE",
                                    "REMARKS",           "COLUMN_DEF",       "SQL_DATA_TYPE",  "SQL_DATETIME_SUB",
                                    "CHAR_OCTET_LENGTH", "ORDINAL_POSITION", "IS_NULLABLE",    "IS RESULT SET COLUMN",
                                    "USER_DATA_TYPE"};

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
// SQLProcedureColumns - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Returns parameters for known procedure",
                 "[odbc-api][procedurecolumns][catalog]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::MULTI_PARAM_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char procCat[256] = {};
  char procSchem[256] = {};
  char procName[256] = {};
  char colName[256] = {};

  // Return value is listed first with empty column name
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(SQLGetData(stmt_handle(), 1, SQL_C_CHAR, procCat, sizeof(procCat), nullptr) == SQL_SUCCESS);
  REQUIRE(SQLGetData(stmt_handle(), 2, SQL_C_CHAR, procSchem, sizeof(procSchem), nullptr) == SQL_SUCCESS);
  REQUIRE(SQLGetData(stmt_handle(), 3, SQL_C_CHAR, procName, sizeof(procName), nullptr) == SQL_SUCCESS);
  REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr) == SQL_SUCCESS);
  REQUIRE(std::string(procCat) == database_name());
  REQUIRE(std::string(procSchem) == schema_name());
  REQUIRE(std::string(procName) == readonly_db::MULTI_PARAM_PROC);
  REQUIRE(std::string(colName).empty());

  // Input parameter PNAME
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr) == SQL_SUCCESS);
  REQUIRE(std::string(colName) == "PNAME");

  // Input parameter PAGE
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr) == SQL_SUCCESS);
  REQUIRE(std::string(colName) == "PAGE");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Non-existent procedure returns empty result set",
                 "[odbc-api][procedurecolumns][catalog]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar("NONEXISTENTPROCXYZ99999"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Specific ColumnName filters results",
                 "[odbc-api][procedurecolumns][catalog]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::PROC_FILTER), SQL_NTS, sqlchar("PNAME"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char colName[256] = {};
  REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr) == SQL_SUCCESS);
  REQUIRE(std::string(colName) == "PNAME");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLProcedureColumns: COLUMN_TYPE and ORDINAL_POSITION order the return value before params",
                 "[odbc-api][procedurecolumns][catalog]") {
  // MULTI_PARAM_PROC(pname VARCHAR, page FLOAT) RETURNS VARCHAR: the scalar
  // return value comes first (COLUMN_TYPE=SQL_RETURN_VALUE, ordinal 0), followed
  // by the input parameters in declaration order (SQL_PARAM_INPUT, ordinals 1..).
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::MULTI_PARAM_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT columnType = 0;
  SQLINTEGER ordinalPos = -1;
  SQLSMALLINT isResultSetCol = -1;
  SQLLEN ind = 0;

  // Row 1: return value.
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &columnType, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(columnType == SQL_RETURN_VALUE);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 18, SQL_C_SLONG, &ordinalPos, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLINTEGER));
  REQUIRE(ordinalPos == 0);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 20, SQL_C_SSHORT, &isResultSetCol, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(isResultSetCol == SQL_FALSE);

  // Row 2: first input parameter.
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &columnType, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(columnType == SQL_PARAM_INPUT);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 18, SQL_C_SLONG, &ordinalPos, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLINTEGER));
  REQUIRE(ordinalPos == 1);

  // Row 3: second input parameter.
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &columnType, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(columnType == SQL_PARAM_INPUT);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 18, SQL_C_SLONG, &ordinalPos, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLINTEGER));
  REQUIRE(ordinalPos == 2);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: NUM_PREC_RADIX is 10 for FLOAT params",
                 "[odbc-api][procedurecolumns][catalog]") {
  // MULTI_PARAM_PROC(pname VARCHAR, page FLOAT): page must report catalog radix
  // 10 (same REAL contract as SQLColumns), not ColAttribute binary radix 2.
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::MULTI_PARAM_PROC), SQL_NTS, sqlchar("PAGE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER radix = static_cast<SQLINTEGER>(0x7FFFFFFF);
  SQLLEN radixInd = 0;
  ret = SQLGetData(stmt_handle(), 11, SQL_C_SLONG, &radix, 0, &radixInd);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(radixInd == sizeof(SQLINTEGER));
  CHECK(radix == 10);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: BUFFER_LENGTH is precision+2 for NUMBER/DECIMAL",
                 "[odbc-api][procedurecolumns][catalog]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::NUMBER_BUF_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Snowflake information_schema.argument_signature omits NUMBER precision/scale
  // (returns "(NUM38 NUMBER, NUM18S6 NUMBER)"), so both params report the default
  // NUMBER(38,0) metadata. Only NUM38 is asserted; NUM18S6 would duplicate 38/40.
  const std::map<std::string, SQLINTEGER> expectColSize = {
      {"NUM38", 38},
  };
  const std::map<std::string, SQLINTEGER> expectBufLenNew = {
      {"NUM38", 40},
  };
  const std::map<std::string, SQLINTEGER> expectBufLenOld = {
      {"NUM38", 16},
  };

  std::map<std::string, std::pair<SQLINTEGER, SQLINTEGER>> actual;
  while (true) {
    ret = SQLFetch(stmt_handle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE(ret == SQL_SUCCESS);

    char colName[256] = {};
    SQLLEN colNameInd = SQL_NULL_DATA;
    ret = SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), &colNameInd);
    REQUIRE(ret == SQL_SUCCESS);
    if (colNameInd == SQL_NULL_DATA || expectColSize.count(colName) == 0) {
      continue;
    }

    SQLINTEGER colSize = static_cast<SQLINTEGER>(0x7FFFFFFF);
    SQLLEN colSizeInd = SQL_NULL_DATA;
    ret = SQLGetData(stmt_handle(), 8, SQL_C_SLONG, &colSize, 0, &colSizeInd);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(colSizeInd == sizeof(SQLINTEGER));

    SQLINTEGER bufLen = static_cast<SQLINTEGER>(0x7FFFFFFF);
    SQLLEN bufLenInd = SQL_NULL_DATA;
    ret = SQLGetData(stmt_handle(), 9, SQL_C_SLONG, &bufLen, 0, &bufLenInd);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(bufLenInd == sizeof(SQLINTEGER));

    actual.emplace(colName, std::make_pair(colSize, bufLen));
  }

  for (const auto& [column, wantSize] : expectColSize) {
    const auto it = actual.find(column);
    REQUIRE(it != actual.end());
    INFO("column " << column);
    CHECK(it->second.first == wantSize);
    NEW_DRIVER_ONLY("BD#122") { CHECK(it->second.second == expectBufLenNew.at(column)); }
    OLD_DRIVER_ONLY("BD#122") { CHECK(it->second.second == expectBufLenOld.at(column)); }
  }
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: DATA_TYPE reflects the VARCHAR return type",
                 "[odbc-api][procedurecolumns][catalog]") {
  // BASIC_PROC(p1 VARCHAR) RETURNS VARCHAR: the return value row (fetched first)
  // maps to SQL_VARCHAR and reports NULLABLE=SQL_NULLABLE / IS_NULLABLE="YES".
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  SQLLEN dataTypeInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 6, SQL_C_SSHORT, &dataType, 0, &dataTypeInd) == SQL_SUCCESS);
  REQUIRE(dataTypeInd == sizeof(SQLSMALLINT));
  REQUIRE(dataType == SQL_VARCHAR);

  SQLSMALLINT nullable = -1;
  SQLLEN nullableInd = 0;
  REQUIRE(SQLGetData(stmt_handle(), 12, SQL_C_SSHORT, &nullable, 0, &nullableInd) == SQL_SUCCESS);
  REQUIRE(nullableInd == sizeof(SQLSMALLINT));
  REQUIRE(nullable == SQL_NULLABLE);

  char isNullable[8] = {};
  REQUIRE(SQLGetData(stmt_handle(), 19, SQL_C_CHAR, isNullable, sizeof(isNullable), nullptr) == SQL_SUCCESS);
  REQUIRE(std::string(isNullable) == "YES");
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLProcedureColumns: table-valued return emits SQL_RESULT_COL rows before params",
                 "[odbc-api][procedurecolumns][catalog]") {
  // TABLE_PROC(pid INTEGER) RETURNS TABLE(id, name): the result-set columns come
  // first (COLUMN_TYPE=SQL_RESULT_COL, IS RESULT SET COLUMN=SQL_TRUE, ordinals
  // 1..), followed by the input parameter (SQL_PARAM_INPUT, IS RESULT SET
  // COLUMN=SQL_FALSE). A table-valued procedure has no SQL_RETURN_VALUE row.
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::TABLE_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char colName[256] = {};
  SQLSMALLINT columnType = 0;
  SQLSMALLINT dataType = 0;
  SQLINTEGER ordinalPos = -1;
  SQLSMALLINT isResultSetCol = -1;
  SQLLEN ind = 0;

  // Row 1: first result-set column (ID).
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr) == SQL_SUCCESS);
  REQUIRE(std::string(colName) == "ID");
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &columnType, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(columnType == SQL_RESULT_COL);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 18, SQL_C_SLONG, &ordinalPos, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLINTEGER));
  REQUIRE(ordinalPos == 1);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 20, SQL_C_SSHORT, &isResultSetCol, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(isResultSetCol == SQL_TRUE);

  // Row 2: second result-set column (NAME), a VARCHAR.
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr) == SQL_SUCCESS);
  REQUIRE(std::string(colName) == "NAME");
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &columnType, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(columnType == SQL_RESULT_COL);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 18, SQL_C_SLONG, &ordinalPos, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLINTEGER));
  REQUIRE(ordinalPos == 2);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 6, SQL_C_SSHORT, &dataType, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(dataType == SQL_VARCHAR);

  // Row 3: the input parameter (PID) follows the result columns.
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr) == SQL_SUCCESS);
  REQUIRE(std::string(colName) == "PID");
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &columnType, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(columnType == SQL_PARAM_INPUT);
  ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 20, SQL_C_SSHORT, &isResultSetCol, 0, &ind) == SQL_SUCCESS);
  REQUIRE(ind == sizeof(SQLSMALLINT));
  REQUIRE(isResultSetCol == SQL_FALSE);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLProcedureColumns - Parameter Variations
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Various parameter combinations are accepted",
                 "[odbc-api][procedurecolumns][catalog][long_running]") {
  // Return value + 1 input parameter = 2 rows
  // Explicit catalog, schema, proc with SQL_NTS
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 2);
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Explicit string lengths instead of SQL_NTS
  const std::string proc = readonly_db::BASIC_PROC;
  const std::string db = database_name();
  const std::string schema = schema_name();
  ret = SQLProcedureColumns(stmt_handle(), sqlchar(db.c_str()), static_cast<SQLSMALLINT>(db.length()),
                            sqlchar(schema.c_str()), static_cast<SQLSMALLINT>(schema.length()), sqlchar(proc.c_str()),
                            static_cast<SQLSMALLINT>(proc.length()), nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 2);
}

// ============================================================================
// SQLProcedureColumns - Statement Reuse
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLProcedureColumns: Can call multiple times on same statement after close cursor",
                 "[odbc-api][procedurecolumns][catalog][long_running]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  // Return value + 1 input parameter = 2 rows
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 2);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: SQLRowCount after catalog function call",
                 "[odbc-api][procedurecolumns][catalog]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLProcedureColumns - Error Cases
// ============================================================================

TEST_CASE("SQLProcedureColumns: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][procedurecolumns][catalog][error]") {
  const SQLRETURN ret =
      SQLProcedureColumns(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, sqlchar("PROC"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedureColumns: HY090 - Negative CatalogName length",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SQLRETURN ret =
      SQLProcedureColumns(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("PROC"), SQL_NTS, nullptr, 0);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedureColumns: HY090 - Negative SchemaName length",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SQLRETURN ret =
      SQLProcedureColumns(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("PROC"), SQL_NTS, nullptr, 0);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedureColumns: HY090 - Negative ProcName length",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("PROC"), -999, nullptr, 0);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedureColumns: HY090 - Negative ColumnName length",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SQLRETURN ret =
      SQLProcedureColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("PROC"), SQL_NTS, sqlchar("COL"), -999);
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

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: metadata_id=TRUE with NULL CatalogName returns HY009",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // ColumnName may be NULL in identifier mode; the error comes from CatalogName.
  ret = SQLProcedureColumns(stmt_handle(), nullptr, 0, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: metadata_id=TRUE with NULL SchemaName returns HY009",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, nullptr, 0,
                            sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: metadata_id=TRUE with NULL ProcName returns HY009",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS, nullptr,
                            0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: 24000 - Cursor already open",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_PROC), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLProcedureColumns: Requires active connection",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}

// ============================================================================
// SQLProcedureColumns - SQL_ATTR_METADATA_ID identifier matching (BD#91)
// ============================================================================

// In identifier mode, unquoted identifiers are case-insensitive (folded to
// uppercase), so lowercase catalog/schema/procedure/column names must still
// match the uppercase names Snowflake stores. The new driver folds unquoted
// identifiers (ODBC-spec compliant) so the PNAME row matches; the legacy driver
// compares case-sensitively and drops every row (BD#91).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLProcedureColumns: metadata_id=TRUE matches unquoted identifiers case-insensitively",
                 "[odbc-api][procedurecolumns][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  const std::string cat = to_lower_copy(database_name());
  const std::string sch = to_lower_copy(schema_name());
  const std::string proc = to_lower_copy(readonly_db::PROC_FILTER);
  const std::string col = to_lower_copy("PNAME");

  ret = SQLProcedureColumns(stmt_handle(), sqlchar(cat.c_str()), SQL_NTS, sqlchar(sch.c_str()), SQL_NTS,
                            sqlchar(proc.c_str()), SQL_NTS, sqlchar(col.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  NEW_DRIVER_ONLY("BD#91") {
    REQUIRE(ret == SQL_SUCCESS);

    char colName[256] = {};
    REQUIRE(SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr) == SQL_SUCCESS);
    REQUIRE(std::string(colName) == "PNAME");

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_NO_DATA);
  }
  OLD_DRIVER_ONLY("BD#91") { REQUIRE(ret == SQL_NO_DATA); }
}

// In pattern mode (default), the ColumnName argument is an ordinary
// case-sensitive search value, so a lowercase column name must NOT match the
// uppercase parsed column name. Both drivers are case-sensitive here, so this
// needs no NEW/OLD split; it guards the client-side like_match (which is
// intentionally case-sensitive) from over-reaching into pattern mode.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: metadata_id=FALSE treats ColumnName case-sensitively",
                 "[odbc-api][procedurecolumns][catalog]") {
  const std::string col = to_lower_copy("PNAME");

  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::PROC_FILTER), SQL_NTS, sqlchar(col.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}
