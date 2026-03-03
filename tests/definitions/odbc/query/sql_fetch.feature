@odbc
Feature: ODBC SQLFetch function behavior
  # Tests for SQLFetch based on ODBC specification

  @odbc_e2e
  Scenario: SQLFetch fetches a row from SELECT query
    Given Snowflake client is logged in
    When Query "SELECT 42 AS value" is executed
    Then SQLFetch should return SQL_SUCCESS and retrieve the value
    And subsequent fetch should return SQL_NO_DATA

  @odbc_e2e
  Scenario: SQLFetch returns data about number of rows affected.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute the query that returns 1 row
    And SQLSetStmtAttr is called to set the rows fetched pointer
    And SQLFetch is called to fetch the row
    Then SQLFetch should return SQL_SUCCESS and retrieve the value
    And the number of rows affected should be 1

  @odbc_e2e
  Scenario: SQLSetStmtAttr sets supported cursor types.
    Given Snowflake client is logged in
    When SQLSetStmtAttr is called with SQL_ATTR_CURSOR_TYPE to set the cursor type
    And SQLGetStmtAttr is called to get the current cursor type
    Then default cursor type is SQL_CURSOR_FORWARD_ONLY
    And SQL_CURSOR_STATIC is not supported
    And SQL_CURSOR_KEYSET_DRIVEN is not supported
    And SQL_CURSOR_DYNAMIC is not supported
    And SQL_CURSOR_FORWARD_ONLY is supported

  @odbc_e2e
  Scenario: SQLFetch can be mixed with SQLFetchScroll.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute the query that returns 10 rows
    Then calls to SQLFetch and SQLFetchScroll can be mixed
    And SQLGetData returns correct values for the current row
    And SQLFetch returns SQL_NO_DATA when there are no more rows

  @odbc_e2e
  Scenario: SQLFetch returns multiple rows when SQL_ATTR_ROW_ARRAY_SIZE is set.
    Given Snowflake client is logged in
    When SQLSetStmtAttr is called to set the row array size
    And SQL_ATTR_ROW_ARRAY_SIZE is set to the correct value
    And SQLExecDirect is called to execute the query that returns 15 rows
    And SQLBindCol is called to bind the column to the value
    Then SQLFetch should return SQL_SUCCESS and retrieve the first 10 rows
    And SQLFetch should return SQL_SUCCESS and retrieve the next 5 rows
    And SQLFetch should return SQL_NO_DATA when there are no more rows

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_STATUS_PTR returns SQL_ROW_SUCCESS for successfully fetched rows.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_STATUS_PTR is set to point to a row status array
    And SQL_ATTR_ROW_ARRAY_SIZE is set
    And SQLExecDirect is called to execute the query that returns 5 rows
    And SQLBindCol is called to bind the column to the value
    And SQLFetch is called to fetch the rows
    Then row status array should be updated with SQL_ROW_SUCCESS for all fetched rows
    And subsequent fetch should return SQL_NO_DATA

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_STATUS_PTR returns SQL_ROW_SUCCESS_WITH_INFO when data is truncated.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_STATUS_PTR is set to point to a row status array
    And SQL_ATTR_ROW_ARRAY_SIZE is set
    And SQLExecDirect is called to execute the query that returns a long string
    And SQLBindCol is called with a small buffer that will cause truncation
    And SQLFetch is called to fetch the row
    Then SQLFetch should return SQL_SUCCESS_WITH_INFO
    And row status should be SQL_ROW_SUCCESS_WITH_INFO
    And SQLSTATE should indicate string data truncation

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_STATUS_PTR returns SQL_ROW_ERROR when conversion error occurs.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_STATUS_PTR is set to point to a row status array
    And SQL_ATTR_ROW_ARRAY_SIZE is set
    And SQLExecDirect is called to execute the query that returns a non-numeric string
    And SQLBindCol is called to bind to an integer type (will cause conversion error)
    And SQLFetch is called to fetch the row
    Then SQLFetch should return SQL_ERROR or SQL_SUCCESS_WITH_INFO depending on error handling
    And row status should be SQL_ROW_ERROR

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_STATUS_PTR returns SQL_ROW_NOROW when rowset overlaps end of result set.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_STATUS_PTR is set to point to a row status array
    And SQL_ATTR_ROW_ARRAY_SIZE is set to 10
    And SQLExecDirect is called to execute the query that returns only 3 rows
    And SQLBindCol is called to bind the column
    And SQLFetch is called to fetch the rows
    Then first 3 rows should have SQL_ROW_SUCCESS
    And remaining 7 rows should have SQL_ROW_NOROW
    And subsequent fetch should return SQL_NO_DATA

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_ARRAY_SIZE set on ARD.
    Given Snowflake client is logged in
    When SQL_DESC_ARRAY_SIZE is set on ARD to fetch multiple rows
    And SQLExecDirect is called to execute a query that returns 5 rows
    And SQLBindCol is called to bind the column
    Then SQLFetch should return SQL_SUCCESS and retrieve all 5 rows
    And subsequent fetch should return SQL_NO_DATA

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_ARRAY_STATUS_PTR set on IRD.
    Given Snowflake client is logged in
    When SQL_DESC_ARRAY_STATUS_PTR is set on IRD
    And SQL_ATTR_ROW_ARRAY_SIZE is set
    And SQLExecDirect is called to execute a query that returns 3 rows
    And SQLBindCol is called to bind the column
    Then SQLFetch should populate the row status array via IRD descriptor

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_BIND_OFFSET_PTR set on ARD.
    Given Snowflake client is logged in
    When SQL_DESC_BIND_OFFSET_PTR is set on ARD
    And SQLExecDirect is called to execute a query that returns 2 rows
    And SQLBindCol binds to the first row's buffer
    Then first fetch with offset=0 should populate rows[0]
    And second fetch with offset pointing to rows[1] should populate rows[1]

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_BIND_TYPE set on ARD for row-wise binding.
    Given Snowflake client is logged in
    When SQL_DESC_BIND_TYPE is set on ARD for row-wise binding
    And SQL_DESC_ARRAY_SIZE is set
    And SQLExecDirect is called to execute a query with two columns
    And SQLBindCol binds columns using the row structure
    Then SQLFetch should populate rows using row-wise binding

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_COUNT set on ARD.
    Given Snowflake client is logged in
    When columns are bound
    And SQLExecDirect is called to execute a query with two columns
    And only the first column is bound
    Then SQL_DESC_COUNT should reflect one bound column
    And SQLFetch should successfully fetch only the bound column
    And when SQL_DESC_COUNT is set to 0, no columns are bound
    And verify the count is 0

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_DATA_PTR set on ARD.
    Given Snowflake client is logged in
    When SQL_DESC_DATA_PTR is set directly on ARD
    And SQLExecDirect is called
    And descriptor fields are set directly instead of using SQLBindCol
    Then SQLFetch should use the descriptor-specified data buffer

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_INDICATOR_PTR set on ARD.
    Given Snowflake client is logged in
    When SQL_DESC_INDICATOR_PTR is set directly on ARD
    And SQLExecDirect is called with a query returning NULL
    And descriptor fields are set directly
    Then SQLFetch should set indicator to SQL_NULL_DATA for NULL value

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_OCTET_LENGTH set on ARD.
    Given Snowflake client is logged in
    When SQL_DESC_OCTET_LENGTH is set on ARD
    And SQLExecDirect is called with a string query
    And descriptor fields are set with a small octet length to cause truncation
    Then SQLFetch should respect the octet length and truncate

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_OCTET_LENGTH_PTR set on ARD.
    Given Snowflake client is logged in
    When SQL_DESC_OCTET_LENGTH_PTR is set on ARD
    And SQLExecDirect is called
    And descriptor fields are set with separate octet length pointer
    Then SQLFetch should set the octet length pointer to the actual data length

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_ROWS_PROCESSED_PTR set on IRD.
    Given Snowflake client is logged in
    When SQL_DESC_ROWS_PROCESSED_PTR is set on IRD
    And SQL_ATTR_ROW_ARRAY_SIZE is set
    And SQLExecDirect is called to execute a query that returns 3 rows
    And SQLBindCol is called to bind the column
    Then SQLFetch should set rows_processed via the IRD descriptor

  @odbc_e2e
  Scenario: SQLFetch respects SQL_DESC_TYPE set on ARD.
    Given Snowflake client is logged in
    When SQL_DESC_TYPE is set on ARD for type conversion
    And SQLExecDirect is called with a numeric query
    And descriptor fields are set to convert to string
    Then SQLFetch should convert the numeric value to string

  @odbc_e2e
  Scenario: SQLFetch respects multiple ARD descriptor fields set together.
    Given Snowflake client is logged in
    When multiple ARD descriptor fields are set together
    And SQLExecDirect is called
    And descriptor fields for column 1 (id)
    And descriptor fields for column 2 (name)
    Then SQLFetch should use all descriptor settings together

  @odbc_e2e
  Scenario: SQLFetch respects both ARD and IRD descriptor fields.
    Given Snowflake client is logged in
    When both ARD and IRD descriptor fields are set
    And SQLExecDirect is called to return 4 rows
    And SQLBindCol is called
    Then SQLFetch should respect both ARD and IRD settings

  @odbc_e2e
  Scenario: SQLFetch ignores SQL_ATTR_MAX_LENGTH on statement handle.
    Given Snowflake client is logged in
    When SQL_ATTR_MAX_LENGTH is set on the statement handle
    And SQLGetStmtAttr is called to verify the attribute was set
    And SQLExecDirect is called to execute a query returning a string longer than max_length
    And SQLBindCol is called with a buffer large enough to hold the full string
    And SQLFetch is called to fetch the row
    Then the data should be truncated to SQL_ATTR_MAX_LENGTH characters

  @odbc_e2e
  Scenario: SQLFetch returns 22002 when NULL data fetched without indicator pointer.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a query returning NULL
    And SQLBindCol is called without an indicator pointer (NULL for StrLen_or_IndPtr)
    Then SQLFetch should return SQL_ERROR with SQLSTATE 22002

  @odbc_e2e
  Scenario: SQLFetch returns 22018 when invalid date string is bound to SQL_C_TYPE_DATE.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a query returning an invalid date string
    And SQLBindCol is called to bind to a DATE structure
    Then SQLFetch should return SQL_ERROR with SQLSTATE 22007 (Invalid datetime format)

  @odbc_e2e
  Scenario: SQLFetch returns 24000 when no result set exists.
    Given Snowflake client is logged in
    When a non-SELECT statement is executed (no result set)
    Then SQLFetch should return SQL_ERROR with SQLSTATE 24000 (Invalid cursor state)

  @odbc_e2e
  Scenario: SQLFetch returns SQL_NO_DATA when result set is empty.
    Given Snowflake client is logged in
    When a SELECT statement is executed that returns no rows
    Then SQLFetch should return SQL_NO_DATA (no rows to fetch)

  @odbc_e2e
  Scenario: SQLFetch returns HY010 when called without executing statement.
    Given Snowflake client is logged in
    When SQLFetch is called without executing any statement first
    Then SQLFetch should return SQL_ERROR with SQLSTATE HY010 (Function sequence error)

  @odbc_e2e
  Scenario: SQLFetch moves cursor forward when no columns are bound.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a query that returns 3 rows
    And no columns are bound
    Then SQLFetch should return SQL_SUCCESS for each row
    And SQLFetch should return SQL_NO_DATA after all rows are consumed
    And SQLGetData can still retrieve data after moving cursor without bound columns Reset and test again

  @odbc_e2e
  Scenario: SQLFetch supports separate length and indicator buffers via descriptor.
    Given Snowflake client is logged in
    When SQL_DESC_INDICATOR_PTR and SQL_DESC_OCTET_LENGTH_PTR are set to different buffers
    And SQLExecDirect is called with a query returning a string
    And descriptor fields are set with separate indicator and length pointers
    Then SQLFetch should set both indicator and length appropriately

  @odbc_e2e
  Scenario: SQLFetch cannot be called after SQLExtendedFetch without SQLFreeStmt.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a query
    And SQLExtendedFetch is called first
    Then SQLFetch should return SQL_ERROR with SQLSTATE HY010 (Function sequence error) because SQLFetch cannot be mixed with SQLExtendedFetch without closing cursor
    But after SQLFreeStmt with SQL_CLOSE and re-executing, SQLFetch should work

  @odbc_e2e
  Scenario: SQLGetDiagField returns correct row and column number on fetch error.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_ARRAY_SIZE is set for block cursor
    And SQLExecDirect is called to execute a query with data that will cause conversion error Using UNION to create specific rows with a bad value in the middle
    And SQLBindCol is called to bind to an integer type (will cause conversion error on row 2)
    And SQL_ATTR_ROW_STATUS_PTR is set
    Then SQLFetch should return SQL_SUCCESS_WITH_INFO or SQL_ERROR
    And SQLGetDiagField should return the row number where error occurred
    And SQLGetDiagField should return the column number

  @odbc_e2e
  Scenario: SQLFetch returns SQL_SUCCESS_WITH_INFO when error occurs on subset of rows in block cursor.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_ARRAY_SIZE is set for block cursor
    And SQL_ATTR_ROW_STATUS_PTR is set
    And SQL_ATTR_ROWS_FETCHED_PTR is set
    And SQLExecDirect is called with a mix of valid and invalid conversion data Row 1: valid, Row 2: invalid, Row 3: valid, Row 4: invalid, Row 5: valid
    And SQLBindCol is called to bind to an integer type
    Then SQLFetch should return SQL_SUCCESS_WITH_INFO (not SQL_ERROR) because errors occurred on some but not all rows
    And rows_fetched should indicate how many rows were attempted
    And row_status array should show mixed results Valid rows should have SQL_ROW_SUCCESS Invalid rows should have SQL_ROW_ERROR or SQL_ROW_SUCCESS_WITH_INFO
