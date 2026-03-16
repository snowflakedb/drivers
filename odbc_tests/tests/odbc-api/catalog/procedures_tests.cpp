#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "query_helpers.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLProcedures - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: Result set has correct number of columns",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(
      stmt_handle(),
      sqlchar("CREATE PROCEDURE test_procs_numcols(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_NUMCOLS"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // ODBC 3.x spec defines 8 columns
  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 8);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: Result set column names match ODBC 3.x spec",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(
      stmt_handle(),
      sqlchar(
          "CREATE PROCEDURE test_procs_colnames(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_COLNAMES"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: Returns known procedure with correct metadata",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(
      stmt_handle(),
      sqlchar("CREATE PROCEDURE test_procs_meta(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_META"), SQL_NTS);
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

  REQUIRE(std::string(procCat) == currentDb);
  REQUIRE(std::string(procSchem) == schema.name());
  REQUIRE(std::string(procName) == "TEST_PROCS_META");
  // SQL_PT_FUNCTION since it has RETURNS
  REQUIRE(procType == SQL_PT_FUNCTION);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: Wildcard search finds procedure",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(
      stmt_handle(),
      sqlchar(
          "CREATE PROCEDURE test_procs_wildcard(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_WILD%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char name[256] = {};
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name, sizeof(name), nullptr);
  REQUIRE(std::string(name) == "TEST_PROCS_WILDCARD");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: Multiple VARCHAR-returning procs are all returned",
                 "[odbc-api][procedures][catalog][known-bug]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(),
                                sqlchar("CREATE PROCEDURE test_procs_multi_a(p1 VARCHAR) RETURNS VARCHAR "
                                        "LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
                                SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(stmt_handle(),
                      sqlchar("CREATE PROCEDURE test_procs_multi_b(p1 VARCHAR) RETURNS VARCHAR "
                              "LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
                      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_MULTI%"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: NUMBER-returning proc is returned alongside VARCHAR proc",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(),
                                sqlchar("CREATE PROCEDURE test_procs_dtype_a(p1 VARCHAR) RETURNS VARCHAR "
                                        "LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
                                SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(stmt_handle(),
                      sqlchar("CREATE PROCEDURE test_procs_dtype_b(p1 INT) RETURNS INT "
                              "LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
                      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_DTYPE%"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: Multiple NUMBER-returning procs are all returned",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(),
                                sqlchar("CREATE PROCEDURE test_procs_num_a(p1 INT) RETURNS INT "
                                        "LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
                                SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(stmt_handle(),
                      sqlchar("CREATE PROCEDURE test_procs_num_b(p1 INT) RETURNS INT "
                              "LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
                      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_NUM%"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: Non-existent procedure returns empty result set",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  const std::string currentDb = get_current_database(dbc_handle());

  SQLRETURN ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()),
                                SQL_NTS, sqlchar("NONEXISTENT_PROC_XYZ_99999"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLProcedures - Parameter Variations
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: Various parameter combinations are accepted",
                 "[odbc-api][procedures][catalog]") {
  SKIP("Long-running: multiple catalog round-trips cause timeout");
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(
      stmt_handle(),
      sqlchar("CREATE PROCEDURE test_procs_params(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());
  const std::string& schemaName = schema.name();

  // Explicit catalog, schema, proc with SQL_NTS
  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schemaName.c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_PARAMS"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Explicit string lengths instead of SQL_NTS
  const std::string proc = "TEST_PROCS_PARAMS";
  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), static_cast<SQLSMALLINT>(currentDb.length()),
                      sqlchar(schemaName.c_str()), static_cast<SQLSMALLINT>(schemaName.length()), sqlchar(proc.c_str()),
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: Can call multiple times on same statement after close cursor",
                 "[odbc-api][procedures][catalog]") {
  SKIP("Long-running: multiple catalog round-trips cause timeout");
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(
      stmt_handle(),
      sqlchar("CREATE PROCEDURE test_procs_reuse(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_REUSE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_REUSE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: SQLRowCount after catalog function call",
                 "[odbc-api][procedures][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(
      stmt_handle(),
      sqlchar(
          "CREATE PROCEDURE test_procs_rowcount(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_ROWCOUNT"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLProcedures: 24000 - Cursor already open",
                 "[odbc-api][procedures][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(
      stmt_handle(),
      sqlchar("CREATE PROCEDURE test_procs_cursor(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END'"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_CURSOR"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLProcedures(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                      sqlchar("TEST_PROCS_CURSOR"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLProcedures: Requires active connection", "[odbc-api][procedures][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
