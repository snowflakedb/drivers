#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

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
// SQLProcedures - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Result set has correct number of columns",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char procCat[256] = {};
  char procSchem[256] = {};
  char procName[256] = {};
  SQLSMALLINT procType = 0;

  SQLGetData(stmt_handle(), 1, SQL_C_CHAR, procCat, sizeof(procCat), nullptr);
  SQLGetData(stmt_handle(), 2, SQL_C_CHAR, procSchem, sizeof(procSchem), nullptr);
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, procName, sizeof(procName), nullptr);
  SQLGetData(stmt_handle(), 8, SQL_C_SSHORT, &procType, 0, nullptr);

  REQUIRE(std::string(procCat) == database_name());
  REQUIRE(std::string(procSchem) == schema_name());
  REQUIRE(std::string(procName) == readonly_db::BASIC_PROC);
  // SQL_PT_FUNCTION since it has RETURNS
  REQUIRE(procType == SQL_PT_FUNCTION);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Wildcard search finds procedure",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar("BASICPR%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char name[256] = {};
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name, sizeof(name), nullptr);
  REQUIRE(std::string(name) == readonly_db::BASIC_PROC);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Multiple VARCHAR-returning procs are all returned",
                 "[odbc-api][procedures][catalog][known-bug]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar("PROCMULTI%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    char name[256] = {};
    SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name, sizeof(name), nullptr);
    INFO("Row " << (rowCount + 1) << ": " << name);
    rowCount++;
  }
  // Both procedures should be returned
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: NUMBER-returning proc is returned alongside VARCHAR proc",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar("PROCDTYPE%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    char name[256] = {};
    SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name, sizeof(name), nullptr);
    INFO("Row " << (rowCount + 1) << ": " << name);
    rowCount++;
  }
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Multiple NUMBER-returning procs are all returned",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar("PROCNUM%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    char name[256] = {};
    SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name, sizeof(name), nullptr);
    INFO("Row " << (rowCount + 1) << ": " << name);
    rowCount++;
  }
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: Non-existent procedure returns empty result set",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
                 "[odbc-api][procedures][catalog]") {
  SKIP("Long-running: multiple catalog round-trips cause timeout");
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
                 "[odbc-api][procedures][catalog]") {
  SKIP("Long-running: multiple catalog round-trips cause timeout");
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("PROC"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: HY090 - Negative SchemaName length",
                 "[odbc-api][procedures][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedures(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("PROC"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: HY090 - Negative ProcName length",
                 "[odbc-api][procedures][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedures(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("PROC"), -999);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLProcedures: 24000 - Cursor already open",
                 "[odbc-api][procedures][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLProcedures(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                      sqlchar(readonly_db::BASIC_PROC), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLProcedures: Requires active connection", "[odbc-api][procedures][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
