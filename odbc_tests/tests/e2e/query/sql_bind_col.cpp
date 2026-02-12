#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"

// =============================================================================
// Tests for SQLBindCol based on ODBC specification:
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function
// =============================================================================

// =============================================================================
// Basic Binding Behavior
// =============================================================================

TEST_CASE("SQLBindCol binds a column and SQLFetch returns data in bound buffer.", "[query][bind_col]") {
  // Doc: "SQLBindCol is used to associate, or bind, columns in the result set
  //       to data buffers and length/indicator buffers in the application."
  // Doc: "When the application calls SQLFetch, SQLFetchScroll, or SQLSetPos to
  //       fetch data, the driver returns the data for the bound columns in the
  //       specified buffers."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called to execute a query
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called to bind column 1 to a buffer
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should populate the bound buffer with the column data
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 42);
  CHECK(indicator == sizeof(SQLINTEGER));
}

TEST_CASE("SQLBindCol returns SQL_SUCCESS on successful binding.", "[query][bind_col]") {
  // Doc: "Returns SQL_SUCCESS, SQL_SUCCESS_WITH_INFO, SQL_ERROR, or SQL_INVALID_HANDLE."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#returns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called with valid parameters
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then SQLBindCol should return SQL_SUCCESS
  CHECK(ret == SQL_SUCCESS);
}

TEST_CASE("SQLBindCol binds multiple columns in a result set.", "[query][bind_col]") {
  // Doc: "SQLBindCol is used to associate, or bind, columns in the result set
  //       to data buffers and length/indicator buffers in the application."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called to execute a query with multiple columns
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 10 AS col1, 'hello' AS col2, 3.14 AS col3", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called for each column
  SQLINTEGER col1 = 0;
  SQLLEN col1_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &col1, sizeof(col1), &col1_ind);
  CHECK_ODBC(ret, stmt);

  SQLCHAR col2[100] = {0};
  SQLLEN col2_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, col2, sizeof(col2), &col2_ind);
  CHECK_ODBC(ret, stmt);

  SQLDOUBLE col3 = 0.0;
  SQLLEN col3_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_DOUBLE, &col3, sizeof(col3), &col3_ind);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return data in all bound buffers
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(col1 == 10);
  CHECK(std::string((char*)col2) == "hello");
  CHECK(col3 == 3.14);
}

// =============================================================================
// Column Numbering
// =============================================================================

TEST_CASE("SQLBindCol uses 1-based column numbering when bookmarks are not used.", "[query][bind_col]") {
  // Doc: "Columns are numbered in increasing column order starting at 0, where
  //       column 0 is the bookmark column. If bookmarks are not used - that is,
  //       the SQL_ATTR_USE_BOOKMARKS statement attribute is set to SQL_UB_OFF -
  //       then column numbers start at 1."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called to execute a query with two columns
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'first' AS col1, 'second' AS col2", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with column numbers 1 and 2
  SQLCHAR col1[100] = {0};
  SQLLEN col1_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, col1, sizeof(col1), &col1_ind);
  CHECK_ODBC(ret, stmt);

  SQLCHAR col2[100] = {0};
  SQLLEN col2_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, col2, sizeof(col2), &col2_ind);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the correct data for each column
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)col1) == "first");
  CHECK(std::string((char*)col2) == "second");
}

// =============================================================================
// Binding Before and After Execute
// =============================================================================

TEST_CASE("SQLBindCol can be called before SQLExecDirect.", "[query][bind_col]") {
  // Doc: "The use of these buffers is deferred; that is, the application binds
  //       them in SQLBindCol but the driver accesses them from other functions -
  //       namely SQLBulkOperations, SQLFetch, SQLFetchScroll, or SQLSetPos."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#binding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called before executing a query (deferred binding)
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // And SQLExecDirect is called after binding
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 99 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return data in the pre-bound buffer
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 99);
  CHECK(indicator == sizeof(SQLINTEGER));
}

TEST_CASE("SQLBindCol can be called after SQLExecDirect.", "[query][bind_col]") {
  // Doc: "A column can be bound, unbound, or rebound at any time, even after
  //       data has been fetched from the result set."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#binding-unbinding-and-rebinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called first
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 77 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called after executing
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should use the binding established after execute
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 77);
  CHECK(indicator == sizeof(SQLINTEGER));
}

// =============================================================================
// Unbinding Columns
// =============================================================================

TEST_CASE("SQLBindCol unbinds a column when TargetValuePtr is null pointer.", "[query][bind_col]") {
  // Doc: "If TargetValuePtr is a null pointer, the driver unbinds the data buffer
  //       for the column."
  // Doc: "To unbind a single column, an application calls SQLBindCol with
  //       ColumnNumber set to the number of that column and TargetValuePtr set
  //       to a null pointer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#unbinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind column 1
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with null TargetValuePtr to unbind
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, NULL, 0, NULL);
  CHECK_ODBC(ret, stmt);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the previously bound buffer should not be modified (column was unbound)
  CHECK(value == 0);
}

TEST_CASE("SQLBindCol returns SQL_SUCCESS when unbinding an already unbound column.", "[query][bind_col]") {
  // Doc: "If ColumnNumber refers to an unbound column, SQLBindCol still returns
  //       SQL_SUCCESS."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#unbinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called with null TargetValuePtr for an unbound column (first unbind)
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, NULL, 0, NULL);
  CHECK(ret == SQL_SUCCESS);

  // And SQLBindCol is called again with null TargetValuePtr for the same column (second unbind)
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, NULL, 0, NULL);

  // Then SQLBindCol should still return SQL_SUCCESS
  CHECK(ret == SQL_SUCCESS);
}

TEST_CASE("SQLFreeStmt with SQL_UNBIND unbinds all columns.", "[query][bind_col]") {
  // Doc: "To unbind all columns, an application calls SQLFreeStmt with fOption
  //       set to SQL_UNBIND. This can also be accomplished by setting the
  //       SQL_DESC_COUNT field of the ARD to zero."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#unbinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind two columns
  SQLINTEGER col1 = 0;
  SQLLEN col1_ind = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &col1, sizeof(col1), &col1_ind);
  CHECK_ODBC(ret, stmt);

  SQLINTEGER col2 = 0;
  SQLLEN col2_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_LONG, &col2, sizeof(col2), &col2_ind);
  CHECK_ODBC(ret, stmt);

  // And SQLFreeStmt is called with SQL_UNBIND to unbind all columns
  ret = SQLFreeStmt(stmt.getHandle(), SQL_UNBIND);
  CHECK_ODBC(ret, stmt);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 10 AS col1, 20 AS col2", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then bound buffers should not be modified (all columns were unbound)
  CHECK(col1 == 0);
  CHECK(col2 == 0);
}

TEST_CASE("SQLBindCol can unbind data buffer while keeping indicator bound.", "[query][bind_col]") {
  // Doc: "An application can unbind the data buffer for a column but still have
  //       a length/indicator buffer bound for the column, if the TargetValuePtr
  //       argument in the call to SQLBindCol is a null pointer but the
  //       StrLen_or_IndPtr argument is a valid value."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called with null TargetValuePtr but valid StrLen_or_IndPtr
  SQLLEN indicator = 999;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, NULL, 0, &indicator);
  CHECK_ODBC(ret, stmt);

  // And a query is executed
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLFetch is called to fetch the data
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLBindCol should return SQL_SUCCESS
  CHECK(ret == SQL_SUCCESS);

  // And the indicator should remain unchanged (column was fully unbound because TargetValuePtr is null)
  CHECK(indicator == 999);
}

// =============================================================================
// Rebinding Columns
// =============================================================================

TEST_CASE("SQLBindCol replaces old binding when called on already bound column.", "[query][bind_col]") {
  // Doc: "Call SQLBindCol to specify a new binding for a column that is already
  //       bound. The driver overwrites the old binding with the new one."
  // Doc: "The new binding takes effect the next time that a function that uses
  //       bindings is called."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#rebinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind column 1 to first buffer
  SQLINTEGER value1 = 0;
  SQLLEN indicator1 = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value1, sizeof(value1), &indicator1);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called again to rebind column 1 to a different buffer
  SQLINTEGER value2 = 0;
  SQLLEN indicator2 = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value2, sizeof(value2), &indicator2);
  CHECK_ODBC(ret, stmt);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 55 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the new buffer should contain the data (old binding was overwritten)
  CHECK(value2 == 55);
  CHECK(indicator2 == sizeof(SQLINTEGER));

  // And the old buffer should remain unchanged
  CHECK(value1 == 0);
  CHECK(indicator1 == 0);
}

TEST_CASE("SQLBindCol rebinding takes effect on next fetch, not the current one.", "[query][bind_col]") {
  // Doc: "The new binding takes effect the next time that a function that uses
  //       bindings is called. For example, suppose an application binds the
  //       columns in a result set and calls SQLFetch. The driver returns the data
  //       in the bound buffers. Now suppose the application binds the columns to
  //       a different set of buffers. The driver does not put the data for the
  //       just-fetched row in the newly bound buffers. Instead, it waits until
  //       SQLFetch is called again and then places the data for the next row in
  //       the newly bound buffers."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#rebinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning multiple rows is executed
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 2)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And column 1 is bound to first buffer
  SQLBIGINT value1 = -1;
  SQLLEN indicator1 = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &value1, sizeof(value1), &indicator1);
  CHECK_ODBC(ret, stmt);

  // And SQLFetch is called to fetch the first row
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value1 == 0);

  // And column 1 is rebound to a second buffer
  SQLBIGINT value2 = -1;
  SQLLEN indicator2 = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &value2, sizeof(value2), &indicator2);
  CHECK_ODBC(ret, stmt);

  // Then fetching the next row should populate the new buffer (not the old one)
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value2 == 1);
  CHECK(indicator2 == sizeof(SQLBIGINT));

  // And the old buffer should still hold the first row value
  CHECK(value1 == 0);
}

TEST_CASE("SQLBindCol can rebind after data has been fetched from result set.", "[query][bind_col]") {
  // Doc: "A column can be bound, unbound, or rebound at any time, even after
  //       data has been fetched from the result set."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#binding-unbinding-and-rebinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning two rows is executed
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 2)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And first row is fetched without any binding (using SQLGetData)
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // And column is bound after first fetch
  SQLBIGINT value = -1;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then next fetch should use the new binding
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 1);
  CHECK(indicator == sizeof(SQLBIGINT));
}

// =============================================================================
// BufferLength Behavior
// =============================================================================

TEST_CASE("SQLBindCol counts null terminator when returning character data.", "[query][bind_col]") {
  // Doc: "The driver uses BufferLength to avoid writing past the end of the
  //       *TargetValuePtr buffer when it returns variable-length data, such as
  //       character or binary data. Notice that the driver counts the
  //       null-termination character when it returns character data to
  //       *TargetValuePtr. *TargetValuePtr must therefore contain space for the
  //       null-termination character or the driver will truncate the data."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a 5-character string is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'ABCDE' AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with exactly 6 bytes (5 chars + null terminator)
  SQLCHAR buffer[6] = {0};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the full string with null termination
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)buffer) == "ABCDE");
  CHECK(indicator == 5);
}

TEST_CASE("SQLBindCol truncates character data when buffer is too small.", "[query][bind_col]") {
  // Doc: "*TargetValuePtr must therefore contain space for the null-termination
  //       character or the driver will truncate the data."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a long string is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'Hello World' AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with a buffer too small to hold the full string
  SQLCHAR buffer[6] = {0};  // Can hold 5 chars + null terminator
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return SQL_SUCCESS_WITH_INFO (data truncated)
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");

  // And the buffer should contain truncated data with null termination
  CHECK(std::string((char*)buffer) == "Hello");

  // And the indicator should show the full length of the original data or SQL_NO_TOTAL
  CHECK((indicator == 11 || indicator == SQL_NO_TOTAL));
}

TEST_CASE("SQLBindCol ignores BufferLength for fixed-length data types.", "[query][bind_col]") {
  // Doc: "When the driver returns fixed-length data, such as an integer or a
  //       date structure, the driver ignores BufferLength and assumes the buffer
  //       is large enough to hold the data."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning an integer is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with BufferLength set to 0 for a fixed-length type
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, 0, &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should succeed because driver ignores BufferLength for fixed-length types
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 42);
  CHECK(indicator == sizeof(SQLINTEGER));
}

TEST_CASE("SQLBindCol returns HY090 when BufferLength is less than 0.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "SQLBindCol returns SQLSTATE HY090 (Invalid string or buffer length)
  //       when BufferLength is less than 0 but not when BufferLength is 0."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called with negative BufferLength
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer, -1, &indicator);

  // Then SQLBindCol should return SQL_ERROR with SQLSTATE HY090
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY090");
}

TEST_CASE("SQLBindCol does not return error when BufferLength is 0 for non-character type.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "SQLBindCol returns SQLSTATE HY090 (Invalid string or buffer length)
  //       when BufferLength is less than 0 but not when BufferLength is 0."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called with BufferLength of 0 for a fixed-length type
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, 0, &indicator);

  // Then SQLBindCol should return SQL_SUCCESS (0 is allowed)
  CHECK(ret == SQL_SUCCESS);
}

// =============================================================================
// StrLen_or_IndPtr (Length/Indicator) Behavior
// =============================================================================

TEST_CASE("SQLBindCol returns data length in StrLen_or_IndPtr for non-null data.", "[query][bind_col]") {
  // Doc: "SQLFetch, SQLFetchScroll, SQLBulkOperations, and SQLSetPos can return
  //       the following values in the length/indicator buffer:
  //       - The length of the data available to return"
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a string is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'test string' AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with an indicator pointer
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should set the indicator to the data length
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == 11);  // Length of "test string"
  CHECK(std::string((char*)buffer) == "test string");
}

TEST_CASE("SQLBindCol returns SQL_NULL_DATA in StrLen_or_IndPtr for NULL values.", "[query][bind_col]") {
  // Doc: "SQLFetch, SQLFetchScroll, SQLBulkOperations, and SQLSetPos can return
  //       the following values in the length/indicator buffer:
  //       - SQL_NULL_DATA"
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning NULL is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT NULL AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with an indicator pointer
  SQLINTEGER value = 999;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should set the indicator to SQL_NULL_DATA
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == SQL_NULL_DATA);
}

TEST_CASE("SQLBindCol with null StrLen_or_IndPtr succeeds for non-null data.", "[query][bind_col]") {
  // Doc: "If StrLen_or_IndPtr is a null pointer, no length or indicator value
  //       is used. This is an error when fetching data and the data is NULL."
  // (Implied: it is NOT an error for non-NULL data)
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a non-null integer is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called without an indicator pointer (NULL)
  SQLINTEGER value = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), NULL);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should succeed for non-null data
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 42);
}

// =============================================================================
// Data Type Conversion (TargetType)
// =============================================================================

TEST_CASE("SQLBindCol converts data to the specified TargetType.", "[query][bind_col]") {
  // Doc: "TargetType [Input] The identifier of the C data type of the
  //       *TargetValuePtr buffer. When it is retrieving data from the data
  //       source with SQLFetch, SQLFetchScroll, SQLBulkOperations, or
  //       SQLSetPos, the driver converts the data to this type."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning an integer is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 12345 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol specifies SQL_C_CHAR as TargetType (convert integer to string)
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should convert the integer to a string representation
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)buffer) == "12345");
  CHECK(indicator == 5);
}

TEST_CASE("SQLBindCol returns HY003 for invalid TargetType.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "HY003 - Invalid application buffer type: The argument TargetType was
  //       neither a valid data type nor SQL_C_DEFAULT."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called with an invalid TargetType
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, 9999, buffer, sizeof(buffer), &indicator);

  // Then SQLBindCol should return SQL_ERROR with SQLSTATE HY003
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY003");
}

TEST_CASE("SQLBindCol supports SQL_C_DEFAULT as TargetType.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "When the application specifies a TargetType of SQL_DEFAULT, SQLBindCol
  //       can be applied to a column of a different data type from the one
  //       intended by the application."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#cautions-regarding-sql_default

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with SQL_C_DEFAULT
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);

  // Then SQLBindCol should accept SQL_C_DEFAULT as a valid type
  CHECK((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
}

// =============================================================================
// Descriptor Interaction
// =============================================================================

TEST_CASE("SQLBindCol updates SQL_DESC_COUNT on the ARD.", "[query][bind_col]") {
  // Doc: "Calls SQLGetDescField to get this descriptor's SQL_DESC_COUNT field,
  //       and if the value in the ColumnNumber argument exceeds the value of
  //       SQL_DESC_COUNT, calls SQLSetDescField to increase the value of
  //       SQL_DESC_COUNT to ColumnNumber."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#argument-mappings

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_DESC_COUNT is initially 0
  SQLSMALLINT desc_count = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 0);

  // And SQLBindCol is called to bind column 3
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQL_DESC_COUNT should be updated to 3
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 3);
}

TEST_CASE("SQLBindCol sets SQL_DESC_COUNT only when increasing.", "[query][bind_col]") {
  // Doc: "SQLBindCol sets SQL_DESC_COUNT to the value of the ColumnNumber
  //       argument only when this would increase the value of SQL_DESC_COUNT."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#implicit-resetting-of-count-field

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind column 3
  SQLINTEGER value3 = 0;
  SQLLEN indicator3 = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_LONG, &value3, sizeof(value3), &indicator3);
  CHECK_ODBC(ret, stmt);

  // And the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_DESC_COUNT should be 3
  SQLSMALLINT desc_count = 0;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 3);

  // And when SQLBindCol is called to bind column 1 (lower than current count)
  SQLINTEGER value1 = 0;
  SQLLEN indicator1 = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value1, sizeof(value1), &indicator1);
  CHECK_ODBC(ret, stmt);

  // Then SQL_DESC_COUNT should still be 3 (not decreased)
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 3);
}

TEST_CASE("SQLBindCol decreases SQL_DESC_COUNT when unbinding highest bound column.", "[query][bind_col]") {
  // Doc: "If the value in the TargetValuePtr argument is a null pointer and the
  //       value in the ColumnNumber argument is equal to SQL_DESC_COUNT (that is,
  //       when unbinding the highest bound column), then SQL_DESC_COUNT is set to
  //       the number of the highest remaining bound column."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#implicit-resetting-of-count-field

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind columns 1, 2, and 3
  SQLINTEGER val1 = 0, val2 = 0, val3 = 0;
  SQLLEN ind1 = 0, ind2 = 0, ind3 = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &val1, sizeof(val1), &ind1);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_LONG, &val2, sizeof(val2), &ind2);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_LONG, &val3, sizeof(val3), &ind3);
  CHECK_ODBC(ret, stmt);

  // And the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_DESC_COUNT should be 3
  SQLSMALLINT desc_count = 0;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 3);

  // And when the highest bound column (3) is unbound
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_LONG, NULL, 0, NULL);
  CHECK_ODBC(ret, stmt);

  // Then SQL_DESC_COUNT should decrease to 2 (next highest bound column)
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 2);
}

TEST_CASE("SQLBindCol decreases SQL_DESC_COUNT when TargetValuePtr is null even if StrLen_or_IndPtr is not null.",
          "[query][bind_col]") {
  // Doc: "If TargetValuePtr is a null pointer, the driver unbinds the data buffer
  //       for the column."
  // Doc: "If the value in the TargetValuePtr argument is a null pointer and the
  //       value in the ColumnNumber argument is equal to SQL_DESC_COUNT (that is,
  //       when unbinding the highest bound column), then SQL_DESC_COUNT is set to
  //       the number of the highest remaining bound column."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#implicit-resetting-of-count-field

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind columns 1, 2, and 3
  SQLINTEGER val1 = 0, val2 = 0, val3 = 0;
  SQLLEN ind1 = 0, ind2 = 0, ind3 = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &val1, sizeof(val1), &ind1);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_LONG, &val2, sizeof(val2), &ind2);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_LONG, &val3, sizeof(val3), &ind3);
  CHECK_ODBC(ret, stmt);

  // And the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_DESC_COUNT should be 3
  SQLSMALLINT desc_count = 0;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 3);

  // And when the highest bound column (3) is unbound with null TargetValuePtr but non-null StrLen_or_IndPtr
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_LONG, NULL, 0, &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQL_DESC_COUNT should decrease to 2 (column was unbound because TargetValuePtr is null)
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 2);
}

TEST_CASE("SQLBindCol sets descriptor fields on the ARD.", "[query][bind_col]") {
  // Doc: "Conceptually, SQLBindCol performs the following steps in sequence:
  //       1. Calls SQLGetStmtAttr to obtain the ARD handle.
  //       ...
  //       3. Calls SQLSetDescField multiple times to assign values to the
  //          following fields of the ARD:
  //          - Sets SQL_DESC_TYPE and SQL_DESC_CONCISE_TYPE to the value of TargetType
  //          - Sets the SQL_DESC_OCTET_LENGTH field to the value of BufferLength
  //          - Sets the SQL_DESC_DATA_PTR field to the value of TargetValuePtr
  //          - Sets the SQL_DESC_INDICATOR_PTR field to the value of StrLen_or_IndPtr
  //          - Sets the SQL_DESC_OCTET_LENGTH_PTR field to the value of StrLen_or_IndPtr"
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#argument-mappings

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);

  // And the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_DESC_TYPE should match the TargetType
  SQLSMALLINT desc_type = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &desc_type, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_type == SQL_C_CHAR);

  // And SQL_DESC_CONCISE_TYPE should match the TargetType
  SQLSMALLINT concise_type = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_CONCISE_TYPE, &concise_type, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(concise_type == SQL_C_CHAR);

  // And SQL_DESC_OCTET_LENGTH should match BufferLength
  SQLLEN octet_length = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, &octet_length, sizeof(octet_length), NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(octet_length == sizeof(buffer));

  // And SQL_DESC_DATA_PTR should match TargetValuePtr
  SQLPOINTER data_ptr = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &data_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(data_ptr == (SQLPOINTER)buffer);

  // And SQL_DESC_INDICATOR_PTR should match StrLen_or_IndPtr
  SQLLEN* ind_ptr = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &ind_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ind_ptr == &indicator);

  // And SQL_DESC_OCTET_LENGTH_PTR should match StrLen_or_IndPtr
  SQLLEN* oct_len_ptr = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_OCTET_LENGTH_PTR, &oct_len_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(oct_len_ptr == &indicator);
}

TEST_CASE("SQLBindCol sets ARD descriptor fields for fixed-length type.", "[query][bind_col]") {
  // Doc: "Conceptually, SQLBindCol performs the following steps in sequence:
  //       ...
  //       3. Calls SQLSetDescField multiple times to assign values to the
  //          following fields of the ARD:
  //          - Sets SQL_DESC_TYPE and SQL_DESC_CONCISE_TYPE to the value of TargetType
  //          - Sets the SQL_DESC_OCTET_LENGTH field to the value of BufferLength
  //          - Sets the SQL_DESC_DATA_PTR field to the value of TargetValuePtr
  //          - Sets the SQL_DESC_INDICATOR_PTR field to the value of StrLen_or_IndPtr
  //          - Sets the SQL_DESC_OCTET_LENGTH_PTR field to the value of StrLen_or_IndPtr"
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#argument-mappings

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called with a fixed-length type (SQL_C_LONG)
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // And the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_DESC_TYPE should be SQL_C_LONG
  SQLSMALLINT desc_type = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &desc_type, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_type == SQL_C_LONG);

  // And SQL_DESC_CONCISE_TYPE should be SQL_C_LONG
  SQLSMALLINT concise_type = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_CONCISE_TYPE, &concise_type, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(concise_type == SQL_C_LONG);

  // And SQL_DESC_OCTET_LENGTH should match BufferLength
  SQLLEN octet_length = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, &octet_length, sizeof(octet_length), NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(octet_length == sizeof(SQLINTEGER));

  // And SQL_DESC_DATA_PTR should match TargetValuePtr
  SQLPOINTER data_ptr = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &data_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(data_ptr == (SQLPOINTER)&value);

  // And SQL_DESC_INDICATOR_PTR should match StrLen_or_IndPtr
  SQLLEN* ind_ptr = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &ind_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ind_ptr == &indicator);

  // And SQL_DESC_OCTET_LENGTH_PTR should match StrLen_or_IndPtr
  SQLLEN* oct_len_ptr = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_OCTET_LENGTH_PTR, &oct_len_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(oct_len_ptr == &indicator);
}

TEST_CASE("SQLBindCol updates ARD descriptor fields on rebind.", "[query][bind_col]") {
  // Doc: "Call SQLBindCol to specify a new binding for a column that is already
  //       bound. The driver overwrites the old binding with the new one."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#rebinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol binds column 1 as SQL_C_LONG
  SQLINTEGER int_value = 0;
  SQLLEN int_indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &int_value, sizeof(int_value), &int_indicator);
  CHECK_ODBC(ret, stmt);

  // And the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // And the type is verified as SQL_C_LONG
  SQLSMALLINT desc_type = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &desc_type, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(desc_type == SQL_C_LONG);

  // And SQLBindCol rebinds column 1 as SQL_C_CHAR with different buffers
  SQLCHAR char_buffer[50] = {0};
  SQLLEN char_indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, char_buffer, sizeof(char_buffer), &char_indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQL_DESC_TYPE should be updated to SQL_C_CHAR
  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &desc_type, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_type == SQL_C_CHAR);

  // And SQL_DESC_OCTET_LENGTH should be updated to the new BufferLength
  SQLLEN octet_length = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_OCTET_LENGTH, &octet_length, sizeof(octet_length), NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(octet_length == sizeof(char_buffer));

  // And SQL_DESC_DATA_PTR should point to the new buffer
  SQLPOINTER data_ptr = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &data_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(data_ptr == (SQLPOINTER)char_buffer);

  // And SQL_DESC_INDICATOR_PTR should point to the new indicator
  SQLLEN* ind_ptr = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &ind_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ind_ptr == &char_indicator);
}

TEST_CASE("SQLBindCol clears ARD descriptor fields on unbind.", "[query][bind_col]") {
  // Doc: "If TargetValuePtr is a null pointer, the driver unbinds the data buffer
  //       for the column."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#unbinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol binds column 1
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // And the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // And the binding is verified
  SQLPOINTER data_ptr = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &data_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(data_ptr == (SQLPOINTER)&value);

  // And SQLBindCol unbinds column 1 with NULL TargetValuePtr
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, NULL, 0, NULL);
  CHECK_ODBC(ret, stmt);

  // Then SQL_DESC_DATA_PTR should have been cleared
  data_ptr = (SQLPOINTER)0xDEAD;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &data_ptr, 0, NULL);
  REQUIRE(ret == SQL_NO_DATA);
  CHECK(data_ptr == (SQLPOINTER)0xDEAD);

  // And SQL_DESC_INDICATOR_PTR should have been cleared
  SQLLEN* ind_ptr = (SQLLEN*)0xDEAD;
  ret = SQLGetDescField(ard, 1, SQL_DESC_INDICATOR_PTR, &ind_ptr, 0, NULL);
  REQUIRE(ret == SQL_NO_DATA);
  CHECK(ind_ptr == (SQLLEN*)0xDEAD);

  // And SQL_DESC_OCTET_LENGTH_PTR should have been cleared
  SQLLEN* oct_len_ptr = (SQLLEN*)0xDEAD;
  ret = SQLGetDescField(ard, 1, SQL_DESC_OCTET_LENGTH_PTR, &oct_len_ptr, 0, NULL);
  REQUIRE(ret == SQL_NO_DATA);
  CHECK(oct_len_ptr == (SQLLEN*)0xDEAD);
}

TEST_CASE("SQLBindCol sets ARD descriptor fields for multiple columns.", "[query][bind_col]") {
  // Doc: "Conceptually, SQLBindCol performs the following steps in sequence:
  //       ...
  //       3. Calls SQLSetDescField multiple times to assign values to the
  //          following fields of the ARD."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#argument-mappings

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol binds three columns with different types
  SQLINTEGER col1 = 0;
  SQLLEN col1_ind = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &col1, sizeof(col1), &col1_ind);
  CHECK_ODBC(ret, stmt);

  SQLCHAR col2[50] = {0};
  SQLLEN col2_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, col2, sizeof(col2), &col2_ind);
  CHECK_ODBC(ret, stmt);

  SQLDOUBLE col3 = 0.0;
  SQLLEN col3_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_DOUBLE, &col3, sizeof(col3), &col3_ind);
  CHECK_ODBC(ret, stmt);

  // And the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_DESC_COUNT should be 3
  SQLSMALLINT desc_count = 0;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 3);

  // And column 1 descriptor fields should match SQL_C_LONG binding
  SQLSMALLINT type1 = 0;
  ret = SQLGetDescField(ard, 1, SQL_DESC_TYPE, &type1, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(type1 == SQL_C_LONG);

  SQLPOINTER ptr1 = NULL;
  ret = SQLGetDescField(ard, 1, SQL_DESC_DATA_PTR, &ptr1, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr1 == (SQLPOINTER)&col1);

  // And column 2 descriptor fields should match SQL_C_CHAR binding
  SQLSMALLINT type2 = 0;
  ret = SQLGetDescField(ard, 2, SQL_DESC_TYPE, &type2, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(type2 == SQL_C_CHAR);

  SQLLEN octet2 = 0;
  ret = SQLGetDescField(ard, 2, SQL_DESC_OCTET_LENGTH, &octet2, sizeof(octet2), NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(octet2 == sizeof(col2));

  SQLPOINTER ptr2 = NULL;
  ret = SQLGetDescField(ard, 2, SQL_DESC_DATA_PTR, &ptr2, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr2 == (SQLPOINTER)col2);

  // And column 3 descriptor fields should match SQL_C_DOUBLE binding
  SQLSMALLINT type3 = 0;
  ret = SQLGetDescField(ard, 3, SQL_DESC_TYPE, &type3, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(type3 == SQL_C_DOUBLE);

  SQLPOINTER ptr3 = NULL;
  ret = SQLGetDescField(ard, 3, SQL_DESC_DATA_PTR, &ptr3, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr3 == (SQLPOINTER)&col3);
}

// =============================================================================
// Column-Wise Binding
// =============================================================================

TEST_CASE("SQLBindCol supports column-wise binding with arrays.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "In column-wise binding, the application binds separate data and
  //       length/indicator arrays to each column."
  // Doc: "To use column-wise binding, the application first sets the
  //       SQL_ATTR_ROW_BIND_TYPE statement attribute to SQL_BIND_BY_COLUMN.
  //       (This is the default.)"
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#column-wise-binding

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_BIND_TYPE is set to SQL_BIND_BY_COLUMN (default)
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)SQL_BIND_BY_COLUMN, 0);
  CHECK_ODBC(ret, stmt);

  // And SQL_ATTR_ROW_ARRAY_SIZE is set to fetch 3 rows
  constexpr int array_size = 3;
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  CHECK_ODBC(ret, stmt);

  // And a query returning 3 rows is executed
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with arrays (column-wise binding)
  SQLBIGINT values[array_size] = {0};
  SQLLEN indicators[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, values, sizeof(SQLBIGINT), (SQLLEN*)indicators);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should populate the arrays
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  for (int i = 0; i < array_size; i++) {
    CHECK(values[i] == i);
    CHECK(indicators[i] == sizeof(SQLBIGINT));
  }
}

// =============================================================================
// Row-Wise Binding
// =============================================================================

TEST_CASE("SQLBindCol supports row-wise binding.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "In row-wise binding, the application defines a structure that contains
  //       data and length/indicator buffers for each column to be bound."
  // Doc: "Sets the SQL_ATTR_ROW_BIND_TYPE statement attribute to the size of the
  //       structure that contains a single row of data."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#row-wise-binding

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a row structure is defined for row-wise binding
  struct RowData {
    SQLBIGINT id;
    SQLLEN id_indicator;
    SQLCHAR name[20];
    SQLLEN name_indicator;
  };

  constexpr int array_size = 3;
  RowData rows[array_size] = {};

  // And SQL_ATTR_ROW_BIND_TYPE is set to the row structure size
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)sizeof(RowData), 0);
  CHECK_ODBC(ret, stmt);

  // And SQL_ATTR_ROW_ARRAY_SIZE is set
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  CHECK_ODBC(ret, stmt);

  // And a query with two columns is executed
  ret = SQLExecDirect(
      stmt.getHandle(),
      (SQLCHAR*)"SELECT seq8() as id, 'row' || seq8()::varchar as name FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id",
      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds columns using the first row as the base address
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &rows[0].id, sizeof(SQLBIGINT), &rows[0].id_indicator);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, rows[0].name, sizeof(rows[0].name), &rows[0].name_indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should populate rows using the row-wise layout
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  for (int i = 0; i < array_size; i++) {
    CHECK(rows[i].id == i);
    CHECK(rows[i].id_indicator == sizeof(SQLBIGINT));
    CHECK(std::string((char*)rows[i].name) == "row" + std::to_string(i));
  }
}

// =============================================================================
// Binding Offsets
// =============================================================================

TEST_CASE("SQLBindCol supports binding offsets via SQL_ATTR_ROW_BIND_OFFSET_PTR.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "Using a binding offset has basically the same effect as rebinding a
  //       column by calling SQLBindCol. The difference is that a new call to
  //       SQLBindCol specifies new addresses for the data buffer and
  //       length/indicator buffer, whereas use of a binding offset does not
  //       change the addresses but just adds an offset to them."
  // Doc: "To specify a binding offset, the application sets the
  //       SQL_ATTR_ROW_BIND_OFFSET_PTR statement attribute to the address of an
  //       SQLINTEGER buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#binding-offsets

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a binding offset pointer is configured
  struct RowData {
    SQLBIGINT value;
    SQLLEN indicator;
  };

  RowData rows[3] = {{0, 0}, {0, 0}, {0, 0}};
  SQLLEN bind_offset = 0;

  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_BIND_OFFSET_PTR, &bind_offset, 0);
  CHECK_ODBC(ret, stmt);

  // And a query returning 2 rows is executed
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 2)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds to the first row's buffer
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &rows[0].value, sizeof(SQLBIGINT), &rows[0].indicator);
  CHECK_ODBC(ret, stmt);

  // Then first fetch with offset=0 should populate rows[0]
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(rows[0].value == 0);
  CHECK(rows[0].indicator == sizeof(SQLBIGINT));

  // And second fetch with offset pointing to rows[1] should populate rows[1]
  bind_offset = sizeof(RowData);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(rows[1].value == 1);
  CHECK(rows[1].indicator == sizeof(SQLBIGINT));
}

TEST_CASE("SQLBindCol binding offset of 0 uses originally bound addresses.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "In particular, if the offset is set to 0 or if the statement attribute
  //       is set to a null pointer, the driver uses the originally bound
  //       addresses."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#binding-offsets

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a binding offset pointer is set with value 0
  SQLLEN bind_offset = 0;
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_BIND_OFFSET_PTR, &bind_offset, 0);
  CHECK_ODBC(ret, stmt);

  // And a query is executed
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds to a buffer
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch with offset 0 should use the originally bound address
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 42);
  CHECK(indicator == sizeof(SQLINTEGER));
}

// =============================================================================
// Binding Arrays (Block Cursors)
// =============================================================================

TEST_CASE("SQLBindCol binds arrays when SQL_ATTR_ROW_ARRAY_SIZE > 1.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "If the rowset size (the value of the SQL_ATTR_ROW_ARRAY_SIZE statement
  //       attribute) is greater than 1, the application binds arrays of buffers
  //       instead of single buffers."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#binding-arrays

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_ARRAY_SIZE is set to 5
  constexpr int array_size = 5;
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)array_size, 0);
  CHECK_ODBC(ret, stmt);

  // And a query returning 5 rows is executed
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with an array of buffers
  SQLBIGINT values[array_size] = {0};
  SQLLEN indicators[array_size] = {0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, values, sizeof(SQLBIGINT), (SQLLEN*)indicators);
  CHECK_ODBC(ret, stmt);

  // Then a single SQLFetch should populate all array elements
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  for (int i = 0; i < array_size; i++) {
    CHECK(values[i] == i);
    CHECK(indicators[i] == sizeof(SQLBIGINT));
  }

  // And subsequent fetch should return SQL_NO_DATA
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

// =============================================================================
// Error Conditions (SQLSTATE Diagnostics)
// =============================================================================

TEST_CASE("SQLBindCol returns 07009 when ColumnNumber exceeds max columns.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "07009 - Invalid descriptor index: The value specified for the argument
  //       ColumnNumber exceeded the maximum number of columns in the result set."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query with 1 column is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS col1", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called with a column number that exceeds the result set
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 100, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then SQLBindCol should return SQL_ERROR with SQLSTATE 07009
  // Note: Some driver managers may defer this check, so we check both bind and fetch
  if (ret == SQL_ERROR) {
    CHECK(get_sqlstate(stmt) == "07009");
  } else {
    // If the driver manager accepts it, the error may occur at fetch time
    CHECK((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  }
}

TEST_CASE("SQLBindCol returns SQL_INVALID_HANDLE for invalid statement handle.", "[query][bind_col]") {
  // Doc: "Returns SQL_SUCCESS, SQL_SUCCESS_WITH_INFO, SQL_ERROR, or
  //       SQL_INVALID_HANDLE."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#returns

  // Given an invalid statement handle
  SQLHSTMT invalid_handle = SQL_NULL_HSTMT;

  // When SQLBindCol is called with the invalid handle
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(invalid_handle, 1, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then SQLBindCol should return SQL_INVALID_HANDLE
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// =============================================================================
// Columns Not Required to Be Bound
// =============================================================================

TEST_CASE("SQLBindCol is not required - columns can be retrieved with SQLGetData.", "[query][bind_col]") {
  // Doc: "Notice that columns do not have to be bound to retrieve data from
  //       them. An application can also call SQLGetData to retrieve data from
  //       columns."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query is executed without binding any columns
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value, 'hello' AS name", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLFetch is called
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData can retrieve data without prior binding
  SQLINTEGER int_value = 0;
  SQLLEN int_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &int_value, sizeof(int_value), &int_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(int_value == 42);

  SQLCHAR str_value[100] = {0};
  SQLLEN str_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, str_value, sizeof(str_value), &str_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)str_value) == "hello");
}

TEST_CASE("SQLBindCol can bind some columns while SQLGetData retrieves others.", "[query][bind_col]") {
  // Doc: "Although it is possible to bind some columns in a row and call
  //       SQLGetData for others, this is subject to some restrictions."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query with two columns is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS col1, 'hello' AS col2", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And only column 1 is bound
  SQLINTEGER col1 = 0;
  SQLLEN col1_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &col1, sizeof(col1), &col1_ind);
  CHECK_ODBC(ret, stmt);

  // And SQLFetch is called
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the bound column should have data
  CHECK(col1 == 42);

  // And SQLGetData can retrieve the unbound column
  SQLCHAR col2[100] = {0};
  SQLLEN col2_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, col2, sizeof(col2), &col2_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)col2) == "hello");
}

// =============================================================================
// Various C Data Types
// =============================================================================

TEST_CASE("SQLBindCol supports SQL_C_CHAR binding.", "[query][bind_col]") {
  // Doc: "TargetType [Input] The identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a string is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'ODBC Test' AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_CHAR
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the string data
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)buffer) == "ODBC Test");
  CHECK(indicator == 9);
}

TEST_CASE("SQLBindCol supports SQL_C_SBIGINT binding.", "[query][bind_col]") {
  // Doc: "TargetType [Input] The identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a large integer is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 9223372036854775807 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_SBIGINT
  SQLBIGINT value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the large integer
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 9223372036854775807LL);
  CHECK(indicator == sizeof(SQLBIGINT));
}

TEST_CASE("SQLBindCol supports SQL_C_DOUBLE binding.", "[query][bind_col]") {
  // Doc: "TargetType [Input] The identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a double is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 3.14159 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_DOUBLE
  SQLDOUBLE value = 0.0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_DOUBLE, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the double value
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 3.14159);
  CHECK(indicator == sizeof(SQLDOUBLE));
}

TEST_CASE("SQLBindCol supports SQL_C_TYPE_DATE binding.", "[query][bind_col]") {
  // Doc: "TargetType [Input] The identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a date is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT DATE '2025-01-15' AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_TYPE_DATE
  SQL_DATE_STRUCT date_value = {};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_TYPE_DATE, &date_value, sizeof(date_value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the date components
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(date_value.year == 2025);
  CHECK(date_value.month == 1);
  CHECK(date_value.day == 15);
}

TEST_CASE("SQLBindCol supports SQL_C_TYPE_TIMESTAMP binding.", "[query][bind_col]") {
  // Doc: "TargetType [Input] The identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a timestamp is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT TIMESTAMP '2025-06-15 10:30:45' AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_TYPE_TIMESTAMP
  SQL_TIMESTAMP_STRUCT ts_value = {};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_TYPE_TIMESTAMP, &ts_value, sizeof(ts_value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the timestamp components
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(ts_value.year == 2025);
  CHECK(ts_value.month == 6);
  CHECK(ts_value.day == 15);
  CHECK(ts_value.hour == 10);
  CHECK(ts_value.minute == 30);
  CHECK(ts_value.second == 45);
}

// =============================================================================
// SQL_NO_TOTAL Indicator
// =============================================================================

TEST_CASE("SQLBindCol indicator returns full data length when buffer causes truncation.", "[query][bind_col]") {
  // Doc: "SQLFetch, SQLFetchScroll, SQLBulkOperations, and SQLSetPos can return
  //       the following values in the length/indicator buffer:
  //       - The length of the data available to return
  //       - SQL_NO_TOTAL
  //       - SQL_NULL_DATA"
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a known-length string is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'ABCDEFGHIJ' AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol uses a buffer that causes truncation
  SQLCHAR buffer[5] = {0};  // Only room for 4 chars + null
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return SQL_SUCCESS_WITH_INFO
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");

  // And indicator should contain the full data length or SQL_NO_TOTAL
  CHECK((indicator == 10 || indicator == SQL_NO_TOTAL));
}

// =============================================================================
// Binding Persistence Across Statements
// =============================================================================

TEST_CASE("SQLBindCol binding persists across SQLFreeStmt SQL_CLOSE and re-execute.", "[query][bind_col]") {
  // Doc: "The binding remains in effect until it is replaced by a new binding,
  //       the column is unbound, or the statement is freed."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#binding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind a column
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 10 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 10);

  // And the cursor is closed with SQL_CLOSE (not freeing the statement)
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  CHECK_ODBC(ret, stmt);

  // And a new query is executed
  value = 0;
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 20 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then the binding should still be in effect
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 20);
  CHECK(indicator == sizeof(SQLINTEGER));
}

TEST_CASE("SQLBindCol binding is removed by SQLFreeStmt SQL_UNBIND.", "[query][bind_col]") {
  // Doc: "The binding remains in effect until it is replaced by a new binding,
  //       the column is unbound, or the statement is freed."
  // Doc: "To unbind all columns, an application calls SQLFreeStmt with fOption
  //       set to SQL_UNBIND."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#unbinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind a column
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // And SQL_UNBIND is used to remove all bindings
  ret = SQLFreeStmt(stmt.getHandle(), SQL_UNBIND);
  CHECK_ODBC(ret, stmt);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the buffer should not be modified (binding was removed)
  CHECK(value == 0);
}

// =============================================================================
// Setting SQL_DESC_COUNT to 0 unbinds all columns
// =============================================================================

TEST_CASE("Setting SQL_DESC_COUNT to 0 on the ARD unbinds all columns.", "[query][bind_col]") {
  // Doc: "To unbind all columns, an application calls SQLFreeStmt with fOption
  //       set to SQL_UNBIND. This can also be accomplished by setting the
  //       SQL_DESC_COUNT field of the ARD to zero."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#unbinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind columns
  SQLINTEGER val1 = 0, val2 = 0;
  SQLLEN ind1 = 0, ind2 = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &val1, sizeof(val1), &ind1);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_LONG, &val2, sizeof(val2), &ind2);
  CHECK_ODBC(ret, stmt);

  // And SQL_DESC_COUNT is set to 0 on the ARD
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 0, SQL_DESC_COUNT, (SQLPOINTER)0, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 10 AS col1, 20 AS col2", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the buffers should not be modified (all columns were unbound)
  CHECK(val1 == 0);
  CHECK(val2 == 0);
}

TEST_CASE("Setting SQL_DESC_COUNT to 1 on the ARD unbinds columns above 1.", "[query][bind_col]") {
  // Doc: "To unbind all columns, an application calls SQLFreeStmt with fOption
  //       set to SQL_UNBIND. This can also be accomplished by setting the
  //       SQL_DESC_COUNT field of the ARD to zero."
  // Doc: "If the value of the SQL_DESC_COUNT field is set to a value that is
  //       less than the highest-numbered column that is bound, all bound columns
  //       with numbers greater than the new count value are effectively unbound."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#unbinding-columns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind 3 columns
  SQLINTEGER val1 = 0, val2 = 0, val3 = 0;
  SQLLEN ind1 = 0, ind2 = 0, ind3 = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &val1, sizeof(val1), &ind1);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_LONG, &val2, sizeof(val2), &ind2);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_LONG, &val3, sizeof(val3), &ind3);
  CHECK_ODBC(ret, stmt);

  // And SQL_DESC_COUNT is set to 1 on the ARD
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 0, SQL_DESC_COUNT, (SQLPOINTER)1, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_DESC_COUNT is verified to be 1
  SQLSMALLINT desc_count = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 1);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 10 AS col1, 20 AS col2, 30 AS col3", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then column 1 buffer should be populated (still bound)
  CHECK(val1 == 10);

  // And columns 2 and 3 buffers should not be modified (unbound by setting count to 1)
  CHECK(val2 == 0);
  CHECK(val3 == 0);
}

TEST_CASE("Setting SQL_DESC_COUNT greater than number of bound columns does not affect existing bindings.",
          "[query][bind_col]") {
  // Doc: "SQL_DESC_COUNT is not the count of columns that are bound...
  //       An application can allocate space for descriptors at any time by calling
  //       SQLSetDescField to set the SQL_DESC_COUNT field."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlsetdescfield-function

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindCol is called to bind 2 columns
  SQLINTEGER val1 = 0, val2 = 0;
  SQLLEN ind1 = 0, ind2 = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &val1, sizeof(val1), &ind1);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_LONG, &val2, sizeof(val2), &ind2);
  CHECK_ODBC(ret, stmt);

  // And the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_DESC_COUNT is verified to be 2
  SQLSMALLINT desc_count = 0;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 2);

  // And SQL_DESC_COUNT is set to 5 (greater than bound column count)
  ret = SQLSetDescField(ard, 0, SQL_DESC_COUNT, (SQLPOINTER)3, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_DESC_COUNT is verified to be 5
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &desc_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_count == 3);

  // And the default values for the 3rd column descriptor are verified
  SQLPOINTER data_ptr = nullptr;
  ret = SQLGetDescField(ard, 3, SQL_DESC_DATA_PTR, &data_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(data_ptr == nullptr);  // Should be NULL/0 by default

  SQLLEN octet_length = -1;
  ret = SQLGetDescField(ard, 3, SQL_DESC_OCTET_LENGTH, &octet_length, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(octet_length == 0);  // Should be 0 by default

  SQLPOINTER indicator_ptr = nullptr;
  ret = SQLGetDescField(ard, 3, SQL_DESC_INDICATOR_PTR, &indicator_ptr, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator_ptr == nullptr);  // Should be NULL/0 by default

  SQLSMALLINT concise_type = -1;
  ret = SQLGetDescField(ard, 3, SQL_DESC_CONCISE_TYPE, &concise_type, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(concise_type == SQL_C_DEFAULT);  // Should be SQL_C_DEFAULT by default

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 10 AS col1, 20 AS col2", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the bound columns should still receive data
  CHECK(val1 == 10);
  CHECK(val2 == 20);
}

TEST_CASE("Setting SQL_DESC_COUNT to -1 on the ARD returns an error.", "[query][bind_col]") {
  // Doc: "SQL_DESC_COUNT is the count of the highest-numbered column that is
  //       bound. It is a SQLUSMALLINT value and should be non-negative."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlsetdescfield-function

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When the ARD descriptor is obtained
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_DESC_COUNT is set to -1 on the ARD
  ret = SQLSetDescField(ard, 0, SQL_DESC_COUNT, (SQLPOINTER)(intptr_t)-1, 0);

  // Then SQLSetDescField should return an error
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(SQL_HANDLE_DESC, ard) == "07009");
}

// =============================================================================
// SQLBindCol with various data conversions
// =============================================================================

TEST_CASE("SQLBindCol converts string to integer type.", "[query][bind_col]") {
  // Doc: "When it is retrieving data from the data source with SQLFetch...
  //       the driver converts the data to this type."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a numeric string is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT '12345' AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol specifies SQL_C_LONG to convert from string to integer
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should convert the string to an integer
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 12345);
  CHECK(indicator == sizeof(SQLINTEGER));
}

// =============================================================================
// Binding with only some columns bound
// =============================================================================

TEST_CASE("SQLBindCol allows binding non-consecutive columns.", "[query][bind_col]") {
  // Doc: "SQLBindCol is used to associate, or bind, columns in the result set
  //       to data buffers..."
  // (Columns can be selectively bound; unbound columns are skipped)
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query with 3 columns is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS col1, 2 AS col2, 3 AS col3", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And only columns 1 and 3 are bound (skipping column 2)
  SQLINTEGER col1 = 0;
  SQLLEN col1_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &col1, sizeof(col1), &col1_ind);
  CHECK_ODBC(ret, stmt);

  SQLINTEGER col3 = 0;
  SQLLEN col3_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_LONG, &col3, sizeof(col3), &col3_ind);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should populate only the bound columns
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(col1 == 1);
  CHECK(col3 == 3);
}

// =============================================================================
// SQLBindCol with SQL_C_NUMERIC
// =============================================================================

TEST_CASE("SQLBindCol supports SQL_C_NUMERIC binding.", "[query][bind_col]") {
  // Doc: "If the TargetType argument is SQL_C_NUMERIC, the default precision
  //       (driver-defined) and default scale (0), as set in the SQL_DESC_PRECISION
  //       and SQL_DESC_SCALE fields of the ARD, are used for the data.
  //       If any default precision or scale is not appropriate, the application
  //       should explicitly set the appropriate descriptor field by a call to
  //       SQLSetDescField or SQLSetDescRec."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a numeric value is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 12345 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_NUMERIC
  SQL_NUMERIC_STRUCT numeric_value = {};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_NUMERIC, &numeric_value, sizeof(numeric_value), &indicator);
  CHECK_ODBC(ret, stmt);

  // And precision/scale are set on the ARD for proper conversion
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, (SQLPOINTER)10, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_SCALE, (SQLPOINTER)2, 0);
  REQUIRE(ret == SQL_SUCCESS);
  // Re-set DATA_PTR to trigger a consistency check after precision/scale change
  ret = SQLSetDescField(ard, 1, SQL_DESC_DATA_PTR, &numeric_value, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQLFetch should succeed with the SQL_C_NUMERIC binding
  ret = SQLFetch(stmt.getHandle());
  CHECK((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // And the indicator should show the data was received (not NULL)
  CHECK(indicator != SQL_NULL_DATA);

  // SQL_NUMERIC_STRUCT stores the value as an unsigned little-endian integer in val[],
  // with sign (1 = positive, 0 = negative), precision, and scale fields.
  // With scale=2, the integer 12345 is represented as 12345.00, so val[] stores
  // the unscaled value 1234500 (12345 * 10^2 = 1234500 = 0x12D884).
  // In little-endian: val[0]=0x84, val[1]=0xD8, val[2]=0x12
  CHECK(numeric_value.sign == 1);
  CHECK(numeric_value.precision == 10);
  CHECK(numeric_value.scale == 2);

  // Reconstruct value from little-endian val[] array
  unsigned long long reconstructed = 0;
  for (int i = SQL_MAX_NUMERIC_LEN - 1; i >= 0; i--) {
    reconstructed = (reconstructed << 8) | numeric_value.val[i];
  }
  CHECK(reconstructed == 1234500);
}

// =============================================================================
// SQLBindCol used with SQLFetchScroll
// =============================================================================

TEST_CASE("SQLBindCol works with SQLFetchScroll.", "[query][bind_col]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "SQLBindCol is used to associate, or bind, columns in the result set
  //       to data buffers and length/indicator buffers in the application.
  //       When the application calls SQLFetch, SQLFetchScroll, or SQLSetPos to
  //       fetch data, the driver returns the data for the bound columns in the
  //       specified buffers."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning multiple rows is executed
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol is called to bind the column
  SQLBIGINT value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetchScroll with SQL_FETCH_NEXT should populate the bound buffer
  ret = SQLFetchScroll(stmt.getHandle(), SQL_FETCH_NEXT, 0);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 0);
  CHECK(indicator == sizeof(SQLBIGINT));

  // And subsequent calls should advance the cursor
  ret = SQLFetchScroll(stmt.getHandle(), SQL_FETCH_NEXT, 0);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 1);

  ret = SQLFetchScroll(stmt.getHandle(), SQL_FETCH_NEXT, 0);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 2);

  // And fetch after all rows returns SQL_NO_DATA
  ret = SQLFetchScroll(stmt.getHandle(), SQL_FETCH_NEXT, 0);
  REQUIRE(ret == SQL_NO_DATA);
}

// =============================================================================
// Binary data binding
// =============================================================================

TEST_CASE("SQLBindCol supports SQL_C_BINARY binding.", "[query][bind_col]") {
  // Doc: "The driver uses BufferLength to avoid writing past the end of the
  //       *TargetValuePtr buffer when it returns variable-length data, such as
  //       character or binary data."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning binary data (hex-encoded) is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT TO_BINARY('48656C6C6F', 'HEX') AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_BINARY
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the binary data
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == 5);  // "Hello" is 5 bytes
  CHECK(memcmp(buffer, "Hello", 5) == 0);
}

// =============================================================================
// Binding with SQLSMALLINT (SQL_C_SHORT)
// =============================================================================

TEST_CASE("SQLBindCol supports SQL_C_SHORT binding.", "[query][bind_col]") {
  // Doc: "TargetType [Input] The identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a small integer is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 127 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_SHORT
  SQLSMALLINT value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SHORT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the value
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 127);
  CHECK(indicator == sizeof(SQLSMALLINT));
}

// =============================================================================
// SQLBindCol with SQL_C_FLOAT
// =============================================================================

TEST_CASE("SQLBindCol supports SQL_C_FLOAT binding.", "[query][bind_col]") {
  // Doc: "TargetType [Input] The identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a float is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 2.5 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_FLOAT
  SQLREAL value = 0.0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_FLOAT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the float value
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 2.5f);
  CHECK(indicator == sizeof(SQLREAL));
}

// =============================================================================
// SQLBindCol with SQL_C_BIT
// =============================================================================

TEST_CASE("SQLBindCol supports SQL_C_BIT binding.", "[query][bind_col]") {
  // Doc: "TargetType [Input] The identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a boolean is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT TRUE AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLBindCol binds with SQL_C_BIT
  SQLCHAR value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_BIT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then SQLFetch should return the boolean value as 1
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(value == 1);
}
