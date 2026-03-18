@odbc
Feature: ODBC SQLGetData function behavior
  # Tests for SQLGetData based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlgetdata-function

  @odbc_e2e
  Scenario: SQLGetData retrieves data for a single column after SQLFetch.
    Given Snowflake client is logged in
    When a query returning data is executed
    And SQLFetch is called to position the cursor
    Then SQLGetData should retrieve the data for the column

  @odbc_e2e
  Scenario: SQLGetData can be called multiple times to retrieve variable-length data in parts.
    Given Snowflake client is logged in
    When a query returning a long string is executed
    And SQLFetch is called to position the cursor
    Then SQLGetData can be called multiple times with small buffers to retrieve data in parts
    And subsequent calls should retrieve the remaining data
    And final call should retrieve the last part

  @odbc_e2e
  Scenario: SQLGetData can only be called after rows have been fetched.
    Given Snowflake client is logged in
    When a query is executed but SQLFetch is NOT called
    Then SQLGetData should return SQL_ERROR with SQLSTATE 24000 (Invalid cursor state)

  @odbc_e2e
  Scenario: SQLGetData can retrieve data after SQLFetchScroll.
    Given Snowflake client is logged in
    When a query returning multiple rows is executed
    And SQLFetchScroll is called to fetch the first row
    Then SQLGetData should retrieve data for the current row

  @odbc_e2e
  Scenario: SQLGetData retrieves data for multiple columns.
    Given Snowflake client is logged in
    When a query with multiple columns is executed and fetched
    Then SQLGetData should retrieve each column individually

  @odbc_e2e
  Scenario: SQLGetData returns SQL_SUCCESS on successful retrieval.
    Given Snowflake client is logged in
    When SQLGetData is called with valid parameters
    Then SQLGetData should return SQL_SUCCESS

  @odbc_e2e
  Scenario: SQLGetData returns SQL_INVALID_HANDLE for invalid statement handle.
    Given an invalid statement handle
    When SQLGetData is called with the invalid handle
    Then SQLGetData should return SQL_INVALID_HANDLE

  @odbc_e2e
  Scenario: SQLGetData converts data to the specified TargetType.
    Given Snowflake client is logged in
    When SQLGetData specifies SQL_C_CHAR to convert integer to string
    Then the data should be converted to string representation

  @odbc_e2e
  Scenario: SQLGetData with SQL_C_DEFAULT selects default C type based on SQL type.
    Given Snowflake client is logged in
    And we determine the SQL data type of the column
    When SQLGetData is called with SQL_C_DEFAULT
    Then the driver should select a default C type and return data

  @odbc_e2e
  Scenario: SQLGetData overrides SQLBindCol type when a different TargetType is specified.
    Given Snowflake client is logged in
    When a column is bound as SQL_C_LONG via SQLBindCol
    And a query is executed and fetched
    Then SQLGetData with SQL_C_CHAR should override the bound type and return a string

  @odbc_e2e
  Scenario: SQLGetData returns HY009 when TargetValuePtr is null.
    Given Snowflake client is logged in
    When SQLGetData is called with NULL TargetValuePtr
    Then SQLGetData should return SQL_ERROR with SQLSTATE HY009

  @odbc_e2e
  Scenario: SQLGetData counts null terminator when returning character data.
    Given Snowflake client is logged in
    When SQLGetData is called with exactly 6 bytes (5 chars + null)
    Then the full string should be returned with null termination

  @odbc_e2e
  Scenario: SQLGetData truncates character data when buffer is too small.
    Given Snowflake client is logged in
    When SQLGetData is called with a buffer too small for the full string
    Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
    And the buffer should contain truncated data with null termination
    And the indicator should show the full length of the original data

  @odbc_e2e
  Scenario: SQLGetData truncates binary data to BufferLength bytes.
    Given Snowflake client is logged in
    When SQLGetData is called with a small buffer for binary data
    Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
    And the indicator should show SQL_NO_TOTAL
    And the buffer should contain the first 3 bytes

  @odbc_e2e
  Scenario: SQLGetData ignores BufferLength for fixed-length data types.
    Given Snowflake client is logged in
    When SQLGetData is called with BufferLength 0 for a fixed-length type
    Then the driver should ignore BufferLength and return the data

  @odbc_e2e
  Scenario: SQLGetData returns HY090 when BufferLength is less than 0.
    Given Snowflake client is logged in
    When SQLGetData is called with negative BufferLength
    Then SQLGetData should return SQL_ERROR with SQLSTATE HY090

  @odbc_e2e
  Scenario: SQLGetData does not return error when BufferLength is 0.
    Given Snowflake client is logged in
    When SQLGetData is called with BufferLength 0 for fixed-length type
    Then SQLGetData should not return SQL_ERROR for BufferLength 0

  @odbc_e2e
  Scenario: SQLGetData returns data length in StrLen_or_IndPtr for character data.
    Given Snowflake client is logged in
    When SQLGetData is called with a large buffer
    Then the indicator should contain the data length (not including null terminator)

  @odbc_e2e
  Scenario: SQLGetData returns type size in StrLen_or_IndPtr for fixed-length data.
    Given Snowflake client is logged in
    When SQLGetData retrieves fixed-length data
    Then the indicator should contain the size of the C type

  @odbc_e2e
  Scenario: SQLGetData returns SQL_NULL_DATA in StrLen_or_IndPtr for NULL values.
    Given Snowflake client is logged in
    When SQLGetData is called for a NULL column
    Then the indicator should be SQL_NULL_DATA

  @odbc_e2e
  Scenario: SQLGetData returns 22002 when NULL data fetched without indicator pointer.
    Given Snowflake client is logged in
    When SQLGetData is called with NULL StrLen_or_IndPtr for a NULL column
    Then SQLGetData should return SQL_ERROR with SQLSTATE 22002

  @odbc_e2e
  Scenario: SQLGetData with null StrLen_or_IndPtr succeeds for non-null data.
    Given Snowflake client is logged in
    When SQLGetData is called without an indicator pointer
    Then SQLGetData should succeed for non-null data

  @odbc_e2e
  Scenario: SQLGetData retrieves character data in parts with multiple calls.
    Given Snowflake client is logged in
    When SQLGetData is called with a 4-byte buffer (3 chars + null)
    Then the first call should return "ABC" with SQL_SUCCESS_WITH_INFO
    And the second call should return "DEF" with SQL_SUCCESS_WITH_INFO
    And the third call should return "GHI" with SQL_SUCCESS_WITH_INFO
    And the fourth call should return "J" with SQL_SUCCESS (last part)
    And the complete string should be reconstructed

  @odbc_e2e
  Scenario: SQLGetData returns SQL_NO_DATA after all data for a column has been retrieved.
    Given Snowflake client is logged in
    When SQLGetData retrieves all data in the first call
    Then subsequent SQLGetData on the same column should return SQL_NO_DATA

  @odbc_e2e
  Scenario: SQLGetData returns SQL_NO_DATA for fixed-length data after first successful call.
    Given Snowflake client is logged in
    When SQLGetData retrieves the fixed-length integer
    Then subsequent calls should return SQL_NO_DATA

  @odbc_e2e
  Scenario: SQLGetData can be called on unbound columns after bound columns.
    Given Snowflake client is logged in
    When column 1 is bound
    And a query with two columns is executed and fetched
    Then the bound column should have data
    And SQLGetData should retrieve the unbound column (column 2 > last bound column 1)

  @odbc_e2e
  Scenario: SQLGetData retrieves data in increasing column number order.
    Given Snowflake client is logged in
    When SQLGetData is called in increasing column order
    Then each column should return its correct value

  @odbc_e2e
  Scenario: SQLGetInfo returns SQL_GETDATA_EXTENSIONS bitmask.
    Given Snowflake client is logged in
    When SQLGetInfo is called for SQL_GETDATA_EXTENSIONS
    Then the call should succeed and return a valid bitmask
    Then the bitmask should report SQL_GD_ANY_COLUMN, SQL_GD_ANY_ORDER, and SQL_GD_BOUND

  @odbc_e2e
  Scenario: SQLGetData for a different column resets prior column offset.
    Given Snowflake client is logged in
    And SQLGetInfo indicates SQL_GD_ANY_ORDER is supported
    When SQLGetData partially reads column 1
    And SQLGetData reads column 2
    Then reading column 1 again should start from the beginning (offset reset)

  @odbc_e2e
  Scenario: SQLGetData returns 07006 when data cannot be converted to TargetType.
    Given Snowflake client is logged in
    When SQLGetData tries to convert a non-numeric string to an integer
    Then SQLGetData should return SQL_ERROR
    And the SQLSTATE should indicate a conversion error

  @odbc_e2e
  Scenario: SQLGetData returns 07009 when Col_or_Param_Num is 0 and bookmarks are off.
    Given Snowflake client is logged in
    And bookmarks are off (default)
    When a query is executed and fetched
    And SQLGetData is called with column 0
    Then SQLGetData should return SQL_ERROR with SQLSTATE 07009

  @odbc_e2e
  Scenario: SQLGetData returns 07009 when Col_or_Param_Num exceeds result set columns.
    Given Snowflake client is logged in
    When SQLGetData is called with a column number that exceeds the result set
    Then SQLGetData should return SQL_ERROR with SQLSTATE 07009

  @odbc_e2e
  Scenario: SQLGetData returns 24000 when cursor is positioned after end of result set.
    Given Snowflake client is logged in
    When a query returning one row is executed
    And all rows have been fetched
    Then SQLGetData should return SQL_ERROR because cursor is past end

  @odbc_e2e
  Scenario: SQLGetData supports SQL_C_CHAR retrieval.
    Given Snowflake client is logged in
    When SQLGetData retrieves data as SQL_C_CHAR
    Then the string data should be returned

  @odbc_e2e
  Scenario: SQLGetData supports SQL_C_SBIGINT retrieval.
    Given Snowflake client is logged in
    When SQLGetData retrieves data as SQL_C_SBIGINT
    Then the large integer should be returned

  @odbc_e2e
  Scenario: SQLGetData supports SQL_C_DOUBLE retrieval.
    Given Snowflake client is logged in
    When SQLGetData retrieves data as SQL_C_DOUBLE
    Then the double value should be returned

  @odbc_e2e
  Scenario: SQLGetData supports SQL_C_FLOAT retrieval.
    Given Snowflake client is logged in
    When SQLGetData retrieves data as SQL_C_FLOAT
    Then the float value should be returned

  @odbc_e2e
  Scenario: SQLGetData supports SQL_C_SHORT retrieval.
    Given Snowflake client is logged in
    When SQLGetData retrieves data as SQL_C_SHORT
    Then the short value should be returned

  @odbc_e2e
  Scenario: SQLGetData supports SQL_C_BIT retrieval.
    Given Snowflake client is logged in
    When SQLGetData retrieves data as SQL_C_BIT
    Then the boolean value should be returned

  @odbc_e2e
  Scenario: SQLGetData supports SQL_C_TYPE_DATE retrieval.
    Given Snowflake client is logged in
    When SQLGetData retrieves data as SQL_C_TYPE_DATE
    Then the date components should be returned

  @odbc_e2e
  Scenario: SQLGetData supports SQL_C_TYPE_TIMESTAMP retrieval.
    Given Snowflake client is logged in
    When SQLGetData retrieves data as SQL_C_TYPE_TIMESTAMP
    Then the timestamp components should be returned

  @odbc_e2e
  Scenario: SQLGetData supports SQL_C_BINARY retrieval.
    Given Snowflake client is logged in
    When SQLGetData retrieves data as SQL_C_BINARY
    Then the binary data should be returned

  @odbc_e2e
  Scenario: SQLGetData returns 22003 when numeric value is out of range for target type.
    Given Snowflake client is logged in
    When SQLGetData tries to retrieve a large number into a small integer type
    Then SQLGetData should return an error indicating numeric overflow

  @odbc_e2e
  Scenario: SQLGetData returns 01S07 when fractional part is truncated.
    Given Snowflake client is logged in
    When SQLGetData converts a decimal to an integer (truncating fractional part)
    Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01S07
    And the integer part should be preserved

  @odbc_e2e
  Scenario: SQLGetData returns 01004 when string data is truncated.
    Given Snowflake client is logged in
    When SQLGetData is called with a small buffer
    Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
    And the buffer should contain the truncated string

  @odbc_e2e
  Scenario: SQLGetData returns SQL_SUCCESS on the last part of truncated data.
    Given Snowflake client is logged in
    When SQLGetData is called with a 2-byte buffer (1 char + null)
    Then the first call returns SQL_SUCCESS_WITH_INFO (truncated)
    And the second call returns the last part with SQL_SUCCESS
    And subsequent call returns SQL_NO_DATA

  @odbc_e2e
  Scenario: SQLGetData cannot be called when SQL_ATTR_ROW_ARRAY_SIZE is set and SQL_GD_BLOCK is not supported.
    Given Snowflake client is logged in
    When SQLSetStmtAttr is called to set the row array size
    And SQL_GD_BLOCK is not supported
    And SQLExecDirect is called to execute the query that returns 10 rows
    And SQLFetch is called to fetch the rows
    Then SQLGetData should return SQL_ERROR with SQLSTATE HY109 (Invalid cursor position)

  @odbc_e2e
  Scenario: SQLGetData retrieves correct data on each successive row after SQLFetch.
    Given Snowflake client is logged in
    When a query returning multiple rows is executed
    Then SQLGetData should return the correct value for each row
    And SQLFetch returns SQL_NO_DATA after all rows

  @odbc_e2e
  Scenario: SQLGetData with SQL_ARD_TYPE uses the type from the ARD descriptor.
    Given Snowflake client is logged in
    When the ARD is configured with SQL_C_SBIGINT type
    And a query is executed and fetched
    Then SQLGetData with SQL_ARD_TYPE should use the ARD's type

  @odbc_e2e
  Scenario: SQLGetData with SQL_ARD_TYPE returns 07009 error when ARD is unmodified.
    Given Snowflake client is logged in
    When a query is executed and fetched without modifying the ARD
    Then SQLGetData with SQL_ARD_TYPE should return SQL_ERROR with SQLSTATE 07009

  @odbc_e2e
  Scenario: SQLGetData does not modify ARD descriptor fields.
    Given Snowflake client is logged in
    When the ARD descriptor count is checked before SQLGetData
    And a query is executed, fetched, and data retrieved via SQLGetData
    Then the ARD descriptor count should remain unchanged

  @odbc_e2e
  Scenario: SQLGetData works after SQLFreeStmt SQL_CLOSE and re-execute.
    Given Snowflake client is logged in
    When a first query is executed, fetched, and data retrieved
    And the cursor is closed
    And a new query is executed and fetched
    Then SQLGetData should work on the new result set

  @odbc_e2e
  Scenario: SQLGetData returns SQL_NULL_DATA for NULL value regardless of TargetType.
    Given Snowflake client is logged in
    When SQLGetData is called with SQL_C_CHAR for a NULL column
    And when called with SQL_C_LONG for a NULL column
    Then both should return SQL_NULL_DATA indicator

  @odbc_e2e
  Scenario: SQLGetData retrieves same data for same column on same row with SQL_GD_ANY_ORDER.
    Given Snowflake client is logged in
    And SQL_GD_ANY_ORDER is supported
    When SQLGetData retrieves column 2 first, then column 1
    Then both columns should return their correct values

  @odbc_e2e
  Scenario: SQLGetData on a bound column when SQL_GD_BOUND is supported.
    Given Snowflake client is logged in
    And SQL_GD_BOUND is supported
    When column 1 is bound
    And a query is executed and fetched
    Then SQLGetData should also be able to retrieve the bound column
    And the bound buffer should also have the data from SQLFetch

  @odbc_e2e
  Scenario: SQLGetData can be mixed with SQLFetch and SQLFetchScroll calls.
    Given Snowflake client is logged in
    When a query returning 4 rows is executed
    Then alternating between SQLFetch and SQLFetchScroll should work with SQLGetData

  @odbc_e2e
  Scenario: SQLGetData returns empty string with indicator 0 for empty string column.
    Given Snowflake client is logged in
    When SQLGetData retrieves an empty string
    Then the data should be an empty string

  @odbc_e2e
  Scenario: SQLGetData with BufferLength 1 for character data returns only null terminator.
    Given Snowflake client is logged in
    When SQLGetData is called with BufferLength 1 (only room for null terminator)
    Then SQLGetData should return SQL_SUCCESS_WITH_INFO with truncation
    And the buffer should contain only the null terminator
    And the indicator should show the full data length

  @odbc_e2e
  Scenario: SQLGetData converts same integer to multiple C types.
    Given Snowflake client is logged in
    When SQLGetData retrieves the same integer value using different C types
    Then each C type conversion should return the correct value

  @odbc_e2e
  Scenario: SQLGetData with BufferLength 0 for SQL_C_CHAR returns data length without writing data.
    Given Snowflake client is logged in
    When SQLGetData is called with BufferLength = 0 for SQL_C_CHAR
    Then the call should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
    And the indicator should contain the full data length

  @odbc_e2e
  Scenario: SQLGetData with BufferLength 0 for SQL_C_BINARY returns data length without writing data.
    Given Snowflake client is logged in
    When SQLGetData is called with BufferLength = 0 for SQL_C_BINARY
    Then the call should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
    And the indicator should contain the full data length

  @odbc_e2e
  Scenario: SQLGetData retrieves wide string data in parts with SQL_C_WCHAR.
    Given Snowflake client is logged in
    When SQLGetData is called with SQL_C_WCHAR and a small buffer
    Then the first call should return 3 wide chars with SQL_SUCCESS_WITH_INFO
    And the second call should return the next 3 wide chars
    And the third call should return the next 3 wide chars
    And the fourth call should return the last character with SQL_SUCCESS
    And the complete string should be reconstructed correctly

  @odbc_e2e
  Scenario: SQLGetData retrieves binary data in parts with multiple SQL_C_BINARY calls.
    Given Snowflake client is logged in
    When SQLGetData is called with SQL_C_BINARY and a 4-byte buffer
    Then the first call should return the first 4 bytes with SQL_SUCCESS_WITH_INFO
    And the second call should return the next 4 bytes
    And the third call should return the last 2 bytes with SQL_SUCCESS
    And the complete binary data should be reconstructed

  @odbc_e2e
  Scenario: SQLGetData returns SQL_NULL_DATA for NULL double column.
    Given Snowflake client is logged in
    When SQLGetData is called with SQL_C_DOUBLE for a NULL FLOAT column
    Then the indicator should be SQL_NULL_DATA and the call should succeed

  @odbc_e2e
  Scenario: SQLGetData returns SQL_NULL_DATA for NULL binary column.
    Given Snowflake client is logged in
    When SQLGetData is called with SQL_C_BINARY for a NULL BINARY column
    Then the indicator should be SQL_NULL_DATA and the call should succeed

  @odbc_e2e
  Scenario: SQLGetData returns SQL_NULL_DATA for NULL wide string column.
    Given Snowflake client is logged in
    When SQLGetData is called with SQL_C_WCHAR for a NULL column
    Then the indicator should be SQL_NULL_DATA and the call should succeed

  @odbc_e2e
  Scenario: SQLGetData returns HY010 when called on a statement with no executed query.
    Given Snowflake client is logged in
    When SQLGetData is called without executing any query first
    Then SQLGetData should return SQL_ERROR with SQLSTATE HY010

  @odbc_e2e
  Scenario: SQLGetData on a lower column number after partial read resets the offset.
    Given Snowflake client is logged in
    And SQL_GD_ANY_ORDER is supported
    When SQLGetData partially reads column 3
    And SQLGetData is called on column 1 (lower number than previous call on column 3)
    Then reading column 3 again should start from the beginning (offset was reset)
