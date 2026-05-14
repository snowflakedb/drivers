#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SQL_ATTR_PARAMSET_SIZE (22)
// ============================================================================

TEST_CASE("SQL_ATTR_PARAMSET_SIZE default value is 1.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMSET_SIZE is queried on a fresh statement
  SQLULEN value = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, &value, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value 1
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 1);
}

TEST_CASE("SQL_ATTR_PARAMSET_SIZE can be set to 3 and retrieved.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMSET_SIZE is set to 3
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then get should return 3
  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 3);
}

TEST_CASE("SQL_ATTR_PARAMSET_SIZE can be reset to 1 and retrieved.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMSET_SIZE is set to 1
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)1, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then get should return 1
  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 1);
}

TEST_CASE("SQL_ATTR_PARAMSET_SIZE value 0 is coerced to 1 with SQL_SUCCESS_WITH_INFO.") {
#ifdef _WIN32
  // The Windows DM intercepts SQL_ATTR_PARAMSET_SIZE=0 and returns SQL_ERROR before
  // reaching the driver, so the driver coercion behaviour is not observable on Windows.
  SKIP("Windows DM intercepts PARAMSET_SIZE=0; coercion not observable through DM");
#endif
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMSET_SIZE is set to 0 (invalid per spec)
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)0, 0);

  // Then the new driver coerces value to 1 with SQL_SUCCESS_WITH_INFO; the old
  // driver returns SQL_SUCCESS and leaves the value as 0 (BD#58)
  NEW_DRIVER_ONLY("BD#58") {
    REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
    SQLULEN value = 0;
    ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, &value, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(value == 1);
  }
  OLD_DRIVER_ONLY("BD#58") { REQUIRE(ret == SQL_SUCCESS); }
}

// ============================================================================
// SQL_ATTR_PARAM_BIND_TYPE (18)
// ============================================================================

TEST_CASE("SQL_ATTR_PARAM_BIND_TYPE default value is SQL_PARAM_BIND_BY_COLUMN (0).") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_BIND_TYPE is queried on a fresh statement
  SQLULEN value = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, &value, 0, nullptr);

  // Then it should return SQL_SUCCESS and SQL_PARAM_BIND_BY_COLUMN (0)
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == SQL_PARAM_BIND_BY_COLUMN);
}

TEST_CASE("SQL_ATTR_PARAM_BIND_TYPE can be set and retrieved.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_BIND_TYPE is set to a row-wise struct size
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, (SQLPOINTER)64, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then get should return the value that was set
  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 64);
}

// ============================================================================
// SQL_ATTR_PARAM_BIND_OFFSET_PTR (17)
// ============================================================================

TEST_CASE("SQL_ATTR_PARAM_BIND_OFFSET_PTR default value is null.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_BIND_OFFSET_PTR is queried on a fresh statement
  SQLLEN* ptr = reinterpret_cast<SQLLEN*>(0xDEAD);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_OFFSET_PTR, &ptr, 0, nullptr);

  // Then it should return SQL_SUCCESS and a null pointer
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == nullptr);
}

TEST_CASE("SQL_ATTR_PARAM_BIND_OFFSET_PTR can be set and retrieved.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_BIND_OFFSET_PTR is set to a valid pointer
  SQLLEN offset = 0;
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_OFFSET_PTR, &offset, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then get should return the same pointer
  SQLLEN* ptr = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_OFFSET_PTR, &ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == &offset);
}

// ============================================================================
// SQL_ATTR_PARAM_STATUS_PTR (20)
// ============================================================================

TEST_CASE("SQL_ATTR_PARAM_STATUS_PTR default value is null.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_STATUS_PTR is queried on a fresh statement
  SQLUSMALLINT* ptr = reinterpret_cast<SQLUSMALLINT*>(0xDEAD);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, &ptr, 0, nullptr);

  // Then it should return SQL_SUCCESS and a null pointer
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == nullptr);
}

TEST_CASE("SQL_ATTR_PARAM_STATUS_PTR can be set and retrieved.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_STATUS_PTR is set to a valid pointer
  SQLUSMALLINT status[3] = {};
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, status, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then get should return the same pointer
  SQLUSMALLINT* ptr = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, &ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == status);
}

// ============================================================================
// SQL_ATTR_PARAMS_PROCESSED_PTR (21)
// ============================================================================

TEST_CASE("SQL_ATTR_PARAMS_PROCESSED_PTR default value is null.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMS_PROCESSED_PTR is queried on a fresh statement
  SQLULEN* ptr = reinterpret_cast<SQLULEN*>(0xDEAD);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &ptr, 0, nullptr);

  // Then it should return SQL_SUCCESS and a null pointer
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == nullptr);
}

TEST_CASE("SQL_ATTR_PARAMS_PROCESSED_PTR can be set and retrieved.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMS_PROCESSED_PTR is set to a valid pointer
  SQLULEN processed = SQLULEN(-1);
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then get should return the same pointer
  SQLULEN* ptr = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == &processed);
}

// ============================================================================
// SQL_ATTR_PARAM_OPERATION_PTR (19)
// ============================================================================

TEST_CASE("SQL_ATTR_PARAM_OPERATION_PTR default value is null.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_OPERATION_PTR is queried on a fresh statement
  SQLUSMALLINT* ptr = reinterpret_cast<SQLUSMALLINT*>(0xDEAD);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, &ptr, 0, nullptr);

  // Then it should return SQL_SUCCESS and a null pointer
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == nullptr);
}

TEST_CASE("SQL_ATTR_PARAM_OPERATION_PTR can be set and retrieved.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_OPERATION_PTR is set to a valid pointer
  SQLUSMALLINT ops[3] = {};
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, ops, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then get should return the same pointer
  SQLUSMALLINT* ptr = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, &ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == ops);
}

// ============================================================================
// Array parameter execution
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: PARAMS_PROCESSED_PTR is written after execution.",
                 "[query][param_array][execution]") {
  // Given a temp table and a prepared INSERT
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_1 (val INTEGER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_1 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And 3 integer values bound as a column-wise array
  SQLINTEGER values[3] = {10, 20, 30};
  SQLLEN indicators[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // When PARAMSET_SIZE = 3 and PARAMS_PROCESSED_PTR are set and SQLExecute is called
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);
  SQLULEN processed = SQLULEN(-1);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then PARAMS_PROCESSED_PTR should be 3
  CHECK(processed == 3);

  // And 3 rows should be in the table
  auto sel = conn.execute("SELECT COUNT(*) FROM param_array_exec_1");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 3);
}

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: PARAM_STATUS_PTR written SQL_PARAM_SUCCESS per set.",
                 "[query][param_array][execution]") {
  // Given a temp table and a prepared INSERT
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_2 (val INTEGER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_2 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And 3 integer values bound as a column-wise array
  SQLINTEGER values[3] = {1, 2, 3};
  SQLLEN indicators[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // When PARAMSET_SIZE = 3, PARAM_STATUS_PTR is set and SQLExecute is called
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);
  SQLUSMALLINT status[3] = {0xFF, 0xFF, 0xFF};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, status, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then each set should be SQL_PARAM_SUCCESS (0)
  CHECK(status[0] == SQL_PARAM_SUCCESS);
  CHECK(status[1] == SQL_PARAM_SUCCESS);
  CHECK(status[2] == SQL_PARAM_SUCCESS);
}

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAM_OPERATION_PTR SQL_PARAM_IGNORE skips that set.",
                 "[query][param_array][execution]") {
  // Given a temp table and a prepared INSERT
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_3 (val INTEGER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_3 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And 3 integer values bound as a column-wise array
  SQLINTEGER values[3] = {100, 200, 300};
  SQLLEN indicators[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // When PARAMSET_SIZE = 3, set 1 (middle) is marked SQL_PARAM_IGNORE
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);
  SQLUSMALLINT ops[3] = {SQL_PARAM_PROCEED, SQL_PARAM_IGNORE, SQL_PARAM_PROCEED};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, ops, 0);
  REQUIRE_ODBC(ret, stmt);
  SQLULEN processed = SQLULEN(-1);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then only 2 rows should be inserted (both drivers skip the ignored set)
  auto sel = conn.execute("SELECT COUNT(*) FROM param_array_exec_3");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 2);

  // PARAMS_PROCESSED_PTR: new driver counts only non-ignored sets (2);
  // old driver counts all sets including ignored ones (3) (BD#59)
  NEW_DRIVER_ONLY("BD#59") { CHECK(processed == 2); }
  OLD_DRIVER_ONLY("BD#59") { CHECK(processed == 3); }
}

// ============================================================================
// array_size == 1: PARAMS_PROCESSED_PTR and PARAM_STATUS_PTR still written (M1)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture,
                 "SQL_ATTR_PARAMSET_SIZE == 1: PARAMS_PROCESSED_PTR and PARAM_STATUS_PTR are written.",
                 "[query][param_array][execution]") {
  // Given a prepared INSERT with array_size left at its default of 1
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_single (val INTEGER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_single VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER val = 7;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &val, sizeof(SQLINTEGER),
                         &ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Set PARAMS_PROCESSED_PTR and PARAM_STATUS_PTR before executing
  SQLULEN processed = 0xDEAD;
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE_ODBC(ret, stmt);
  SQLUSMALLINT status[1] = {0xFF};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, status, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with the default array_size == 1
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then both pointers should be written: processed == 1, status[0] == SQL_PARAM_SUCCESS
  CHECK(processed == 1);
  CHECK(status[0] == SQL_PARAM_SUCCESS);
}

// ============================================================================
// SQL_PARAM_UNUSED written in status array for ignored set (T4)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_PARAM_IGNORE: status array slot written SQL_PARAM_UNUSED.",
                 "[query][param_array][execution]") {
  // Given a temp table, array of 3, middle set ignored
  conn.execute("CREATE TEMPORARY TABLE param_array_unused (val INTEGER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_unused VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER values[3] = {1, 2, 3};
  SQLLEN indicators[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  SQLUSMALLINT ops[3] = {SQL_PARAM_PROCEED, SQL_PARAM_IGNORE, SQL_PARAM_PROCEED};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, ops, 0);
  REQUIRE_ODBC(ret, stmt);

  SQLUSMALLINT status[3] = {0xFF, 0xFF, 0xFF};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, status, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with SQL_PARAM_IGNORE on the middle set
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the ignored slot should be SQL_PARAM_UNUSED (7), the others SQL_PARAM_SUCCESS (0)
  NEW_DRIVER_ONLY("BD#59") {
    CHECK(status[0] == SQL_PARAM_SUCCESS);
    CHECK(status[1] == SQL_PARAM_UNUSED);
    CHECK(status[2] == SQL_PARAM_SUCCESS);
  }
}

// ============================================================================
// Two bound parameters (T6)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: two bound parameters.",
                 "[query][param_array][execution]") {
  // Given a two-column table
  conn.execute("CREATE TEMPORARY TABLE param_array_two_params (a INTEGER, b INTEGER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_two_params VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Bind two column-wise arrays of 3 elements each
  SQLINTEGER col_a[3] = {1, 2, 3};
  SQLLEN ind_a[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, col_a,
                         sizeof(SQLINTEGER), ind_a);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  SQLINTEGER col_b[3] = {10, 20, 30};
  SQLLEN ind_b[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, col_b,
                         sizeof(SQLINTEGER), ind_b);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with PARAMSET_SIZE = 3 and two bound parameters
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then 3 rows should be present with correct (a, b) pairs
  auto sel = conn.execute("SELECT a, b FROM param_array_two_params ORDER BY a");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 1);
  CHECK(get_data<SQL_C_LONG>(sel, 2) == 10);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 2);
  CHECK(get_data<SQL_C_LONG>(sel, 2) == 20);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 3);
  CHECK(get_data<SQL_C_LONG>(sel, 2) == 30);
}

// ============================================================================
// VARCHAR / SQL_C_CHAR parameters in an array (T7)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: SQL_C_CHAR array inserts all sets.",
                 "[query][param_array][execution]") {
  // Given a varchar table and 3 string values
  conn.execute("CREATE TEMPORARY TABLE param_array_varchar (s VARCHAR(20))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_varchar VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Column-wise: 3 fixed-width 20-char buffers
  const SQLLEN BUF = 20;
  char values[3][20] = {"hello", "world", "odbc"};
  SQLLEN indicators[3] = {SQL_NTS, SQL_NTS, SQL_NTS};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 20, 0, values, BUF, indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with PARAMSET_SIZE = 3 and SQL_C_CHAR parameters
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then 3 rows should be present
  auto sel = conn.execute("SELECT COUNT(*) FROM param_array_varchar");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 3);
}

// ============================================================================
// SQL_NULL_DATA in one array slot (T8)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: SQL_NULL_DATA indicator inserts NULL.",
                 "[query][param_array][execution]") {
  // Given a nullable integer column
  conn.execute("CREATE TEMPORARY TABLE param_array_null (val INTEGER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_null VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER values[3] = {1, 0, 3};
  SQLLEN indicators[3] = {0, SQL_NULL_DATA, 0};  // middle slot is NULL
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with PARAMSET_SIZE = 3 and SQL_NULL_DATA in one indicator
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then 3 rows inserted, 1 of which is NULL
  auto sel = conn.execute("SELECT COUNT(*) FROM param_array_null");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 3);

  auto sel2 = conn.execute("SELECT COUNT(*) FROM param_array_null WHERE val IS NULL");
  ret = SQLFetch(sel2.getHandle());
  REQUIRE_ODBC(ret, sel2);
  CHECK(get_data<SQL_C_LONG>(sel2, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: SQLExecDirect inserts all sets.",
                 "[query][param_array][execution]") {
  // Given a temp table
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_4 (val INTEGER)");
  auto stmt = conn.createStatement();

  // And 3 integer values bound as a column-wise array
  SQLINTEGER values[3] = {11, 22, 33};
  SQLLEN indicators[3] = {0, 0, 0};
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                                   sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // When PARAMSET_SIZE = 3 and SQLExecDirect is called
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);
  SQLULEN processed = SQLULEN(-1);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_4 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then all 3 rows should be inserted and processed count written
  CHECK(processed == 3);
  auto sel = conn.execute("SELECT COUNT(*) FROM param_array_exec_4");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 3);
}

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: row-wise binding inserts all sets.",
                 "[query][param_array][execution]") {
  // Given a temp table
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_5 (val INTEGER)");
  auto stmt = conn.createStatement();

  // And 3 rows packed in a struct array (row-wise binding)
  struct Row {
    SQLINTEGER val;
    SQLLEN indicator;
  };
  Row rows[3] = {{42, 0}, {43, 0}, {44, 0}};

  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_5 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, (SQLPOINTER)sizeof(Row), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &rows[0].val,
                         sizeof(SQLINTEGER), &rows[0].indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // When PARAMSET_SIZE = 3 and SQLExecute is called
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then 3 rows should be in the table with values 42, 43, 44
  auto sel = conn.execute("SELECT val FROM param_array_exec_5 ORDER BY val");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 42);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 43);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 44);
}

// ============================================================================
// T1: Partial-success — some sets succeed, some fail → SQL_SUCCESS_WITH_INFO
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: partial failure returns SQL_SUCCESS_WITH_INFO.",
                 "[query][param_array][execution]") {
  // Given a table with a UNIQUE constraint so duplicate inserts fail
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_6 (val INTEGER UNIQUE)");

  // Pre-insert the value that will cause a duplicate on set 1 (0-based)
  conn.execute("INSERT INTO param_array_exec_6 VALUES (20)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_6 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Bind 3 values: set 0 = 10 (OK), set 1 = 20 (duplicate → error), set 2 = 30 (OK)
  SQLINTEGER values[3] = {10, 20, 30};
  SQLLEN indicators[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  SQLUSMALLINT status[3] = {0xFF, 0xFF, 0xFF};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, status, 0);
  REQUIRE_ODBC(ret, stmt);

  SQLULEN processed = SQLULEN(-1);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called
  ret = SQLExecute(stmt.getHandle());

  // Then the return code should be SQL_SUCCESS_WITH_INFO (partial failure)
  NEW_DRIVER_ONLY("partial-failure") {
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccessWithInfo());

    // And PARAMS_PROCESSED_PTR = 3 (all sets attempted)
    CHECK(processed == 3);

    // And status array reflects per-set outcome
    CHECK(status[0] == SQL_PARAM_SUCCESS);
    CHECK(status[1] == SQL_PARAM_ERROR);
    CHECK(status[2] == SQL_PARAM_SUCCESS);

    // And 2 new rows were inserted (10 and 30; 20 already existed)
    auto sel = conn.execute("SELECT COUNT(*) FROM param_array_exec_6");
    ret = SQLFetch(sel.getHandle());
    REQUIRE_ODBC(ret, sel);
    CHECK(get_data<SQL_C_LONG>(sel, 1) == 3);
  }
}

// ============================================================================
// T2: All-fail — every set errors → SQL_ERROR
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: all sets fail returns SQL_ERROR.",
                 "[query][param_array][execution]") {
  // Given a table with a NOT NULL constraint
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_7 (val INTEGER NOT NULL)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_7 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Bind 3 NULLs — all sets will fail the NOT NULL constraint
  SQLINTEGER values[3] = {0, 0, 0};
  SQLLEN indicators[3] = {SQL_NULL_DATA, SQL_NULL_DATA, SQL_NULL_DATA};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  SQLUSMALLINT status[3] = {0xFF, 0xFF, 0xFF};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, status, 0);
  REQUIRE_ODBC(ret, stmt);

  SQLULEN processed = SQLULEN(-1);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called
  ret = SQLExecute(stmt.getHandle());

  // Then the return code should be SQL_ERROR
  NEW_DRIVER_ONLY("all-fail") {
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError());

    // And every set should be SQL_PARAM_ERROR
    CHECK(status[0] == SQL_PARAM_ERROR);
    CHECK(status[1] == SQL_PARAM_ERROR);
    CHECK(status[2] == SQL_PARAM_ERROR);

    // And PARAMS_PROCESSED_PTR = 3 (all 3 sets were attempted)
    CHECK(processed == 3);

    // And no rows were inserted
    auto sel = conn.execute("SELECT COUNT(*) FROM param_array_exec_7");
    SQLRETURN fret = SQLFetch(sel.getHandle());
    REQUIRE_ODBC(fret, sel);
    CHECK(get_data<SQL_C_LONG>(sel, 1) == 0);
  }
}

// ============================================================================
// T3: SQLGetDiagField / SQL_DIAG_ROW_NUMBER for per-set errors
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: SQLGetDiagField returns row number for failed set.",
                 "[query][param_array][execution]") {
  // Given a table with a UNIQUE constraint
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_8 (val INTEGER UNIQUE)");
  conn.execute("INSERT INTO param_array_exec_8 VALUES (20)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_8 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Bind 3 values: set 1 (0-based) is a duplicate
  SQLINTEGER values[3] = {10, 20, 30};
  SQLLEN indicators[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called (partial failure)
  ret = SQLExecute(stmt.getHandle());

  NEW_DRIVER_ONLY("T3-diag-row-number") {
    // Then the return code should be SQL_SUCCESS_WITH_INFO (partial failure)
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccessWithInfo());

    // And at least one diagnostic record should be present
    auto diags = get_diag_rec(stmt);
    REQUIRE(!diags.empty());

    // And the row number field for the failing set should be 2 (1-based set_idx+1)
    SQLLEN row_number = 0;
    SQLSMALLINT str_len = 0;
    SQLRETURN diag_ret =
        SQLGetDiagField(SQL_HANDLE_STMT, stmt.getHandle(), 1, SQL_DIAG_ROW_NUMBER, &row_number, 0, &str_len);
    REQUIRE(diag_ret == SQL_SUCCESS);
    CHECK(row_number == 2);
  }
}

// ============================================================================
// T4: bind_offset_ptr end-to-end
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAM_BIND_OFFSET_PTR offsets parameter array reads.",
                 "[query][param_array][execution]") {
  // Given a temp table
  conn.execute("CREATE TEMPORARY TABLE param_array_exec_9 (val INTEGER)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_exec_9 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Allocate an array of 5 integers; bind parameter 1 to element 0.
  // Then set bind_offset to 2 * sizeof(SQLINTEGER) so the driver reads from
  // elements [2, 3, 4] instead of [0, 1, 2].
  SQLINTEGER values[5] = {10, 20, 30, 40, 50};
  SQLLEN indicators[5] = {0, 0, 0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Set bind offset to skip the first 2 elements
  SQLULEN bind_offset = 2 * sizeof(SQLINTEGER);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_OFFSET_PTR, &bind_offset, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with bind_offset applied
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the table should contain exactly 30, 40, 50 (not 10, 20, 30)
  auto sel = conn.execute("SELECT val FROM param_array_exec_9 ORDER BY val");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 30);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 40);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 50);
}

// ============================================================================
// T5: SELECT with PARAMSET_SIZE > 1 — last-set-wins documented behaviour
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: SELECT exposes only the last set's result.",
                 "[query][param_array][execution]") {
  // Given a table with 5 rows
  conn.execute("CREATE TEMPORARY TABLE param_array_select (id INTEGER)");
  conn.execute("INSERT INTO param_array_select VALUES (1),(2),(3),(4),(5)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT id FROM param_array_select WHERE id = ?"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Bind 3 sets: predicate values 1, 3, 5
  SQLINTEGER values[3] = {1, 3, 5};
  SQLLEN indicators[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called
  ret = SQLExecute(stmt.getHandle());

  // Then the result is from the last set (id = 5)
  NEW_DRIVER_ONLY("T5-select-last-set") {
    REQUIRE_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_LONG>(stmt, 1) == 5);
    // No more rows (only 1 row matches id=5)
    ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_NO_DATA);
  }
}

// ============================================================================
// T6: SQL_C_WCHAR (wide-char) array parameters
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: SQL_C_WCHAR array inserts correct strings.",
                 "[query][param_array][execution]") {
  // Given a varchar table
  conn.execute("CREATE TEMPORARY TABLE param_array_wchar (s VARCHAR(20))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_wchar VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // 3 wide-char buffers, fixed width 10 wide chars each (column-wise)
  const SQLLEN BUF = 10 * sizeof(SQLWCHAR);
  SQLWCHAR values[3][10] = {
      {'f', 'o', 'o', 0},
      {'b', 'a', 'r', 0},
      {'b', 'a', 'z', 0},
  };
  SQLLEN indicators[3] = {SQL_NTS, SQL_NTS, SQL_NTS};
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 20, 0, values, BUF, indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with SQL_C_WCHAR parameters
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then 3 rows with the correct string values are present
  auto sel = conn.execute("SELECT s FROM param_array_wchar ORDER BY s");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_CHAR>(sel, 1) == "bar");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_CHAR>(sel, 1) == "baz");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_CHAR>(sel, 1) == "foo");
}

// ============================================================================
// T7: SQL_C_TYPE_TIMESTAMP array parameters
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_ATTR_PARAMSET_SIZE > 1: SQL_C_TYPE_TIMESTAMP array inserts all sets.",
                 "[query][param_array][execution]") {
  // Given a TIMESTAMP column
  conn.execute("CREATE TEMPORARY TABLE param_array_ts (ts TIMESTAMP_NTZ)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_ts VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // 3 timestamp structs in column-wise layout
  SQL_TIMESTAMP_STRUCT values[3] = {};
  values[0] = {2024, 1, 1, 10, 0, 0, 0};
  values[1] = {2024, 6, 15, 12, 30, 0, 0};
  values[2] = {2024, 12, 31, 23, 59, 59, 0};
  SQLLEN indicators[3] = {0, 0, 0};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 19, 0, values,
                         sizeof(SQL_TIMESTAMP_STRUCT), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with SQL_C_TYPE_TIMESTAMP parameters
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then all 3 rows are inserted
  auto sel = conn.execute("SELECT COUNT(*) FROM param_array_ts");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 3);
}

// ============================================================================
// T8: Row-wise binding with two parameters
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture,
                 "SQL_ATTR_PARAMSET_SIZE > 1: row-wise binding with 2 parameters inserts correct values.",
                 "[query][param_array][execution]") {
  // Given a table with two columns
  conn.execute("CREATE TEMPORARY TABLE param_array_rw2 (a INTEGER, b VARCHAR(20))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_rw2 VALUES (?, ?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Row struct with both parameters and their indicators
  struct Row {
    SQLINTEGER a;
    SQLLEN a_ind;
    char b[20];
    SQLLEN b_ind;
  };
  Row rows[3] = {
      {10, 0, "alpha", SQL_NTS},
      {20, 0, "beta", SQL_NTS},
      {30, 0, "gamma", SQL_NTS},
  };

  // Row-wise: stride = sizeof(Row)
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, (SQLPOINTER)sizeof(Row), 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &rows[0].a,
                         sizeof(SQLINTEGER), &rows[0].a_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 20, 0, rows[0].b,
                         sizeof(rows[0].b), &rows[0].b_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with row-wise 2-column binding
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then all 3 rows are inserted with correct values
  auto sel = conn.execute("SELECT a, b FROM param_array_rw2 ORDER BY a");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 10);
  CHECK(get_data<SQL_C_CHAR>(sel, 2) == "alpha");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 20);
  CHECK(get_data<SQL_C_CHAR>(sel, 2) == "beta");
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_LONG>(sel, 1) == 30);
  CHECK(get_data<SQL_C_CHAR>(sel, 2) == "gamma");
}

// ============================================================================
// T11: DAE rejection when PARAMSET_SIZE > 1
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture,
                 "SQL_ATTR_PARAMSET_SIZE > 1: DAE indicator causes SQL_NEED_DATA regardless of array size.",
                 "[query][param_array][execution]") {
  // Given a prepared INSERT with a DAE indicator on parameter 1
  conn.execute("CREATE TEMPORARY TABLE param_array_dae (val INTEGER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_dae VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER values[3] = {1, 2, 3};
  SQLLEN indicators[3] = {SQL_DATA_AT_EXEC, SQL_DATA_AT_EXEC, SQL_DATA_AT_EXEC};
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                         sizeof(SQLINTEGER), indicators);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called with DAE and PARAMSET_SIZE > 1
  ret = SQLExecute(stmt.getHandle());

  // Then the driver initiates the normal DAE handshake regardless of array size
  CHECK(ret == SQL_NEED_DATA);
}

// ============================================================================
// T12: PARAMSET_SIZE == 1 with SQL_PARAM_IGNORE on the only set
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture,
                 "SQL_ATTR_PARAM_OPERATION_PTR SQL_PARAM_IGNORE with PARAMSET_SIZE == 1 skips the set.",
                 "[query][param_array][execution]") {
  // Given a table and a prepared insert
  conn.execute("CREATE TEMPORARY TABLE param_array_ignore1 (val INTEGER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO param_array_ignore1 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER val = 42;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &val, sizeof(SQLINTEGER),
                         &ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Mark the single set as SQL_PARAM_IGNORE
  SQLUSMALLINT op[1] = {SQL_PARAM_IGNORE};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, op, 0);
  REQUIRE_ODBC(ret, stmt);

  SQLUSMALLINT status[1] = {0xFF};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, status, 0);
  REQUIRE_ODBC(ret, stmt);

  SQLULEN processed = SQLULEN(-1);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE_ODBC(ret, stmt);

  // When SQLExecute is called
  ret = SQLExecute(stmt.getHandle());

  // Then the call succeeds but no row is inserted
  NEW_DRIVER_ONLY("T12-single-set-ignore") {
    REQUIRE_ODBC(ret, stmt);
    CHECK(status[0] == SQL_PARAM_UNUSED);
    CHECK(processed == 0);

    auto sel = conn.execute("SELECT COUNT(*) FROM param_array_ignore1");
    SQLRETURN fret = SQLFetch(sel.getHandle());
    REQUIRE_ODBC(fret, sel);
    CHECK(get_data<SQL_C_LONG>(sel, 1) == 0);
  }
}

// ============================================================================
// T13: PARAMSET_SIZE set to (SQLULEN)-1 must return SQL_ERROR HY024
// ============================================================================

TEST_CASE("SQL_ATTR_PARAMSET_SIZE set to (SQLULEN)-1 returns SQL_ERROR HY024.") {
#ifdef _WIN32
  // Windows DM may intercept this before reaching the driver.
  SKIP("Windows DM may intercept out-of-range PARAMSET_SIZE");
#endif
  // Given a connection and statement
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMSET_SIZE is set to the maximum SQLULEN value
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)(SQLULEN)-1, 0);

  // Then the new driver returns SQL_ERROR with SQLSTATE HY024
  NEW_DRIVER_ONLY("T13-paramset-overflow") {
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY024"));
  }
}
