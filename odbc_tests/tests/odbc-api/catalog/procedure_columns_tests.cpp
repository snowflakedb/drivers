#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

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

// ============================================================================
// SQLProcedureColumns - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Result set has correct number of columns",
                 "[odbc-api][procedurecolumns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SQLGetData(stmt_handle(), 1, SQL_C_CHAR, procCat, sizeof(procCat), nullptr);
  SQLGetData(stmt_handle(), 2, SQL_C_CHAR, procSchem, sizeof(procSchem), nullptr);
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, procName, sizeof(procName), nullptr);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr);
  REQUIRE(std::string(procCat) == database_name());
  REQUIRE(std::string(procSchem) == schema_name());
  REQUIRE(std::string(procName) == readonly_db::MULTI_PARAM_PROC);
  REQUIRE(std::string(colName).empty());

  // Input parameter PNAME
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr);
  REQUIRE(std::string(colName) == "PNAME");

  // Input parameter PAGE
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr);
  REQUIRE(std::string(colName) == "PAGE");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Non-existent procedure returns empty result set",
                 "[odbc-api][procedurecolumns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar("NONEXISTENTPROCXYZ99999"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Specific ColumnName filters results",
                 "[odbc-api][procedurecolumns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::PROC_FILTER), SQL_NTS, sqlchar("PNAME"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char colName[256] = {};
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr);
  REQUIRE(std::string(colName) == "PNAME");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLProcedureColumns - Parameter Variations
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: Various parameter combinations are accepted",
                 "[odbc-api][procedurecolumns][catalog]") {
  SKIP("Long-running: multiple catalog round-trips cause timeout");
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
                 "[odbc-api][procedurecolumns][catalog]") {
  SKIP("Long-running: multiple catalog round-trips cause timeout");
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret =
      SQLProcedureColumns(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("PROC"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedureColumns: HY090 - Negative SchemaName length",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret =
      SQLProcedureColumns(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("PROC"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedureColumns: HY090 - Negative ProcName length",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedureColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("PROC"), -999, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedureColumns: HY090 - Negative ColumnName length",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret =
      SQLProcedureColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("PROC"), SQL_NTS, sqlchar("COL"), -999);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedureColumns: 24000 - Cursor already open",
                 "[odbc-api][procedurecolumns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
