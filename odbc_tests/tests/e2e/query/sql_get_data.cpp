#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"

// =============================================================================
// Tests for SQLGetData based on ODBC specification:
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function
// =============================================================================

// =============================================================================
// Basic Retrieval Behavior
// =============================================================================

TEST_CASE("SQLGetData retrieves data for a single column after SQLFetch.", "[query][get_data]") {
  // Doc: "SQLGetData retrieves data for a single column in the result set or for
  //       a single parameter after SQLParamData returns SQL_PARAM_DATA_AVAILABLE."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning data is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLFetch is called to position the cursor
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData should retrieve the data for the column
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 42);
  CHECK(indicator == sizeof(SQLINTEGER));
}

TEST_CASE("SQLGetData can be called multiple times to retrieve variable-length data in parts.", "[query][get_data]") {
  // Doc: "It can be called multiple times to retrieve variable-length data in parts."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning a long string is executed
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(),
                    (SQLCHAR*)"SELECT 'This is a very long string that will be retrieved in parts' AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLFetch is called to position the cursor
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData can be called multiple times with small buffers to retrieve data in parts
  SQLCHAR buffer1[10] = {0};
  SQLLEN indicator1 = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer1, sizeof(buffer1), &indicator1);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);  // Should indicate more data available
  CHECK(get_sqlstate(stmt) == "01004");   // Data truncated
  CHECK(std::string((char*)buffer1) == "This is a");

  // And subsequent calls should retrieve the remaining data
  SQLCHAR buffer2[20] = {0};
  SQLLEN indicator2 = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer2, sizeof(buffer2), &indicator2);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);  // Should indicate more data available
  CHECK(get_sqlstate(stmt) == "01004");   // Data truncated
  CHECK(std::string((char*)buffer2) == " very long string t");

  // And final call should retrieve the last part
  SQLCHAR buffer3[50] = {0};
  SQLLEN indicator3 = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer3, sizeof(buffer3), &indicator3);
  CHECK_ODBC(ret, stmt);  // Should be SQL_SUCCESS (no more data)
  CHECK(std::string((char*)buffer3) == "hat will be retrieved in parts");
}

TEST_CASE("SQLGetData can only be called after rows have been fetched.", "[query][get_data]") {
  // Doc: "SQLGetData returns the data in a specified column. SQLGetData can be called
  //       only after one or more rows have been fetched from the result set by SQLFetch,
  //       SQLFetchScroll, or SQLExtendedFetch."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query is executed but SQLFetch is NOT called
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData should return SQL_ERROR with SQLSTATE 24000 (Invalid cursor state)
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "24000");
}

TEST_CASE("SQLGetData can retrieve data after SQLFetchScroll.", "[query][get_data]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "SQLGetData can be called only after one or more rows have been fetched from
  //       the result set by SQLFetch, SQLFetchScroll, or SQLExtendedFetch."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning multiple rows is executed
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLFetchScroll is called to fetch the first row
  ret = SQLFetchScroll(stmt.getHandle(), SQL_FETCH_NEXT, 0);
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData should retrieve data for the current row
  SQLBIGINT value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 0);
  CHECK(indicator == sizeof(SQLBIGINT));
}

TEST_CASE("SQLGetData retrieves data for multiple columns.", "[query][get_data]") {
  // Doc: "SQLGetData retrieves data for a single column in the result set..."
  // (Called once per column for multiple columns)
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query with multiple columns is executed and fetched
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 10 AS col1, 'hello' AS col2, 3.14 AS col3", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData should retrieve each column individually
  SQLINTEGER col1 = 0;
  SQLLEN col1_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &col1, sizeof(col1), &col1_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(col1 == 10);

  SQLCHAR col2[100] = {0};
  SQLLEN col2_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, col2, sizeof(col2), &col2_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)col2) == "hello");

  SQLDOUBLE col3 = 0.0;
  SQLLEN col3_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 3, SQL_C_DOUBLE, &col3, sizeof(col3), &col3_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(col3 == 3.14);
}

// =============================================================================
// Return Values
// =============================================================================

TEST_CASE("SQLGetData returns SQL_SUCCESS on successful retrieval.", "[query][get_data]") {
  // Doc: "Returns: SQL_SUCCESS, SQL_SUCCESS_WITH_INFO, SQL_NO_DATA,
  //       SQL_STILL_EXECUTING, SQL_ERROR, or SQL_INVALID_HANDLE."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#returns

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLGetData is called with valid parameters
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then SQLGetData should return SQL_SUCCESS
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 42);
}

TEST_CASE("SQLGetData returns SQL_INVALID_HANDLE for invalid statement handle.", "[query][get_data]") {
  // Doc: "Returns: SQL_SUCCESS, SQL_SUCCESS_WITH_INFO, SQL_NO_DATA,
  //       SQL_STILL_EXECUTING, SQL_ERROR, or SQL_INVALID_HANDLE."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#returns

  // Given an invalid statement handle
  SQLHSTMT invalid_handle = SQL_NULL_HSTMT;

  // When SQLGetData is called with the invalid handle
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(invalid_handle, 1, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then SQLGetData should return SQL_INVALID_HANDLE
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// =============================================================================
// TargetType Conversion
// =============================================================================

TEST_CASE("SQLGetData converts data to the specified TargetType.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // Doc: "Otherwise, the C data type specified in SQLGetData overrides the C data
  //       type specified in SQLBindParameter."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 12345 AS value");

  // When SQLGetData specifies SQL_C_CHAR to convert integer to string
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then the data should be converted to string representation
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)buffer) == "12345");
  CHECK(indicator == 5);
}

TEST_CASE("SQLGetData with SQL_C_DEFAULT selects default C type based on SQL type.", "[query][get_data]") {
  // Doc: "If TargetType is SQL_C_DEFAULT, the driver selects the default C data
  //       type based on the SQL data type of the source."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // And we determine the SQL data type of the column
  SQLLEN sql_type = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_TYPE, NULL, 0, NULL, (SQLLEN*)&sql_type);
  CHECK_ODBC(ret, stmt);
  REQUIRE(sql_type == SQL_DECIMAL);

  // When SQLGetData is called with SQL_C_DEFAULT
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);

  // Then the driver should select a default C type and return data
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)buffer) == "42");
  CHECK(indicator == 2);
}

TEST_CASE("SQLGetData overrides SQLBindCol type when a different TargetType is specified.", "[query][get_data]") {
  // Doc: "If TargetType is SQL_ARD_TYPE, the driver uses the type identifier
  //       specified in the SQL_DESC_CONCISE_TYPE field of the ARD... Otherwise,
  //       the C data type specified in SQLGetData overrides the C data type
  //       specified in SQLBindParameter."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a column is bound as SQL_C_LONG via SQLBindCol
  SQLINTEGER bound_value = 0;
  SQLLEN bound_ind = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &bound_value, sizeof(bound_value), &bound_ind);
  CHECK_ODBC(ret, stmt);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData with SQL_C_CHAR should override the bound type and return a string
  SQLCHAR char_buffer[100] = {0};
  SQLLEN char_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, char_buffer, sizeof(char_buffer), &char_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)char_buffer) == "42");
  CHECK(char_ind == 2);
}

// =============================================================================
// TargetValuePtr (Output Buffer)
// =============================================================================

TEST_CASE("SQLGetData returns HY009 when TargetValuePtr is null.", "[query][get_data]") {
  // Doc: "TargetValuePtr cannot be NULL."
  // Doc: "HY009 - Invalid use of null pointer: (DM) The argument TargetValuePtr
  //       was a null pointer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLGetData is called with NULL TargetValuePtr
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, NULL, 0, &indicator);

  // Then SQLGetData should return SQL_ERROR with SQLSTATE HY009
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY009");
}

// =============================================================================
// BufferLength Behavior
// =============================================================================

TEST_CASE("SQLGetData counts null terminator when returning character data.", "[query][get_data]") {
  // Doc: "The driver uses BufferLength to avoid writing past the end of the
  //       *TargetValuePtr buffer when returning variable-length data, such as
  //       character or binary data. Note that the driver counts the
  //       null-termination character when returning character data to
  //       *TargetValuePtr. *TargetValuePtr must therefore contain space for the
  //       null-termination character, or the driver will truncate the data."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'ABCDE' AS value");

  // When SQLGetData is called with exactly 6 bytes (5 chars + null)
  SQLCHAR buffer[6] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then the full string should be returned with null termination
  CHECK_ODBC(ret, stmt);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(std::string((char*)buffer) == "ABCDE");
  CHECK(indicator == 5);
}

TEST_CASE("SQLGetData truncates character data when buffer is too small.", "[query][get_data]") {
  // Doc: "If the length of character data (including the null-termination character)
  //       exceeds BufferLength, SQLGetData truncates the data to BufferLength less
  //       the length of a null-termination character. It then null-terminates the data."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'Hello World' AS value");

  // When SQLGetData is called with a buffer too small for the full string
  SQLCHAR buffer[6] = {0};  // Can hold 5 chars + null
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");

  // And the buffer should contain truncated data with null termination
  CHECK(std::string((char*)buffer) == "Hello");

  // And the indicator should show the full length of the original data
  CHECK((indicator == SQL_NO_TOTAL || indicator == 11));
}

TEST_CASE("SQLGetData truncates binary data to BufferLength bytes.", "[query][get_data]") {
  // Doc: "If the length of binary data exceeds the length of the data buffer,
  //       SQLGetData truncates it to BufferLength bytes."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT TO_BINARY('48656C6C6F576F726C64', 'HEX') AS value");

  // When SQLGetData is called with a small buffer for binary data
  SQLCHAR buffer[3] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");

  // And the indicator should show SQL_NO_TOTAL
  CHECK(indicator == 10);

  // And the buffer should contain the first 3 bytes
  CHECK(memcmp(buffer, "Hel", 3) == 0);
}

TEST_CASE("SQLGetData ignores BufferLength for fixed-length data types.", "[query][get_data]") {
  // Doc: "When the driver returns fixed-length data, such as an integer or a date
  //       structure, the driver ignores BufferLength and assumes the buffer is
  //       large enough to hold the data. It is therefore important for the
  //       application to allocate a large enough buffer for fixed-length data or
  //       the driver will write past the end of the buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLGetData is called with BufferLength 0 for a fixed-length type
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, 0, &indicator);

  // Then the driver should ignore BufferLength and return the data
  CHECK_ODBC(ret, stmt);
  CHECK(value == 42);
  CHECK(indicator == sizeof(SQLINTEGER));
}

TEST_CASE("SQLGetData returns HY090 when BufferLength is less than 0.", "[query][get_data]") {
  // Doc: "SQLGetData returns SQLSTATE HY090 (Invalid string or buffer length) when
  //       BufferLength is less than 0 but not when BufferLength is 0."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'test' AS value");

  // When SQLGetData is called with negative BufferLength
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, -1, &indicator);

  // Then SQLGetData should return SQL_ERROR with SQLSTATE HY090
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY090");
}

TEST_CASE("SQLGetData does not return error when BufferLength is 0.", "[query][get_data]") {
  // Doc: "SQLGetData returns SQLSTATE HY090 (Invalid string or buffer length) when
  //       BufferLength is less than 0 but not when BufferLength is 0."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLGetData is called with BufferLength 0 for fixed-length type
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, 0, &indicator);

  // Then SQLGetData should not return SQL_ERROR for BufferLength 0
  CHECK(ret != SQL_ERROR);
}

// =============================================================================
// StrLen_or_IndPtr (Length/Indicator)
// =============================================================================

TEST_CASE("SQLGetData returns data length in StrLen_or_IndPtr for character data.", "[query][get_data]") {
  // Doc: "For character or binary data, this is the length of the data after
  //       conversion and before truncation due to BufferLength."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'test string' AS value");

  // When SQLGetData is called with a large buffer
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then the indicator should contain the data length (not including null terminator)
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == 11);  // Length of "test string"
  CHECK(std::string((char*)buffer) == "test string");
}

TEST_CASE("SQLGetData returns type size in StrLen_or_IndPtr for fixed-length data.", "[query][get_data]") {
  // Doc: "For all other data types, this is the length of the data after
  //       conversion; that is, it is the size of the type to which the data was
  //       converted."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLGetData retrieves fixed-length data
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then the indicator should contain the size of the C type
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == sizeof(SQLINTEGER));
}

TEST_CASE("SQLGetData returns SQL_NULL_DATA in StrLen_or_IndPtr for NULL values.", "[query][get_data]") {
  // Doc: "Sets *StrLen_or_IndPtr to SQL_NULL_DATA if the data is NULL."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT NULL AS value");

  // When SQLGetData is called for a NULL column
  SQLINTEGER value = 999;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then the indicator should be SQL_NULL_DATA
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == SQL_NULL_DATA);
}

TEST_CASE("SQLGetData returns 22002 when NULL data fetched without indicator pointer.", "[query][get_data]") {
  // Doc: "If the data is NULL and StrLen_or_IndPtr was a null pointer, SQLGetData
  //       returns SQLSTATE 22002 (Indicator variable required but not supplied)."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT NULL AS value");

  // When SQLGetData is called with NULL StrLen_or_IndPtr for a NULL column
  SQLINTEGER value = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), NULL);

  // Then SQLGetData should return SQL_ERROR with SQLSTATE 22002
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22002");
}

TEST_CASE("SQLGetData with null StrLen_or_IndPtr succeeds for non-null data.", "[query][get_data]") {
  // Doc: "If this is a null pointer, no length or indicator value is returned."
  // (Implied: not an error for non-NULL data)
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLGetData is called without an indicator pointer
  SQLINTEGER value = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), NULL);

  // Then SQLGetData should succeed for non-null data
  CHECK_ODBC(ret, stmt);
  CHECK(value == 42);
}

// =============================================================================
// Retrieving Variable-Length Data in Parts
// =============================================================================

TEST_CASE("SQLGetData retrieves character data in parts with multiple calls.", "[query][get_data]") {
  // Doc: "To retrieve data from a column in parts, the application calls SQLGetData
  //       multiple times in succession for the same column. On each call, SQLGetData
  //       returns the next part of the data."
  // Doc: "If there is more data to return or not enough buffer was allocated for
  //       the terminating character, SQLGetData returns SQL_SUCCESS_WITH_INFO and
  //       SQLSTATE 01004 (Data truncated). When it returns the last part of the data,
  //       SQLGetData returns SQL_SUCCESS."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-variable-length-data-in-parts

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'ABCDEFGHIJ' AS value");

  // When SQLGetData is called with a 4-byte buffer (3 chars + null)
  SQLCHAR buffer[4] = {0};
  SQLLEN indicator = 0;
  std::string result;

  // Then the first call should return "ABC" with SQL_SUCCESS_WITH_INFO
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");
  result += std::string((char*)buffer);

  // And the second call should return "DEF" with SQL_SUCCESS_WITH_INFO
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");
  result += std::string((char*)buffer);

  // And the third call should return "GHI" with SQL_SUCCESS_WITH_INFO
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");
  result += std::string((char*)buffer);

  // And the fourth call should return "J" with SQL_SUCCESS (last part)
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  result += std::string((char*)buffer);

  // And the complete string should be reconstructed
  CHECK(result == "ABCDEFGHIJ");
}

TEST_CASE("SQLGetData returns SQL_NO_DATA after all data for a column has been retrieved.", "[query][get_data]") {
  // Doc: "Returns SQL_NO_DATA if it has already returned all of the data for the column."
  // Doc: "If SQLGetData is called after this, it returns SQL_NO_DATA."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-variable-length-data-in-parts

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'test' AS value");

  // When SQLGetData retrieves all data in the first call
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(std::string((char*)buffer) == "test");

  // Then subsequent SQLGetData on the same column should return SQL_NO_DATA
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("SQLGetData returns SQL_NO_DATA for fixed-length data after first successful call.", "[query][get_data]") {
  // Doc: "SQLGetData cannot be used to return fixed-length data in parts. If
  //       SQLGetData is called more than one time in a row for a column containing
  //       fixed-length data, it returns SQL_NO_DATA for all calls after the first."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-variable-length-data-in-parts

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLGetData retrieves the fixed-length integer
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 42);

  // Then subsequent calls should return SQL_NO_DATA
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE(ret == SQL_NO_DATA);
}

// =============================================================================
// Interaction with Bound Columns
// =============================================================================

TEST_CASE("SQLGetData can be called on unbound columns after bound columns.", "[query][get_data]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "It is possible to bind some columns in a row and call SQLGetData for
  //       others, although this is subject to some restrictions."
  // Doc: "If the driver does not support extensions to SQLGetData, the function
  //       can return data only for unbound columns with a number greater than
  //       that of the last bound column."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#comments
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#using-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When column 1 is bound
  SQLINTEGER col1 = 0;
  SQLLEN col1_ind = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &col1, sizeof(col1), &col1_ind);
  CHECK_ODBC(ret, stmt);

  // And a query with two columns is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS col1, 'hello' AS col2", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the bound column should have data
  CHECK(col1 == 42);

  // And SQLGetData should retrieve the unbound column (column 2 > last bound column 1)
  SQLCHAR col2[100] = {0};
  SQLLEN col2_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, col2, sizeof(col2), &col2_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)col2) == "hello");
}

// =============================================================================
// Column Ordering Restrictions
// =============================================================================

TEST_CASE("SQLGetData retrieves data in increasing column number order.", "[query][get_data]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "Furthermore, within a row of data, the value of the Col_or_Param_Num
  //       argument in each call to SQLGetData must be greater than or equal to
  //       the value of Col_or_Param_Num in the previous call; that is, data must
  //       be retrieved in increasing column number order."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#using-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 1 AS col1, 2 AS col2, 3 AS col3");

  // When SQLGetData is called in increasing column order
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;

  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 1);

  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 2);

  ret = SQLGetData(stmt.getHandle(), 3, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 3);
}

// =============================================================================
// SQL_GETDATA_EXTENSIONS - SQL_GD_ANY_COLUMN
// =============================================================================

TEST_CASE("SQLGetInfo returns SQL_GETDATA_EXTENSIONS bitmask.", "[query][get_data]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "To determine what restrictions a driver relaxes, an application calls
  //       SQLGetInfo with any of the following SQL_GETDATA_EXTENSIONS options:
  //       SQL_GD_OUTPUT_PARAMS, SQL_GD_ANY_COLUMN, SQL_GD_ANY_ORDER, SQL_GD_BLOCK,
  //       SQL_GD_BOUND."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#using-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;

  // When SQLGetInfo is called for SQL_GETDATA_EXTENSIONS
  SQLINTEGER extensions = 0;
  SQLSMALLINT length = 0;
  SQLRETURN ret =
      SQLGetInfo(conn.handleWrapper().getHandle(), SQL_GETDATA_EXTENSIONS, &extensions, sizeof(extensions), &length);
  CHECK_ODBC(ret, conn.handleWrapper());

  // Then the call should succeed and return a valid bitmask
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(length == sizeof(SQLINTEGER));

  // Then the bitmask should report SQL_GD_ANY_COLUMN, SQL_GD_ANY_ORDER, and SQL_GD_BOUND
  CHECK((extensions & SQL_GD_ANY_COLUMN) != 0);
  CHECK((extensions & SQL_GD_ANY_ORDER) != 0);
  CHECK((extensions & SQL_GD_BLOCK) == 0);
  CHECK((extensions & SQL_GD_BOUND) != 0);
  CHECK((extensions & SQL_GD_OUTPUT_PARAMS) == 0);
}

// =============================================================================
// Successive Calls Reset Prior Column Offsets
// =============================================================================

TEST_CASE("SQLGetData for a different column resets prior column offset.", "[query][get_data]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "Successive calls to SQLGetData will retrieve data from the last column
  //       requested; prior offsets become invalid. For example, when the following
  //       sequence is performed:
  //       SQLGetData(icol=n), SQLGetData(icol=m), SQLGetData(icol=n)
  //       the second call to SQLGetData(icol=n) retrieves data from the start of
  //       the n column."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;

  // And SQLGetInfo indicates SQL_GD_ANY_ORDER is supported
  SQLINTEGER extensions = 0;
  SQLSMALLINT length = 0;
  SQLRETURN ret =
      SQLGetInfo(conn.handleWrapper().getHandle(), SQL_GETDATA_EXTENSIONS, &extensions, sizeof(extensions), &length);
  CHECK_ODBC(ret, conn.handleWrapper());
  if ((extensions & SQL_GD_ANY_ORDER) == 0) {
    SKIP("Driver does not support SQL_GD_ANY_ORDER");
  }

  auto stmt = conn.execute_fetch("SELECT 'ABCDEFGHIJ' AS col1, 'XY' AS col2");

  // When SQLGetData partially reads column 1
  SQLCHAR buffer[4] = {0};
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(std::string((char*)buffer) == "ABC");

  // And SQLGetData reads column 2
  SQLCHAR col2_buf[100] = {0};
  SQLLEN col2_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, col2_buf, sizeof(col2_buf), &col2_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)col2_buf) == "XY");

  // Then reading column 1 again should start from the beginning (offset reset)
  SQLCHAR full_buffer[100] = {0};
  SQLLEN full_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, full_buffer, sizeof(full_buffer), &full_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)full_buffer) == "ABCDEFGHIJ");
}

// =============================================================================
// Error Conditions (SQLSTATE Diagnostics)
// =============================================================================

TEST_CASE("SQLGetData returns 07006 when data cannot be converted to TargetType.", "[query][get_data]") {
  // Doc: "07006 - Restricted data type attribute violation: The data value of a
  //       column in the result set cannot be converted to the C data type
  //       specified by the argument TargetType."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT TO_BINARY('48656C6C6F', 'HEX') AS value");

  // When SQLGetData tries to convert a non-numeric string to an integer
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then SQLGetData should return SQL_ERROR
  REQUIRE((ret == SQL_ERROR || ret == SQL_SUCCESS_WITH_INFO));

  // And the SQLSTATE should indicate a conversion error
  std::string sqlstate = get_sqlstate(stmt);
  CHECK(sqlstate == "07006");
}

TEST_CASE("SQLGetData returns 07009 when Col_or_Param_Num is 0 and bookmarks are off.", "[query][get_data]") {
  // Doc: "07009 - Invalid descriptor index: The value specified for the argument
  //       Col_or_Param_Num was 0, and the SQL_ATTR_USE_BOOKMARKS statement
  //       attribute was set to SQL_UB_OFF."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // And bookmarks are off (default)
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_USE_BOOKMARKS, (SQLPOINTER)SQL_UB_OFF, 0);
  CHECK_ODBC(ret, stmt);

  // When a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // And SQLGetData is called with column 0
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 0, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then SQLGetData should return SQL_ERROR with SQLSTATE 07009
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "07009");
}

TEST_CASE("SQLGetData returns 07009 when Col_or_Param_Num exceeds result set columns.", "[query][get_data]") {
  // Doc: "07009 - Invalid descriptor index: The value specified for the argument
  //       Col_or_Param_Num was greater than the number of columns in the result set."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLGetData is called with a column number that exceeds the result set
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 100, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then SQLGetData should return SQL_ERROR with SQLSTATE 07009
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "07009");
}

TEST_CASE("SQLGetData returns 24000 when cursor is positioned after end of result set.", "[query][get_data]") {
  // Doc: "24000 - Invalid cursor state: ... the cursor was positioned before the
  //       start of the result set or after the end of the result set."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning one row is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And all rows have been fetched
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);

  // Then SQLGetData should return SQL_ERROR because cursor is past end
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "24000");
}

// =============================================================================
// Data Type Conversions
// =============================================================================

TEST_CASE("SQLGetData supports SQL_C_CHAR retrieval.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'ODBC Test' AS value");

  // When SQLGetData retrieves data as SQL_C_CHAR
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then the string data should be returned
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)buffer) == "ODBC Test");
  CHECK(indicator == 9);
}

TEST_CASE("SQLGetData supports SQL_C_SBIGINT retrieval.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 9223372036854775807 AS value");

  // When SQLGetData retrieves data as SQL_C_SBIGINT
  SQLBIGINT value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);

  // Then the large integer should be returned
  CHECK_ODBC(ret, stmt);
  CHECK(value == 9223372036854775807LL);
  CHECK(indicator == sizeof(SQLBIGINT));
}

TEST_CASE("SQLGetData supports SQL_C_DOUBLE retrieval.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 3.14159 AS value");

  // When SQLGetData retrieves data as SQL_C_DOUBLE
  SQLDOUBLE value = 0.0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &value, sizeof(value), &indicator);

  // Then the double value should be returned
  CHECK_ODBC(ret, stmt);
  CHECK(value == 3.14159);
  CHECK(indicator == sizeof(SQLDOUBLE));
}

TEST_CASE("SQLGetData supports SQL_C_FLOAT retrieval.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 2.5 AS value");

  // When SQLGetData retrieves data as SQL_C_FLOAT
  SQLREAL value = 0.0f;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_FLOAT, &value, sizeof(value), &indicator);

  // Then the float value should be returned
  CHECK_ODBC(ret, stmt);
  CHECK(value == 2.5f);
  CHECK(indicator == sizeof(SQLREAL));
}

TEST_CASE("SQLGetData supports SQL_C_SHORT retrieval.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 127 AS value");

  // When SQLGetData retrieves data as SQL_C_SHORT
  SQLSMALLINT value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SHORT, &value, sizeof(value), &indicator);

  // Then the short value should be returned
  CHECK_ODBC(ret, stmt);
  CHECK(value == 127);
  CHECK(indicator == sizeof(SQLSMALLINT));
}

TEST_CASE("SQLGetData supports SQL_C_BIT retrieval.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT TRUE AS value");

  // When SQLGetData retrieves data as SQL_C_BIT
  SQLCHAR value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BIT, &value, sizeof(value), &indicator);

  // Then the boolean value should be returned
  CHECK_ODBC(ret, stmt);
  CHECK(value == 1);
}

TEST_CASE("SQLGetData supports SQL_C_TYPE_DATE retrieval.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT DATE '2025-01-15' AS value");

  // When SQLGetData retrieves data as SQL_C_TYPE_DATE
  SQL_DATE_STRUCT date_value = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_TYPE_DATE, &date_value, sizeof(date_value), &indicator);

  // Then the date components should be returned
  CHECK_ODBC(ret, stmt);
  CHECK(date_value.year == 2025);
  CHECK(date_value.month == 1);
  CHECK(date_value.day == 15);
}

TEST_CASE("SQLGetData supports SQL_C_TYPE_TIMESTAMP retrieval.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT TIMESTAMP '2025-06-15 10:30:45' AS value");

  // When SQLGetData retrieves data as SQL_C_TYPE_TIMESTAMP
  SQL_TIMESTAMP_STRUCT ts_value = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_TYPE_TIMESTAMP, &ts_value, sizeof(ts_value), &indicator);

  // Then the timestamp components should be returned
  CHECK_ODBC(ret, stmt);
  CHECK(ts_value.year == 2025);
  CHECK(ts_value.month == 6);
  CHECK(ts_value.day == 15);
  CHECK(ts_value.hour == 10);
  CHECK(ts_value.minute == 30);
  CHECK(ts_value.second == 45);
}

TEST_CASE("SQLGetData supports SQL_C_BINARY retrieval.", "[query][get_data]") {
  // Doc: "TargetType [Input] The type identifier of the C data type of the
  //       *TargetValuePtr buffer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT TO_BINARY('48656C6C6F', 'HEX') AS value");

  // When SQLGetData retrieves data as SQL_C_BINARY
  SQLCHAR buffer[100] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then the binary data should be returned
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == 5);  // "Hello" is 5 bytes
  CHECK(memcmp(buffer, "Hello", 5) == 0);
}

// =============================================================================
// Numeric Value Out of Range
// =============================================================================

TEST_CASE("SQLGetData returns 22003 when numeric value is out of range for target type.", "[query][get_data]") {
  // Doc: "22003 - Numeric value out of range: Returning the numeric value (as
  //       numeric or string) for the column would have caused the whole (as
  //       opposed to fractional) part of the number to be truncated."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 9999999999 AS value");

  // When SQLGetData tries to retrieve a large number into a small integer type
  SQLSMALLINT value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SHORT, &value, sizeof(value), &indicator);

  // Then SQLGetData should return an error indicating numeric overflow
  REQUIRE((ret == SQL_ERROR || ret == SQL_SUCCESS_WITH_INFO));
  std::string sqlstate = get_sqlstate(stmt);
  CHECK((sqlstate == "22003" || sqlstate == "22018" || sqlstate == "01S07"));
}

// =============================================================================
// Fractional Truncation
// =============================================================================

TEST_CASE("SQLGetData returns 01S07 when fractional part is truncated.", "[query][get_data]") {
  // Doc: "01S07 - Fractional truncation: The data returned for one or more columns
  //       was truncated. For numeric data types, the fractional part of the number
  //       was truncated."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 3.14159 AS value");

  // When SQLGetData converts a decimal to an integer (truncating fractional part)
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);

  // Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01S07
  CHECK((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  if (ret == SQL_SUCCESS_WITH_INFO) {
    CHECK(get_sqlstate(stmt) == "01S07");
  }

  // And the integer part should be preserved
  CHECK(value == 3);
}

// =============================================================================
// SQL_SUCCESS_WITH_INFO and SQLSTATE 01004 (Data Truncated)
// =============================================================================

TEST_CASE("SQLGetData returns 01004 when string data is truncated.", "[query][get_data]") {
  // Doc: "01004 - String data, right truncated: Not all of the data for the
  //       specified column, Col_or_Param_Num, could be retrieved in a single
  //       call to the function."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'This is a long string' AS value");

  // When SQLGetData is called with a small buffer
  SQLCHAR buffer[6] = {0};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");

  // And the buffer should contain the truncated string
  CHECK(std::string((char*)buffer) == "This ");
}

TEST_CASE("SQLGetData returns SQL_SUCCESS on the last part of truncated data.", "[query][get_data]") {
  // Doc: "When it returns the last part of the data, SQLGetData returns SQL_SUCCESS.
  //       Neither SQL_NO_TOTAL nor zero can be returned on the last valid call to
  //       retrieve data from a column."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-variable-length-data-in-parts

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'AB' AS value");

  // When SQLGetData is called with a 2-byte buffer (1 char + null)
  SQLCHAR buffer[2] = {0};
  SQLLEN indicator = 0;

  // Then the first call returns SQL_SUCCESS_WITH_INFO (truncated)
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(std::string((char*)buffer) == "A");

  // And the second call returns the last part with SQL_SUCCESS
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(std::string((char*)buffer) == "B");

  // And subsequent call returns SQL_NO_DATA
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_NO_DATA);
}

// =============================================================================
// SQLGetData with Block Cursors (SQL_GD_BLOCK)
// =============================================================================

TEST_CASE("SQLGetData cannot be called when SQL_ATTR_ROW_ARRAY_SIZE is set and SQL_GD_BLOCK is not supported.") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "If no extensions are supported, SQLGetData cannot be called if the
  //       rowset size is greater than 1."
  // Doc: "SQL_GD_BLOCK. If this option is returned by SQLGetInfo for the
  //       SQL_GETDATA_EXTENSIONS InfoType, the driver supports calls to SQLGetData
  //       when the rowset size is greater than 1."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#using-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLSetStmtAttr is called to set the row array size
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)10, 0);
  CHECK_ODBC(ret, stmt);

  // And SQL_GD_BLOCK is not supported
  SQLINTEGER getdata_extensions = 0;
  SQLSMALLINT length = 0;
  ret =
      SQLGetInfo(conn.handleWrapper().getHandle(), SQL_GETDATA_EXTENSIONS, (SQLPOINTER)&getdata_extensions, 0, &length);
  CHECK_ODBC(ret, conn.handleWrapper());
  REQUIRE((getdata_extensions & SQL_GD_BLOCK) == 0);
  REQUIRE(length == sizeof(SQLINTEGER));

  // And SQLExecDirect is called to execute the query that returns 10 rows
  ret = SQLExecDirect(stmt.getHandle(),
                      (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 10)) v ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLFetch is called to fetch the rows
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData should return SQL_ERROR with SQLSTATE HY109 (Invalid cursor position)
  SQLBIGINT result = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, 0, &indicator);
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY109");
}

// =============================================================================
// SQLGetData for Multiple Rows
// =============================================================================

TEST_CASE("SQLGetData retrieves correct data on each successive row after SQLFetch.", "[query][get_data]") {
  // Doc: "SQLGetData returns the data in a specified column. SQLGetData can be
  //       called only after one or more rows have been fetched from the result
  //       set by SQLFetch, SQLFetchScroll, or SQLExtendedFetch."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning multiple rows is executed
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData should return the correct value for each row
  for (int i = 0; i < 5; i++) {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    SQLBIGINT value = 0;
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(value == i);
    CHECK(indicator == sizeof(SQLBIGINT));
  }

  // And SQLFetch returns SQL_NO_DATA after all rows
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

// =============================================================================
// SQLGetData with SQL_ARD_TYPE
// =============================================================================

TEST_CASE("SQLGetData with SQL_ARD_TYPE uses the type from the ARD descriptor.", "[query][get_data]") {
  // Doc: "If TargetType is SQL_ARD_TYPE, the driver uses the type identifier
  //       specified in the SQL_DESC_CONCISE_TYPE field of the ARD."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When the ARD is configured with SQL_C_SBIGINT type
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(ard, 1, SQL_DESC_TYPE, (SQLPOINTER)SQL_C_SBIGINT, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ard, 1, SQL_DESC_CONCISE_TYPE, (SQLPOINTER)SQL_C_SBIGINT, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData with SQL_ARD_TYPE should use the ARD's type
  SQLBIGINT value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_ARD_TYPE, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 42);
  CHECK(indicator == sizeof(SQLBIGINT));
}

// =============================================================================
// SQLGetData with SQL_ARD_TYPE and unmodified ARD returns error
// =============================================================================

TEST_CASE("SQLGetData with SQL_ARD_TYPE returns 07009 error when ARD is unmodified.", "[query][get_data]") {
  // Doc: "If TargetType is SQL_ARD_TYPE, the driver uses the type identifier
  //       specified in the SQL_DESC_CONCISE_TYPE field of the ARD."
  // When no descriptor fields have been set, SQL_DESC_CONCISE_TYPE defaults to
  // SQL_C_DEFAULT, but the current implementation returns error 07009 (Invalid descriptor index).
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query is executed and fetched without modifying the ARD
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData with SQL_ARD_TYPE should return SQL_ERROR with SQLSTATE 07009
  SQLCHAR buffer[64] = {0};
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_ARD_TYPE, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "07009");
}

// =============================================================================
// SQLGetData does not interact directly with descriptor fields
// =============================================================================

TEST_CASE("SQLGetData does not modify ARD descriptor fields.", "[query][get_data]") {
  // Doc: "SQLGetData does not interact directly with any descriptor fields."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#descriptors-and-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When the ARD descriptor count is checked before SQLGetData
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT count_before = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count_before, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // And a query is executed, fetched, and data retrieved via SQLGetData
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);

  // Then the ARD descriptor count should remain unchanged
  SQLSMALLINT count_after = -1;
  ret = SQLGetDescField(ard, 0, SQL_DESC_COUNT, &count_after, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(count_after == count_before);
}

// =============================================================================
// SQLGetData after SQLFreeStmt SQL_CLOSE
// =============================================================================

TEST_CASE("SQLGetData works after SQLFreeStmt SQL_CLOSE and re-execute.", "[query][get_data]") {
  // Doc: "SQLGetData can be called only after one or more rows have been fetched
  //       from the result set..."
  // (Verifies SQLGetData works correctly on a new result set after cursor close)
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a first query is executed, fetched, and data retrieved
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 10 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 10);

  // And the cursor is closed
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  CHECK_ODBC(ret, stmt);

  // And a new query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 20 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData should work on the new result set
  value = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 20);
}

// =============================================================================
// SQLGetData with NULL data in various types
// =============================================================================

TEST_CASE("SQLGetData returns SQL_NULL_DATA for NULL value regardless of TargetType.", "[query][get_data]") {
  // Doc: "Sets *StrLen_or_IndPtr to SQL_NULL_DATA if the data is NULL."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT NULL AS value");

  // When SQLGetData is called with SQL_C_CHAR for a NULL column
  SQLCHAR char_buf[100] = {0};
  SQLLEN char_ind = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, char_buf, sizeof(char_buf), &char_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(char_ind == SQL_NULL_DATA);

  // Close and re-execute to test another type
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  CHECK_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT NULL AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // And when called with SQL_C_LONG for a NULL column
  SQLINTEGER int_value = 999;
  SQLLEN int_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &int_value, sizeof(int_value), &int_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(int_ind == SQL_NULL_DATA);
}

// =============================================================================
// SQLGetData on same column returns same data before next fetch
// =============================================================================

TEST_CASE("SQLGetData retrieves same data for same column on same row with SQL_GD_ANY_ORDER.", "[query][get_data]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "SQL_GD_ANY_ORDER. If this option is returned, SQLGetData can be called
  //       for unbound columns in any order."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#using-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;

  // And SQL_GD_ANY_ORDER is supported
  SQLINTEGER extensions = 0;
  SQLSMALLINT length = 0;
  SQLRETURN ret =
      SQLGetInfo(conn.handleWrapper().getHandle(), SQL_GETDATA_EXTENSIONS, &extensions, sizeof(extensions), &length);
  CHECK_ODBC(ret, conn.handleWrapper());
  REQUIRE((extensions & SQL_GD_ANY_ORDER) == SQL_GD_ANY_ORDER);

  auto stmt = conn.execute_fetch("SELECT 42 AS col1, 'hello' AS col2");

  // When SQLGetData retrieves column 2 first, then column 1
  SQLCHAR col2[100] = {0};
  SQLLEN col2_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, col2, sizeof(col2), &col2_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)col2) == "hello");

  SQLINTEGER col1 = 0;
  SQLLEN col1_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &col1, sizeof(col1), &col1_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(col1 == 42);
}

// =============================================================================
// SQLGetData with SQL_GD_BOUND
// =============================================================================

TEST_CASE("SQLGetData on a bound column when SQL_GD_BOUND is supported.", "[query][get_data]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "SQL_GD_BOUND. If this option is returned, SQLGetData can be called for
  //       bound columns as well as unbound columns."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#using-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;

  // And SQL_GD_BOUND is supported
  SQLINTEGER extensions = 0;
  SQLSMALLINT length = 0;
  SQLRETURN ret =
      SQLGetInfo(conn.handleWrapper().getHandle(), SQL_GETDATA_EXTENSIONS, &extensions, sizeof(extensions), &length);
  CHECK_ODBC(ret, conn.handleWrapper());
  REQUIRE((extensions & SQL_GD_BOUND) == SQL_GD_BOUND);

  auto stmt = conn.createStatement();

  // When column 1 is bound
  SQLINTEGER bound_value = 0;
  SQLLEN bound_ind = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &bound_value, sizeof(bound_value), &bound_ind);
  CHECK_ODBC(ret, stmt);

  // And a query is executed and fetched
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then SQLGetData should also be able to retrieve the bound column
  SQLINTEGER getdata_value = 0;
  SQLLEN getdata_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &getdata_value, sizeof(getdata_value), &getdata_ind);
  CHECK_ODBC(ret, stmt);
  CHECK(getdata_value == 42);

  // And the bound buffer should also have the data from SQLFetch
  CHECK(bound_value == 42);
}

// =============================================================================
// SQLGetData mixed with SQLFetch and SQLFetchScroll
// =============================================================================

TEST_CASE("SQLGetData can be mixed with SQLFetch and SQLFetchScroll calls.", "[query][get_data]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Doc: "SQLGetData can be called only after one or more rows have been fetched
  //       from the result set by SQLFetch, SQLFetchScroll, or SQLExtendedFetch."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query returning 4 rows is executed
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 4)) ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then alternating between SQLFetch and SQLFetchScroll should work with SQLGetData
  SQLBIGINT value = 0;
  SQLLEN indicator = 0;

  // Row 0 via SQLFetch
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 0);

  // Row 1 via SQLFetchScroll
  ret = SQLFetchScroll(stmt.getHandle(), SQL_FETCH_NEXT, 0);
  CHECK_ODBC(ret, stmt);
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 1);

  // Row 2 via SQLFetch
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 2);

  // Row 3 via SQLFetchScroll
  ret = SQLFetchScroll(stmt.getHandle(), SQL_FETCH_NEXT, 0);
  CHECK_ODBC(ret, stmt);
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(value == 3);
}

// =============================================================================
// SQLGetData with empty string
// =============================================================================

TEST_CASE("SQLGetData returns empty string with indicator 0 for empty string column.", "[query][get_data]") {
  // Doc: "Places the length of the data in *StrLen_or_IndPtr."
  // (For empty string, length should be 0, not SQL_NULL_DATA)
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT '' AS value");

  // When SQLGetData retrieves an empty string
  SQLCHAR buffer[100] = {'X'};  // Initialize with non-null to detect empty string
  SQLLEN indicator = -1;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then the data should be an empty string
  CHECK_ODBC(ret, stmt);
  CHECK(std::string((char*)buffer) == "");
  CHECK(indicator == 0);
}

// =============================================================================
// SQLGetData with BufferLength exactly 1 for character data
// =============================================================================

TEST_CASE("SQLGetData with BufferLength 1 for character data returns only null terminator.", "[query][get_data]") {
  // Doc: "If the data buffer supplied is too small to hold the null-termination
  //       character, SQLGetData returns SQL_SUCCESS_WITH_INFO and SQLSTATE 01004."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 'ABC' AS value");

  // When SQLGetData is called with BufferLength 1 (only room for null terminator)
  SQLCHAR buffer[1] = {'X'};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQLGetData should return SQL_SUCCESS_WITH_INFO with truncation
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");

  // And the buffer should contain only the null terminator
  CHECK(buffer[0] == '\0');

  // And the indicator should show the full data length
  CHECK((indicator == SQL_NO_TOTAL || indicator == 3));
}

// =============================================================================
// SQLGetData with integer column in various C types
// =============================================================================

TEST_CASE("SQLGetData converts same integer to multiple C types.", "[query][get_data]") {
  // Doc: "Converts the data to the type specified in TargetType."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function#retrieving-data-with-sqlgetdata

  // Given Snowflake client is logged in
  Connection conn;

  // Test SQL_C_LONG
  {
    auto stmt = conn.execute_fetch("SELECT 42 AS value");
    SQLINTEGER value = 0;
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(value == 42);
  }

  // Test SQL_C_SBIGINT
  {
    auto stmt = conn.execute_fetch("SELECT 42 AS value");
    SQLBIGINT value = 0;
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(value == 42);
  }

  // Test SQL_C_DOUBLE
  {
    auto stmt = conn.execute_fetch("SELECT 42 AS value");
    SQLDOUBLE value = 0.0;
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(value == 42.0);
  }

  // Test SQL_C_CHAR
  {
    auto stmt = conn.execute_fetch("SELECT 42 AS value");
    SQLCHAR buffer[100] = {0};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(std::string((char*)buffer) == "42");
  }
}
