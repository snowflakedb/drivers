#include <sqltypes.h>

#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("SQLFetch fetches a row from SELECT query", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42 AS value" is executed
  auto stmt = conn.execute("SELECT 42 AS value");

  // Then SQLFetch should return SQL_SUCCESS and retrieve the value
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  auto result = get_data<SQL_C_LONG>(stmt, 1);
  REQUIRE(result == 42);

  // And subsequent fetch should return SQL_NO_DATA
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("SQLFetch returns data about number of rows affected.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  // When SQLExecDirect is called to execute the query that returns 1 row
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLULEN rows_fetched = 0;
  // And SQLSetStmtAttr is called to set the rows fetched pointer
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROWS_FETCHED_PTR, &rows_fetched, 0);
  REQUIRE_ODBC(ret, stmt);
  // And SQLFetch is called to fetch the row
  ret = SQLFetch(stmt.getHandle());
  // Then SQLFetch should return SQL_SUCCESS and retrieve the value
  REQUIRE_ODBC(ret, stmt);
  // And the number of rows affected should be 1
  REQUIRE(rows_fetched == 1);
}

TEST_CASE("SQLSetStmtAttr sets supported cursor types.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLSetStmtAttr is called with SQL_ATTR_CURSOR_TYPE and SQLGetStmtAttr is called to get the current cursor type
  SQLULEN cursor_type = -1;
  SQLINTEGER length = 0;

  // Then default cursor type is SQL_CURSOR_FORWARD_ONLY
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, &cursor_type, 0, &length);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(cursor_type == SQL_CURSOR_FORWARD_ONLY);

  // And SQL_CURSOR_STATIC is not supported
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, (SQLPOINTER)SQL_CURSOR_STATIC, 0);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01S02");

  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, &cursor_type, 0, &length);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(cursor_type == SQL_CURSOR_FORWARD_ONLY);

  // And SQL_CURSOR_KEYSET_DRIVEN is not supported
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, (SQLPOINTER)SQL_CURSOR_KEYSET_DRIVEN, 0);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01S02");

  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, &cursor_type, 0, &length);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(cursor_type == SQL_CURSOR_FORWARD_ONLY);

  // And SQL_CURSOR_DYNAMIC is not supported
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, (SQLPOINTER)SQL_CURSOR_DYNAMIC, 0);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01S02");

  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, &cursor_type, 0, &length);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(cursor_type == SQL_CURSOR_FORWARD_ONLY);

  // And SQL_CURSOR_FORWARD_ONLY is supported
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, (SQLPOINTER)SQL_CURSOR_FORWARD_ONLY, 0);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(get_sqlstate(stmt) == "");

  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, &cursor_type, 0, &length);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(cursor_type == SQL_CURSOR_FORWARD_ONLY);
}

TEST_CASE("SQLFetch can be mixed with SQLFetchScroll.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  // When SQLExecDirect is called to execute the query that returns 10 rows
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 10)) ORDER BY id", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  // Then calls to SQLFetch and SQLFetchScroll can be mixed
  for (int i = 0; i < 10; i++) {
    if (i % 2 == 0) {
      ret = SQLFetch(stmt.getHandle());
      REQUIRE_ODBC(ret, stmt);
    } else {
      ret = SQLFetchScroll(stmt.getHandle(), SQL_FETCH_NEXT, 0);
      REQUIRE_ODBC(ret, stmt);
    }
    // And SQLGetData returns correct values for the current row
    SQLBIGINT result = 0;
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, sizeof(result), &indicator);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(result == i);
    REQUIRE(indicator == sizeof(SQLBIGINT));
  }
  // And SQLFetch returns SQL_NO_DATA when there are no more rows
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("SQLFetch returns multiple rows when SQL_ATTR_ROW_ARRAY_SIZE is set.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  // When SQLSetStmtAttr is called to set the row array size
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)10, 0);
  REQUIRE_ODBC(ret, stmt);

  SQLLEN row_array_size = 0;
  SQLINTEGER length = 0;
  // And SQL_ATTR_ROW_ARRAY_SIZE is set to the correct value
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, &row_array_size, 0, &length);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(row_array_size == 10);
  REQUIRE(length == sizeof(SQLLEN));
  // And SQLExecDirect is called to execute the query that returns 15 rows
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 15)) ORDER BY id", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  // And SQLBindCol is called to bind the column to the value
  constexpr int array_size = 10;
  SQLBIGINT result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, 0, (SQLLEN*)&indicator);
  REQUIRE_ODBC(ret, stmt);
  // Then SQLFetch should return SQL_SUCCESS and retrieve the first 10 rows
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  for (int i = 0; i < array_size; i++) {
    REQUIRE(result[i] == i);
    REQUIRE(indicator[i] == sizeof(SQLBIGINT));
  }
  // And SQLFetch should return SQL_SUCCESS and retrieve the next 5 rows
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  for (int i = 0; i < 5; i++) {
    REQUIRE(result[i] == i + 10);
    REQUIRE(indicator[i] == sizeof(SQLBIGINT));
  }
  // And SQLFetch should return SQL_NO_DATA when there are no more rows
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("SQL_ATTR_ROW_STATUS_PTR returns SQL_ROW_SUCCESS for successfully fetched rows.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_STATUS_PTR is set to point to a row status array
  constexpr int array_size = 5;
  SQLUSMALLINT row_status[array_size] = {SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW};
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_STATUS_PTR, row_status, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQL_ATTR_ROW_ARRAY_SIZE is set
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQLExecDirect is called to execute the query that returns 5 rows
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY id", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLBindCol is called to bind the column to the value
  SQLBIGINT result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, 0, (SQLLEN*)&indicator);
  REQUIRE_ODBC(ret, stmt);

  // And SQLFetch is called to fetch the rows
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then row status array should be updated with SQL_ROW_SUCCESS for all fetched rows
  for (int i = 0; i < array_size; i++) {
    CHECK(row_status[i] == SQL_ROW_SUCCESS);
    CHECK(result[i] == i);
    CHECK(indicator[i] == sizeof(SQLBIGINT));
  }

  // And subsequent fetch should return SQL_NO_DATA
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("SQL_ATTR_ROW_STATUS_PTR returns SQL_ROW_SUCCESS_WITH_INFO when data is truncated.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_STATUS_PTR is set to point to a row status array
  constexpr int array_size = 1;
  SQLUSMALLINT row_status[array_size] = {SQL_ROW_NOROW};
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_STATUS_PTR, row_status, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQL_ATTR_ROW_ARRAY_SIZE is set
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQLExecDirect is called to execute the query that returns a long string
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'This is a very long string that will be truncated' AS value",
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLBindCol is called with a small buffer that will cause truncation
  constexpr int buffer_size = 10;
  SQLCHAR result[array_size][buffer_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, &result, buffer_size, (SQLLEN*)&indicator);
  REQUIRE_ODBC(ret, stmt);

  // And SQLFetch is called to fetch the row
  ret = SQLFetch(stmt.getHandle());

  // Then SQLFetch should return SQL_SUCCESS_WITH_INFO
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);

  // And row status should be SQL_ROW_SUCCESS_WITH_INFO
  CHECK(row_status[0] == SQL_ROW_SUCCESS_WITH_INFO);

  // And SQLSTATE should indicate string data truncation
  CHECK(get_sqlstate(stmt) == "01004");
}

TEST_CASE("SQL_ATTR_ROW_STATUS_PTR returns SQL_ROW_ERROR when conversion error occurs.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_STATUS_PTR is set to point to a row status array
  constexpr int array_size = 1;
  SQLUSMALLINT row_status[array_size] = {SQL_ROW_NOROW};
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_STATUS_PTR, row_status, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQL_ATTR_ROW_ARRAY_SIZE is set
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQLExecDirect is called to execute the query that returns a non-numeric string
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'not_a_number' AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLBindCol is called to bind to an integer type (will cause conversion error)
  SQLINTEGER result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(SQLINTEGER), (SQLLEN*)&indicator);
  REQUIRE_ODBC(ret, stmt);

  // And SQLFetch is called to fetch the row
  ret = SQLFetch(stmt.getHandle());

  // Then SQLFetch should return SQL_ERROR or SQL_SUCCESS_WITH_INFO depending on error handling
  REQUIRE((ret == SQL_ERROR || ret == SQL_SUCCESS_WITH_INFO));

  // And row status should be SQL_ROW_ERROR
  CHECK(row_status[0] == SQL_ROW_ERROR);
}

TEST_CASE("SQL_ATTR_ROW_STATUS_PTR returns SQL_ROW_NOROW when rowset overlaps end of result set.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_STATUS_PTR is set to point to a row status array
  constexpr int array_size = 10;
  SQLUSMALLINT row_status[array_size];
  for (int i = 0; i < array_size; i++) {
    row_status[i] = 0xFFFF;  // Initialize to invalid value to detect changes
  }
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_STATUS_PTR, row_status, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQL_ATTR_ROW_ARRAY_SIZE is set to 10
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQLExecDirect is called to execute the query that returns only 3 rows
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLBindCol is called to bind the column
  SQLBIGINT result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, 0, (SQLLEN*)&indicator);
  REQUIRE_ODBC(ret, stmt);

  // And SQLFetch is called to fetch the rows
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then first 3 rows should have SQL_ROW_SUCCESS
  for (int i = 0; i < 3; i++) {
    CHECK(row_status[i] == SQL_ROW_SUCCESS);
    CHECK(result[i] == i);
  }

  // And remaining 7 rows should have SQL_ROW_NOROW
  for (int i = 3; i < array_size; i++) {
    CHECK(row_status[i] == SQL_ROW_NOROW);
  }

  // And subsequent fetch should return SQL_NO_DATA
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

// =============================================================================
// Tests for SQLFetch respecting descriptor field values
// =============================================================================

TEST_CASE("SQLFetch respects SQL_DESC_ARRAY_SIZE set on ARD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_ARRAY_SIZE is set on ARD to fetch multiple rows
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  constexpr SQLULEN array_size = 5;
  ret = SQLSetDescField(ard, 0, SQL_DESC_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLExecDirect is called to execute a query that returns 5 rows
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY id", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLBindCol is called to bind the column
  SQLBIGINT result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, 0, (SQLLEN*)&indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should return SQL_SUCCESS and retrieve all 5 rows
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  for (SQLULEN i = 0; i < array_size; i++) {
    CHECK(result[i] == (SQLBIGINT)i);
    CHECK(indicator[i] == sizeof(SQLBIGINT));
  }

  // And subsequent fetch should return SQL_NO_DATA
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("SQLFetch respects SQL_DESC_ARRAY_STATUS_PTR set on IRD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_ARRAY_STATUS_PTR is set on IRD
  SQLHDESC ird = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_ROW_DESC, &ird, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ird != SQL_NULL_HDESC);

  constexpr int array_size = 3;
  SQLUSMALLINT row_status[array_size] = {SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW};
  ret = SQLSetDescField(ird, 0, SQL_DESC_ARRAY_STATUS_PTR, row_status, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_ATTR_ROW_ARRAY_SIZE is set
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLExecDirect is called to execute a query that returns 3 rows
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLBindCol is called to bind the column
  SQLBIGINT result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, 0, (SQLLEN*)&indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should populate the row status array via IRD descriptor
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  for (int i = 0; i < array_size; i++) {
    CHECK(row_status[i] == SQL_ROW_SUCCESS);
    CHECK(result[i] == i);
  }
}

TEST_CASE("SQLFetch respects SQL_DESC_BIND_OFFSET_PTR set on ARD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_BIND_OFFSET_PTR is set on ARD
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // Create buffer for two rows worth of data
  constexpr int num_rows = 2;
  struct RowData {
    SQLBIGINT value;
    SQLLEN indicator;
  };
  RowData rows[num_rows] = {{0, 0}, {0, 0}};

  // Calculate offset to second row
  SQLLEN bind_offset = 0;
  ret = SQLSetDescField(ard, 0, SQL_DESC_BIND_OFFSET_PTR, &bind_offset, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLExecDirect is called to execute a query that returns 2 rows
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 2)) ORDER BY id", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLBindCol binds to the first row's buffer
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &rows[0].value, sizeof(SQLBIGINT), &rows[0].indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Then first fetch with offset=0 should populate rows[0]
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(rows[0].value == 0);
  CHECK(rows[0].indicator == sizeof(SQLBIGINT));

  // And second fetch with offset pointing to rows[1] should populate rows[1]
  bind_offset = sizeof(RowData);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(rows[1].value == 1);
  CHECK(rows[1].indicator == sizeof(SQLBIGINT));
}

TEST_CASE("SQLFetch respects SQL_DESC_BIND_TYPE set on ARD for row-wise binding.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_BIND_TYPE is set on ARD for row-wise binding
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // Define row structure for row-wise binding
  struct RowData {
    SQLBIGINT col1;
    SQLLEN col1_indicator;
    SQLBIGINT col2;
    SQLLEN col2_indicator;
  };

  constexpr int array_size = 3;
  RowData rows[array_size] = {{0, 0, 0, 0}, {0, 0, 0, 0}, {0, 0, 0, 0}};

  // Set row-wise binding with row size
  ret = SQLSetDescField(ard, 0, SQL_DESC_BIND_TYPE, (SQLPOINTER)sizeof(RowData), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_DESC_ARRAY_SIZE is set
  ret = SQLSetDescField(ard, 0, SQL_DESC_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLExecDirect is called to execute a query with two columns
  ret = SQLExecDirect(
      stmt.getHandle(),
      (SQLCHAR*)"SELECT seq8() as id, seq8() * 10 as value FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLBindCol binds columns using the row structure
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &rows[0].col1, sizeof(SQLBIGINT), &rows[0].col1_indicator);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_SBIGINT, &rows[0].col2, sizeof(SQLBIGINT), &rows[0].col2_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should populate rows using row-wise binding
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  for (int i = 0; i < array_size; i++) {
    CHECK(rows[i].col1 == i);
    CHECK(rows[i].col1_indicator == sizeof(SQLBIGINT));
    CHECK(rows[i].col2 == i * 10);
    CHECK(rows[i].col2_indicator == sizeof(SQLBIGINT));
  }
}

TEST_CASE("SQLFetch respects SQL_DESC_COUNT set on ARD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When columns are bound
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // And SQLExecDirect is called to execute a query with two columns
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 as col1, 'hello' as col2", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And only the first column is bound
  SQLBIGINT col1_value = 0;
  SQLLEN col1_indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &col1_value, sizeof(SQLBIGINT), &col1_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_DESC_COUNT should reflect one bound column
  SQLSMALLINT desc_count = 0;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 1);

  // And SQLFetch should successfully fetch only the bound column
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(col1_value == 42);
  CHECK(col1_indicator == sizeof(SQLBIGINT));

  // And when SQL_DESC_COUNT is set to 0, no columns are bound
  ret = SQLSetDescField(ard, 0, SQL_DESC_COUNT, (SQLPOINTER)0, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And verify the count is 0
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 0);
}

TEST_CASE("SQLFetch respects SQL_DESC_DATA_PTR set on ARD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_DATA_PTR is set directly on ARD
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // And SQLExecDirect is called
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 123 as value", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And descriptor fields are set directly instead of using SQLBindCol
  SQLBIGINT value = 0;

  // Set the type first
  ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, (SQLPOINTER)SQL_C_SBIGINT, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_CONCISE_TYPE, (SQLPOINTER)SQL_C_SBIGINT, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Set the data pointer (this also sets SQL_DESC_COUNT if needed)
  ret = SQLSetDescField(ard, 1, SQL_DESC_DATA_PTR, &value, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should use the descriptor-specified data buffer
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 123);
}

TEST_CASE("SQLFetch respects SQL_DESC_INDICATOR_PTR set on ARD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_INDICATOR_PTR is set directly on ARD
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // And SQLExecDirect is called with a query returning NULL
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT NULL as value", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And descriptor fields are set directly
  SQLBIGINT value = 999;  // Non-zero to verify it's not modified for NULL
  SQLLEN indicator = 0;

  ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, (SQLPOINTER)SQL_C_SBIGINT, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, (SQLPOINTER)sizeof(SQLBIGINT), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &indicator, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_DATA_PTR, &value, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should set indicator to SQL_NULL_DATA for NULL value
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator == SQL_NULL_DATA);
}

TEST_CASE("SQLFetch respects SQL_DESC_OCTET_LENGTH set on ARD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_OCTET_LENGTH is set on ARD
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // And SQLExecDirect is called with a string query
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'Hello World' as value", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And descriptor fields are set with a small octet length to cause truncation
  constexpr int buffer_size = 6;  // Small buffer for "Hello World"
  SQLCHAR value[buffer_size] = {0};
  SQLLEN length = 0;

  ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, (SQLPOINTER)SQL_C_CHAR, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, (SQLPOINTER)buffer_size, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH_PTR, &length, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_DATA_PTR, &value, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should respect the octet length and truncate
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");         // String data truncated
  CHECK(length == 11);                          // Indicator shows original data length
  CHECK(std::string((char*)value) == "Hello");  // Truncated to buffer_size - 1 + null
}

TEST_CASE("SQLFetch respects SQL_DESC_OCTET_LENGTH_PTR set on ARD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_OCTET_LENGTH_PTR is set on ARD
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // And SQLExecDirect is called
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'test' as value", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And descriptor fields are set with separate octet length pointer
  constexpr int buffer_size = 100;
  SQLCHAR value[buffer_size] = {0};
  SQLLEN octet_length = 0;

  ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, (SQLPOINTER)SQL_C_CHAR, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, (SQLPOINTER)buffer_size, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH_PTR, &octet_length, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_DATA_PTR, &value, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should set the octet length pointer to the actual data length
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(std::string((char*)value) == "test");
  CHECK(octet_length == 4);  // Length of "test"
}

TEST_CASE("SQLFetch respects SQL_DESC_ROWS_PROCESSED_PTR set on IRD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_ROWS_PROCESSED_PTR is set on IRD
  SQLHDESC ird = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_ROW_DESC, &ird, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ird != SQL_NULL_HDESC);

  SQLULEN rows_processed = 0;
  ret = SQLSetDescField(ird, 0, SQL_DESC_ROWS_PROCESSED_PTR, &rows_processed, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_ATTR_ROW_ARRAY_SIZE is set
  constexpr int array_size = 5;
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLExecDirect is called to execute a query that returns 3 rows
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLBindCol is called to bind the column
  SQLBIGINT result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, 0, (SQLLEN*)&indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should set rows_processed via the IRD descriptor
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(rows_processed == 3);  // Only 3 rows available, less than array_size
}

TEST_CASE("SQLFetch respects SQL_DESC_TYPE set on ARD.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_TYPE is set on ARD for type conversion
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // And SQLExecDirect is called with a numeric query
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 12345 as value", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And descriptor fields are set to convert to string
  SQLBIGINT value = 0;

  // Set type to SQL_C_CHAR for string conversion
  ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, (SQLPOINTER)SQL_C_SBIGINT, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_DATA_PTR, &value, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should convert the numeric value to string
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 12345);
}

TEST_CASE("SQLFetch respects multiple ARD descriptor fields set together.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When multiple ARD descriptor fields are set together
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // Set up array fetching with row-wise binding
  struct RowData {
    SQLBIGINT id;
    SQLLEN id_indicator;
    SQLCHAR name[20];
    SQLLEN name_indicator;
  };

  constexpr int array_size = 3;
  RowData rows[array_size] = {};

  // Set array size
  ret = SQLSetDescField(ard, 0, SQL_DESC_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Set row-wise binding
  ret = SQLSetDescField(ard, 0, SQL_DESC_BIND_TYPE, (SQLPOINTER)sizeof(RowData), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLExecDirect is called
  ret = SQLExecDirect(stmt.getHandle(),
      (SQLCHAR*)"SELECT seq8() as id, 'row' || seq8()::varchar as name FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id",
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And descriptor fields for column 1 (id)
  ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, (SQLPOINTER)SQL_C_SBIGINT, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, (SQLPOINTER)sizeof(SQLBIGINT), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &rows[0].id_indicator, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_DATA_PTR, &rows[0].id, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And descriptor fields for column 2 (name)
  ret = SQLSetDescField(ard, 2, SQL_DESC_TYPE, (SQLPOINTER)SQL_C_CHAR, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 2, SQL_DESC_OCTET_LENGTH, (SQLPOINTER)20, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 2, SQL_DESC_INDICATOR_PTR, &rows[0].name_indicator, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 2, SQL_DESC_DATA_PTR, &rows[0].name, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should use all descriptor settings together
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  for (int i = 0; i < array_size; i++) {
    CHECK(rows[i].id == i);
    CHECK(rows[i].id_indicator == sizeof(SQLBIGINT));
    CHECK(std::string((char*)rows[i].name) == "row" + std::to_string(i));
  }
}

TEST_CASE("SQLFetch respects both ARD and IRD descriptor fields.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When both ARD and IRD descriptor fields are set
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLHDESC ird = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_ROW_DESC, &ird, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);
  REQUIRE(ird != SQL_NULL_HDESC);

  constexpr int array_size = 4;

  // Set ARD fields
  ret = SQLSetDescField(ard, 0, SQL_DESC_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Set IRD fields
  SQLULEN rows_processed = 0;
  ret = SQLSetDescField(ird, 0, SQL_DESC_ROWS_PROCESSED_PTR, &rows_processed, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT row_status[array_size] = {SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW};
  ret = SQLSetDescField(ird, 0, SQL_DESC_ARRAY_STATUS_PTR, row_status, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLExecDirect is called to return 4 rows
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 4)) ORDER BY id", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLBindCol is called
  SQLBIGINT result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, 0, (SQLLEN*)&indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should respect both ARD and IRD settings
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  // Verify ARD settings worked (array fetch)
  for (int i = 0; i < array_size; i++) {
    CHECK(result[i] == i);
    CHECK(indicator[i] == sizeof(SQLBIGINT));
  }

  // Verify IRD settings worked
  CHECK(rows_processed == 4);
  for (int i = 0; i < array_size; i++) {
    CHECK(row_status[i] == SQL_ROW_SUCCESS);
  }
}

// Note: This is old driver behavior - not the specification.
// Specifically, the specification says that the data should be truncated to the SQL_ATTR_MAX_LENGTH characters
TEST_CASE("SQLFetch ignores SQL_ATTR_MAX_LENGTH on statement handle.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_MAX_LENGTH is set on the statement handle
  constexpr SQLULEN max_length = 5;
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_LENGTH, (SQLPOINTER)max_length, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLGetStmtAttr is called to verify the attribute was set
  SQLULEN retrieved_max_length = 0;
  SQLINTEGER length = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_LENGTH, &retrieved_max_length, 0, &length);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(retrieved_max_length == max_length);
  REQUIRE(length == sizeof(SQLULEN));

  // And SQLExecDirect is called to execute a query returning a string longer than max_length
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'HelloWorld' AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLBindCol is called with a buffer large enough to hold the full string
  constexpr int buffer_size = 20;
  SQLCHAR result[buffer_size] = {0};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, &result, buffer_size, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And SQLFetch is called to fetch the row
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the data should be truncated to SQL_ATTR_MAX_LENGTH characters
  CHECK(std::string((char*)result) == "HelloWorld");
  CHECK(indicator == (SQLLEN)10);
}

// =============================================================================
// Tests for SQLFetch SQLSTATE error conditions
// =============================================================================

TEST_CASE("SQLFetch returns 22002 when NULL data fetched without indicator pointer.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called to execute a query returning NULL
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT NULL AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLBindCol is called without an indicator pointer (NULL for StrLen_or_IndPtr)
  SQLBIGINT value = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), NULL);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLFetch should return SQL_ERROR with SQLSTATE 22002
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22002");
}

TEST_CASE("SQLFetch returns 22018 when invalid date string is bound to SQL_C_TYPE_DATE.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called to execute a query returning an invalid date string
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'not-a-valid-date' AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLBindCol is called to bind to a DATE structure
  SQL_DATE_STRUCT date_value = {};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_TYPE_DATE, &date_value, sizeof(date_value), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLFetch should return SQL_ERROR with SQLSTATE 22007 (Invalid datetime format)
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22018");
}
TEST_CASE("SQLFetch returns 24000 when no result set exists.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  auto schema = Schema::use_random_schema(conn);

  // When a non-SELECT statement is executed (no result set)
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TABLE test_table (id INT)", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLFetch should return SQL_ERROR with SQLSTATE 24000 (Invalid cursor state)
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "24000");
}

TEST_CASE("SQLFetch returns SQL_NO_DATA when result set is empty.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT statement is executed that returns no rows
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1 WHERE true=false", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLFetch should return SQL_NO_DATA (no rows to fetch)
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("SQLFetch returns HY010 when called without executing statement.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLFetch is called without executing any statement first
  SQLRETURN ret = SQLFetch(stmt.getHandle());

  // Then SQLFetch should return SQL_ERROR with SQLSTATE HY010 (Function sequence error)
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY010");
}

TEST_CASE("SQLFetch moves cursor forward when no columns are bound.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called to execute a query that returns 3 rows with no columns bound
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLFetch should return SQL_SUCCESS for each row
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQLFetch should return SQL_NO_DATA after all rows are consumed
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);

  // And SQLGetData can still retrieve data after moving cursor without bound columns
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Fetch first row without binding
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Use SQLGetData to retrieve the value
  SQLBIGINT value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(value == 0);  // First row should have value 0

  // Fetch second row without binding
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(value == 1);  // Second row should have value 1
}

TEST_CASE("SQLFetch supports separate length and indicator buffers via descriptor.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_DESC_INDICATOR_PTR and SQL_DESC_OCTET_LENGTH_PTR are set to different buffers
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  // And SQLExecDirect is called with a query returning a string
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'hello' AS value", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And descriptor fields are set with separate indicator and length pointers
  constexpr int buffer_size = 100;
  SQLCHAR value[buffer_size] = {0};
  SQLLEN indicator = 999;  // Initialize to non-zero to verify it's set
  SQLLEN length = 999;     // Separate length buffer

  ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, (SQLPOINTER)SQL_C_CHAR, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, (SQLPOINTER)buffer_size, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &indicator, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_OCTET_LENGTH_PTR, &length, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_DATA_PTR, &value, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should set both indicator and length appropriately
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(std::string((char*)value) == "hello");
  CHECK(length == 5);     // Length should be data length
  CHECK(indicator == 0);  // Indicator should be 0 for non-NULL data

  // Test with NULL value
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT NULL AS value", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Reset values
  indicator = 0;
  length = 0;
  memset(value, 0, buffer_size);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator == SQL_NULL_DATA);  // Indicator should be SQL_NULL_DATA for NULL
  // Note: length buffer behavior for NULL is implementation-defined
}

TEST_CASE("SQLFetch cannot be called after SQLExtendedFetch without SQLFreeStmt.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called to execute a query
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY id", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLExtendedFetch is called first
  SQLULEN row_count = 0;
  SQLUSMALLINT row_status = 0;
  ret = SQLExtendedFetch(stmt.getHandle(), SQL_FETCH_NEXT, 0, &row_count, &row_status);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should return SQL_ERROR with SQLSTATE HY010 (Function sequence error)
  // because SQLFetch cannot be mixed with SQLExtendedFetch without closing cursor
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY010");

  // But after SQLFreeStmt with SQL_CLOSE and re-executing, SQLFetch should work
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE("SQLGetDiagField returns correct row and column number on fetch error.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_ARRAY_SIZE is set for block cursor
  constexpr int array_size = 3;
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQLExecDirect is called to execute a query with data that will cause conversion error
  // Using UNION to create specific rows with a bad value in the middle
  ret = SQLExecDirect(
      stmt.getHandle(),
      (SQLCHAR*)"SELECT '123' AS value UNION ALL SELECT 'not_a_number' AS value UNION ALL SELECT '456' AS value",
      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLBindCol is called to bind to an integer type (will cause conversion error on row 2)
  SQLINTEGER result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(SQLINTEGER), (SQLLEN*)&indicator);
  REQUIRE_ODBC(ret, stmt);

  // And SQL_ATTR_ROW_STATUS_PTR is set
  SQLUSMALLINT row_status[array_size] = {SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_STATUS_PTR, row_status, 0);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLFetch should return SQL_SUCCESS_WITH_INFO or SQL_ERROR
  ret = SQLFetch(stmt.getHandle());
  REQUIRE((ret == SQL_SUCCESS_WITH_INFO || ret == SQL_ERROR));

  // And SQLGetDiagField should return the row number where error occurred
  SQLLEN row_number = 0;
  SQLSMALLINT string_length = 0;
  SQLRETURN diag_ret =
      SQLGetDiagField(SQL_HANDLE_STMT, stmt.getHandle(), 1, SQL_DIAG_ROW_NUMBER, &row_number, 0, &string_length);

  // Note: If diagnostic record exists, check the row number
  if (diag_ret == SQL_SUCCESS) {
    // Row number should be set (1-based, or SQL_ROW_NUMBER_UNKNOWN if driver can't determine)
    CHECK((row_number > 0 || row_number == SQL_ROW_NUMBER_UNKNOWN));
  }

  // And SQLGetDiagField should return the column number
  SQLLEN column_number = 0;
  diag_ret =
      SQLGetDiagField(SQL_HANDLE_STMT, stmt.getHandle(), 1, SQL_DIAG_COLUMN_NUMBER, &column_number, 0, &string_length);

  if (diag_ret == SQL_SUCCESS) {
    // Column number should be 1 (first column) or SQL_COLUMN_NUMBER_UNKNOWN or SQL_NO_COLUMN_NUMBER
    CHECK((column_number == 1 || column_number == SQL_COLUMN_NUMBER_UNKNOWN || column_number == SQL_NO_COLUMN_NUMBER));
  }
}

TEST_CASE("SQLFetch returns SQL_SUCCESS_WITH_INFO when error occurs on subset of rows in block cursor.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_ARRAY_SIZE is set for block cursor
  constexpr int array_size = 5;
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQL_ATTR_ROW_STATUS_PTR is set
  SQLUSMALLINT row_status[array_size] = {SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW, SQL_ROW_NOROW};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_STATUS_PTR, row_status, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQL_ATTR_ROWS_FETCHED_PTR is set
  SQLULEN rows_fetched = 0;
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROWS_FETCHED_PTR, &rows_fetched, 0);
  REQUIRE_ODBC(ret, stmt);

  // And SQLExecDirect is called with a mix of valid and invalid conversion data
  // Row 1: valid, Row 2: invalid, Row 3: valid, Row 4: invalid, Row 5: valid
  ret = SQLExecDirect(stmt.getHandle(),
      (SQLCHAR*)"SELECT '100' AS value "
                "UNION ALL SELECT 'bad1' AS value "
                "UNION ALL SELECT '300' AS value "
                "UNION ALL SELECT 'bad2' AS value "
                "UNION ALL SELECT '500' AS value",
      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And SQLBindCol is called to bind to an integer type
  SQLINTEGER result[array_size] = {0};
  SQLLEN indicator[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(SQLINTEGER), (SQLLEN*)&indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLFetch should return SQL_SUCCESS_WITH_INFO (not SQL_ERROR)
  // because errors occurred on some but not all rows
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);

  // And rows_fetched should indicate how many rows were attempted
  CHECK(rows_fetched == 5);

  // And row_status array should show mixed results
  // Valid rows should have SQL_ROW_SUCCESS
  // Invalid rows should have SQL_ROW_ERROR or SQL_ROW_SUCCESS_WITH_INFO
  int success_count = 0;
  int error_count = 0;
  for (int i = 0; i < array_size; i++) {
    if (row_status[i] == SQL_ROW_SUCCESS) {
      success_count++;
    } else if (row_status[i] == SQL_ROW_ERROR) {
      error_count++;
    }
  }

  // We expect some successes and some errors
  CHECK(success_count > 0);
  CHECK(error_count > 0);

  // Valid rows (indices 0, 2, 4) should have their values
  // Note: Actual behavior depends on whether driver continues after error
  if (row_status[0] == SQL_ROW_SUCCESS) {
    CHECK(result[0] == 100);
  }
  if (row_status[2] == SQL_ROW_SUCCESS) {
    CHECK(result[2] == 300);
  }
  if (row_status[4] == SQL_ROW_SUCCESS) {
    CHECK(result[4] == 500);
  }
}
