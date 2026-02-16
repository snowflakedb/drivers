@odbc
Feature: ODBC SQLBindCol function behavior
  # Tests for SQLBindCol based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindcol-function

  @odbc_e2e
  Scenario: SQLBindCol binds a column and SQLFetch returns data in bound buffer.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a query
    And SQLBindCol is called to bind column 1 to a buffer
    Then SQLFetch should populate the bound buffer with the column data

  @odbc_e2e
  Scenario: SQLBindCol returns SQL_SUCCESS on successful binding.
    Given Snowflake client is logged in
    When SQLBindCol is called with valid parameters
    Then SQLBindCol should return SQL_SUCCESS

  @odbc_e2e
  Scenario: SQLBindCol binds multiple columns in a result set.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a query with multiple columns
    And SQLBindCol is called for each column
    Then SQLFetch should return data in all bound buffers

  @odbc_e2e
  Scenario: SQLBindCol uses 1-based column numbering when bookmarks are not used.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a query with two columns
    And SQLBindCol is called with column numbers 1 and 2
    Then SQLFetch should return the correct data for each column

  @odbc_e2e
  Scenario: SQLBindCol can be called before SQLExecDirect.
    Given Snowflake client is logged in
    When SQLBindCol is called before executing a query (deferred binding)
    And SQLExecDirect is called after binding
    Then SQLFetch should return data in the pre-bound buffer

  @odbc_e2e
  Scenario: SQLBindCol can be called after SQLExecDirect.
    Given Snowflake client is logged in
    When SQLExecDirect is called first
    And SQLBindCol is called after executing
    Then SQLFetch should use the binding established after execute

  @odbc_e2e
  Scenario: SQLBindCol unbinds a column when TargetValuePtr is null pointer.
    Given Snowflake client is logged in
    When SQLBindCol is called to bind column 1
    And SQLBindCol is called with null TargetValuePtr to unbind
    And a query is executed and fetched
    Then the previously bound buffer should not be modified (column was unbound)

  @odbc_e2e
  Scenario: SQLBindCol returns SQL_SUCCESS when unbinding an already unbound column.
    Given Snowflake client is logged in
    When SQLBindCol is called with null TargetValuePtr for an unbound column (first unbind)
    And SQLBindCol is called again with null TargetValuePtr for the same column (second unbind)
    Then SQLBindCol should still return SQL_SUCCESS

  @odbc_e2e
  Scenario: SQLFreeStmt with SQL_UNBIND unbinds all columns.
    Given Snowflake client is logged in
    When SQLBindCol is called to bind two columns
    And SQLFreeStmt is called with SQL_UNBIND to unbind all columns
    And a query is executed and fetched
    Then bound buffers should not be modified (all columns were unbound)

  @odbc_e2e
  Scenario: SQLBindCol can unbind data buffer while keeping indicator bound.
    Given Snowflake client is logged in
    When SQLBindCol is called with null TargetValuePtr but valid StrLen_or_IndPtr
    And a query is executed
    And SQLFetch is called to fetch the data
    Then SQLBindCol should return SQL_SUCCESS
    And the indicator should contain the length of the data (even though data buffer is unbound)

  @odbc_e2e
  Scenario: SQLBindCol replaces old binding when called on already bound column.
    Given Snowflake client is logged in
    When SQLBindCol is called to bind column 1 to first buffer
    And SQLBindCol is called again to rebind column 1 to a different buffer
    And a query is executed and fetched
    Then the new buffer should contain the data (old binding was overwritten)
    And the old buffer should remain unchanged

  @odbc_e2e
  Scenario: SQLBindCol rebinding takes effect on next fetch, not the current one.
    Given Snowflake client is logged in
    When a query returning multiple rows is executed
    And column 1 is bound to first buffer
    And SQLFetch is called to fetch the first row
    And column 1 is rebound to a second buffer
    Then fetching the next row should populate the new buffer (not the old one)
    And the old buffer should still hold the first row value

  @odbc_e2e
  Scenario: SQLBindCol can rebind after data has been fetched from result set.
    Given Snowflake client is logged in
    When a query returning two rows is executed
    And first row is fetched without any binding (using SQLGetData)
    And column is bound after first fetch
    Then next fetch should use the new binding

  @odbc_e2e
  Scenario: SQLBindCol counts null terminator when returning character data.
    Given Snowflake client is logged in
    When a query returning a 5-character string is executed
    And SQLBindCol is called with exactly 6 bytes (5 chars + null terminator)
    Then SQLFetch should return the full string with null termination

  @odbc_e2e
  Scenario: SQLBindCol truncates character data when buffer is too small.
    Given Snowflake client is logged in
    When a query returning a long string is executed
    And SQLBindCol is called with a buffer too small to hold the full string
    Then SQLFetch should return SQL_SUCCESS_WITH_INFO (data truncated)
    And the buffer should contain truncated data with null termination
    And the indicator should show the full length of the original data

  @odbc_e2e
  Scenario: SQLBindCol ignores BufferLength for fixed-length data types.
    Given Snowflake client is logged in
    When a query returning an integer is executed
    And SQLBindCol is called with BufferLength set to 0 for a fixed-length type
    Then SQLFetch should succeed because driver ignores BufferLength for fixed-length types

  @odbc_e2e
  Scenario: SQLBindCol returns HY090 when BufferLength is less than 0.
    Given Snowflake client is logged in
    When SQLBindCol is called with negative BufferLength
    Then SQLBindCol should return SQL_ERROR with SQLSTATE HY090

  @odbc_e2e
  Scenario: SQLBindCol does not return error when BufferLength is 0 for non-character type.
    Given Snowflake client is logged in
    When SQLBindCol is called with BufferLength of 0 for a fixed-length type
    Then SQLBindCol should return SQL_SUCCESS (0 is allowed)

  @odbc_e2e
  Scenario: SQLBindCol returns data length in StrLen_or_IndPtr for non-null data.
    Given Snowflake client is logged in
    When a query returning a string is executed
    And SQLBindCol is called with an indicator pointer
    Then SQLFetch should set the indicator to the data length

  @odbc_e2e
  Scenario: SQLBindCol returns SQL_NULL_DATA in StrLen_or_IndPtr for NULL values.
    Given Snowflake client is logged in
    When a query returning NULL is executed
    And SQLBindCol is called with an indicator pointer
    Then SQLFetch should set the indicator to SQL_NULL_DATA

  @odbc_e2e
  Scenario: SQLBindCol with null StrLen_or_IndPtr succeeds for non-null data.
    Given Snowflake client is logged in
    When a query returning a non-null integer is executed
    And SQLBindCol is called without an indicator pointer (NULL)
    Then SQLFetch should succeed for non-null data

  @odbc_e2e
  Scenario: SQLBindCol converts data to the specified TargetType.
    Given Snowflake client is logged in
    When a query returning an integer is executed
    And SQLBindCol specifies SQL_C_CHAR as TargetType (convert integer to string)
    Then SQLFetch should convert the integer to a string representation

  @odbc_e2e
  Scenario: SQLBindCol returns HY003 for invalid TargetType.
    Given Snowflake client is logged in
    When SQLBindCol is called with an invalid TargetType
    Then SQLBindCol should return SQL_ERROR with SQLSTATE HY003

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_DEFAULT as TargetType.
    Given Snowflake client is logged in
    When a query is executed
    And SQLBindCol is called with SQL_C_DEFAULT
    Then SQLBindCol should accept SQL_C_DEFAULT as a valid type

  @odbc_e2e
  Scenario: SQLBindCol updates SQL_DESC_COUNT on the ARD.
    Given Snowflake client is logged in
    When the ARD descriptor is obtained
    And SQL_DESC_COUNT is initially 0
    And SQLBindCol is called to bind column 3
    Then SQL_DESC_COUNT should be updated to 3

  @odbc_e2e
  Scenario: SQLBindCol sets SQL_DESC_COUNT only when increasing.
    Given Snowflake client is logged in
    When SQLBindCol is called to bind column 3
    And the ARD descriptor is obtained
    Then SQL_DESC_COUNT should be 3
    And when SQLBindCol is called to bind column 1 (lower than current count)
    Then SQL_DESC_COUNT should still be 3 (not decreased)

  @odbc_e2e
  Scenario: SQLBindCol decreases SQL_DESC_COUNT when unbinding highest bound column.
    Given Snowflake client is logged in
    When SQLBindCol is called to bind columns 1, 2, and 3
    And the ARD descriptor is obtained
    Then SQL_DESC_COUNT should be 3
    And when the highest bound column (3) is unbound
    Then SQL_DESC_COUNT should decrease to 2 (next highest bound column)

  @odbc_e2e
  Scenario: SQLBindCol sets descriptor fields on the ARD.
    Given Snowflake client is logged in
    When SQLBindCol is called
    And the ARD descriptor is obtained
    Then SQL_DESC_TYPE should match the TargetType
    And SQL_DESC_CONCISE_TYPE should match the TargetType
    And SQL_DESC_OCTET_LENGTH should match BufferLength
    And SQL_DESC_DATA_PTR should match TargetValuePtr
    And SQL_DESC_INDICATOR_PTR should match StrLen_or_IndPtr
    And SQL_DESC_OCTET_LENGTH_PTR should match StrLen_or_IndPtr

  @odbc_e2e
  Scenario: SQLBindCol supports column-wise binding with arrays.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_BIND_TYPE is set to SQL_BIND_BY_COLUMN (default)
    And SQL_ATTR_ROW_ARRAY_SIZE is set to fetch 3 rows
    And a query returning 3 rows is executed
    And SQLBindCol is called with arrays (column-wise binding)
    Then SQLFetch should populate the arrays

  @odbc_e2e
  Scenario: SQLBindCol supports row-wise binding.
    Given Snowflake client is logged in
    When a row structure is defined for row-wise binding
    And SQL_ATTR_ROW_BIND_TYPE is set to the row structure size
    And SQL_ATTR_ROW_ARRAY_SIZE is set
    And a query with two columns is executed
    And SQLBindCol binds columns using the first row as the base address
    Then SQLFetch should populate rows using the row-wise layout

  @odbc_e2e
  Scenario: SQLBindCol supports binding offsets via SQL_ATTR_ROW_BIND_OFFSET_PTR.
    Given Snowflake client is logged in
    When a binding offset pointer is configured
    And a query returning 2 rows is executed
    And SQLBindCol binds to the first row's buffer
    Then first fetch with offset=0 should populate rows[0]
    And second fetch with offset pointing to rows[1] should populate rows[1]

  @odbc_e2e
  Scenario: SQLBindCol binding offset of 0 uses originally bound addresses.
    Given Snowflake client is logged in
    When a binding offset pointer is set with value 0
    And a query is executed
    And SQLBindCol binds to a buffer
    Then SQLFetch with offset 0 should use the originally bound address

  @odbc_e2e
  Scenario: SQLBindCol binds arrays when SQL_ATTR_ROW_ARRAY_SIZE > 1.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_ARRAY_SIZE is set to 5
    And a query returning 5 rows is executed
    And SQLBindCol is called with an array of buffers
    Then a single SQLFetch should populate all array elements
    And subsequent fetch should return SQL_NO_DATA

  @odbc_e2e
  Scenario: SQLBindCol returns 07009 when ColumnNumber exceeds max columns.
    Given Snowflake client is logged in
    When a query with 1 column is executed
    And SQLBindCol is called with a column number that exceeds the result set
    Then SQLBindCol should return SQL_ERROR with SQLSTATE 07009 Note: Some driver managers may defer this check, so we check both bind and fetch

  @odbc_e2e
  Scenario: SQLBindCol returns SQL_INVALID_HANDLE for invalid statement handle.
    Given an invalid statement handle
    When SQLBindCol is called with the invalid handle
    Then SQLBindCol should return SQL_INVALID_HANDLE

  @odbc_e2e
  Scenario: SQLBindCol is not required - columns can be retrieved with SQLGetData.
    Given Snowflake client is logged in
    When a query is executed without binding any columns
    And SQLFetch is called
    Then SQLGetData can retrieve data without prior binding

  @odbc_e2e
  Scenario: SQLBindCol can bind some columns while SQLGetData retrieves others.
    Given Snowflake client is logged in
    When a query with two columns is executed
    And only column 1 is bound
    And SQLFetch is called
    Then the bound column should have data
    And SQLGetData can retrieve the unbound column

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_CHAR binding.
    Given Snowflake client is logged in
    When a query returning a string is executed
    And SQLBindCol binds with SQL_C_CHAR
    Then SQLFetch should return the string data

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_SBIGINT binding.
    Given Snowflake client is logged in
    When a query returning a large integer is executed
    And SQLBindCol binds with SQL_C_SBIGINT
    Then SQLFetch should return the large integer

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_DOUBLE binding.
    Given Snowflake client is logged in
    When a query returning a double is executed
    And SQLBindCol binds with SQL_C_DOUBLE
    Then SQLFetch should return the double value

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_TYPE_DATE binding.
    Given Snowflake client is logged in
    When a query returning a date is executed
    And SQLBindCol binds with SQL_C_TYPE_DATE
    Then SQLFetch should return the date components

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_TYPE_TIMESTAMP binding.
    Given Snowflake client is logged in
    When a query returning a timestamp is executed
    And SQLBindCol binds with SQL_C_TYPE_TIMESTAMP
    Then SQLFetch should return the timestamp components

  @odbc_e2e
  Scenario: SQLBindCol indicator returns full data length when buffer causes truncation.
    Given Snowflake client is logged in
    When a query returning a known-length string is executed
    And SQLBindCol uses a buffer that causes truncation
    Then SQLFetch should return SQL_SUCCESS_WITH_INFO
    And indicator should contain the full data length or SQL_NO_TOTAL

  @odbc_e2e
  Scenario: SQLBindCol binding persists across SQLFreeStmt SQL_CLOSE and re-execute.
    Given Snowflake client is logged in
    When SQLBindCol is called to bind a column
    And a query is executed and fetched
    And the cursor is closed with SQL_CLOSE (not freeing the statement)
    And a new query is executed
    Then the binding should still be in effect

  @odbc_e2e
  Scenario: SQLBindCol binding is removed by SQLFreeStmt SQL_UNBIND.
    Given Snowflake client is logged in
    When SQLBindCol is called to bind a column
    And SQL_UNBIND is used to remove all bindings
    And a query is executed and fetched
    Then the buffer should not be modified (binding was removed)

  @odbc_e2e
  Scenario: Setting SQL_DESC_COUNT to 0 on the ARD unbinds all columns.
    Given Snowflake client is logged in
    When SQLBindCol is called to bind columns
    And SQL_DESC_COUNT is set to 0 on the ARD
    And a query is executed and fetched
    Then the buffers should not be modified (all columns were unbound)

  @odbc_e2e
  Scenario: SQLBindCol converts string to integer type.
    Given Snowflake client is logged in
    When a query returning a numeric string is executed
    And SQLBindCol specifies SQL_C_LONG to convert from string to integer
    Then SQLFetch should convert the string to an integer

  @odbc_e2e
  Scenario: SQLBindCol allows binding non-consecutive columns.
    Given Snowflake client is logged in
    When a query with 3 columns is executed
    And only columns 1 and 3 are bound (skipping column 2)
    Then SQLFetch should populate only the bound columns

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_NUMERIC binding.
    Given Snowflake client is logged in
    When a query returning a numeric value is executed
    And SQLBindCol binds with SQL_C_NUMERIC
    And precision/scale are set on the ARD for proper conversion
    Then SQLFetch should succeed with the SQL_C_NUMERIC binding
    And the indicator should show the data was received (not NULL)

  @odbc_e2e
  Scenario: SQLBindCol works with SQLFetchScroll.
    Given Snowflake client is logged in
    When a query returning multiple rows is executed
    And SQLBindCol is called to bind the column
    Then SQLFetchScroll with SQL_FETCH_NEXT should populate the bound buffer
    And subsequent calls should advance the cursor
    And fetch after all rows returns SQL_NO_DATA

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_BINARY binding.
    Given Snowflake client is logged in
    When a query returning binary data (hex-encoded) is executed
    And SQLBindCol binds with SQL_C_BINARY
    Then SQLFetch should return the binary data

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_SHORT binding.
    Given Snowflake client is logged in
    When a query returning a small integer is executed
    And SQLBindCol binds with SQL_C_SHORT
    Then SQLFetch should return the value

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_FLOAT binding.
    Given Snowflake client is logged in
    When a query returning a float is executed
    And SQLBindCol binds with SQL_C_FLOAT
    Then SQLFetch should return the float value

  @odbc_e2e
  Scenario: SQLBindCol supports SQL_C_BIT binding.
    Given Snowflake client is logged in
    When a query returning a boolean is executed
    And SQLBindCol binds with SQL_C_BIT
    Then SQLFetch should return the boolean value as 1
