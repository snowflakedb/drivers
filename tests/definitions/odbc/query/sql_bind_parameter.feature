@odbc
Feature: ODBC SQLBindParameter spec compliance
  # Tests based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindparameter-function
  # Covers error codes, API behavior, and descriptor integration.

  # ============================================================================
  # Error Codes
  # ============================================================================

  @odbc_e2e
  Scenario: should return SQL_INVALID_HANDLE for null statement handle.
    Given No statement handle exists
    When SQLBindParameter is called with SQL_NULL_HSTMT
    Then SQL_INVALID_HANDLE should be returned

  @odbc_e2e
  Scenario: should return 07009 when ParameterNumber is zero.
    Given Snowflake client is logged in
    When SQLBindParameter is called with ParameterNumber 0
    Then SQL_ERROR with SQLSTATE 07009 should be returned

  @odbc_e2e
  Scenario: should return HY003 for invalid C data type.
    Given Snowflake client is logged in
    When SQLBindParameter is called with invalid ValueType 9999
    Then SQL_ERROR with SQLSTATE HY003 should be returned

  @odbc_e2e
  Scenario: should return HY004 for invalid SQL data type.
    Given Snowflake client is logged in
    When SQLBindParameter is called with invalid ParameterType 8888
    Then SQL_ERROR with SQLSTATE HY004 should be returned

  @odbc_e2e
  Scenario: should return HY105 for invalid InputOutputType.
    Given Snowflake client is logged in
    When SQLBindParameter is called with invalid InputOutputType 999
    Then SQL_ERROR with SQLSTATE HY105 should be returned

  @odbc_e2e
  Scenario: should return HY009 when both value and indicator pointers are null.
    Given Snowflake client is logged in
    When SQLBindParameter is called with null ParameterValuePtr and null StrLen_or_IndPtr
    Then SQL_ERROR with SQLSTATE HY009 should be returned

  @odbc_e2e
  Scenario: should return HY090 for negative BufferLength.
    Given Snowflake client is logged in
    When SQLBindParameter is called with BufferLength -1
    Then SQL_ERROR with SQLSTATE HY090 should be returned

  @odbc_e2e
  Scenario: should return HY104 for invalid precision or scale.
    Given Snowflake client is logged in
    When SQLBindParameter is called with negative DecimalDigits
    Then the new driver rejects with HY104, the old driver accepts it

  # TODO: Uncomment when descriptor consistency checks are implemented
  # Scenario: should return HY021 for inconsistent descriptor information.

  # TODO: Uncomment when unsupported type conversion detection is implemented
  # Scenario: should return HYC00 for unsupported type conversion.

  # ============================================================================
  # API Behavior
  # ============================================================================

  @odbc_e2e
  Scenario: should execute via SQLExecDirect with pre-bound parameter.
    Given Snowflake client is logged in
    When a parameter is bound before calling SQLExecDirect
    And SQLExecDirect is called with a parameterized query
    Then the bound parameter value should be returned

  @odbc_e2e
  Scenario: should replace binding when same ParameterNumber is rebound.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And parameter 1 is bound to value 111
    And parameter 1 is rebound to value 222
    Then executing should return the latest bound value

  @odbc_e2e
  Scenario: should fail with 07002 after SQL_RESET_PARAMS clears bindings.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared and executed successfully
    And all parameter bindings are reset
    Then re-executing should fail with SQLSTATE 07002

  @odbc_e2e
  Scenario: should fail with 07002 when parameter bindings have a gap.
    Given Snowflake client is logged in
    When a query with 3 parameter markers is prepared
    And only parameters 1 and 3 are bound (gap at parameter 2)
    Then executing should fail with SQLSTATE 07002

  @odbc_e2e
  Scenario: should reflect changed bound variable on re-execution.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared and bound to a variable
    And first execution returns 10
    And the bound variable is changed to 20 without rebinding
    Then re-executing should return the updated value

  @odbc_e2e
  Scenario: should bind multiple parameters to a single statement.
    Given Snowflake client is logged in
    When a SELECT with two parameter markers is prepared
    And an integer and a string parameter are bound
    Then executing and fetching should return both values

  @odbc_e2e
  Scenario: should rebind parameter to different type without SQL_RESET_PARAMS.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an integer parameter is bound and executed
    And the same parameter is rebound as a string without calling SQL_RESET_PARAMS
    Then re-executing should return the new string value

  @odbc_e2e
  Scenario: should bind NULL via SQL_NULL_DATA indicator.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared with a NULL-indicating parameter
    Then executing and fetching should return NULL

  @odbc_e2e
  Scenario: should alternate NULL and non-NULL across sequential executions.
    Given Snowflake client is logged in
    When a parameterized INSERT is prepared with a bound integer
    And rows are inserted: 100, NULL, 200
    Then selecting all rows should return 100, NULL, 200

  @odbc_e2e
  Scenario: should allow rebinding after SQL_RESET_PARAMS.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared and an integer is bound
    And all parameter bindings are reset
    And a new string parameter is bound to the same parameter position
    Then re-executing should return the new string value

  # ============================================================================
  # APD/IPD Descriptor Integration
  # ============================================================================

  @odbc_e2e
  Scenario: should populate APD descriptor fields after SQLBindParameter.
    Given Snowflake client is logged in
    When a char parameter is bound with explicit buffer length and indicator
    Then the APD record should reflect all bound fields
    And the APD header should report the correct count

  @odbc_e2e
  Scenario: should populate IPD descriptor fields after SQLBindParameter.
    Given Snowflake client is logged in
    When a decimal parameter is bound with precision and scale
    Then the IPD record should reflect all bound fields
    And the IPD header should report the correct count

  @odbc_e2e
  Scenario: should report parameter count via SQLNumParams after binding.
    Given Snowflake client is logged in
    When a statement with two parameter markers is prepared and both are bound
    Then SQLNumParams should return 2

  @odbc_e2e
  Scenario: should describe bound parameter via SQLDescribeParam.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared and an integer parameter is bound
    Then SQLDescribeParam should return the SQL type information

  # ============================================================================
  # Descriptor Error Scenarios
  # ============================================================================

  @odbc_e2e
  Scenario: should return SQL_NO_DATA for unbound APD record.
    Given Snowflake client is logged in
    When no parameters are bound and APD record 1 is queried
    Then SQL_NO_DATA should be returned per ODBC spec

  @odbc_e2e
  Scenario: should return SQL_NO_DATA for unbound IPD record.
    Given Snowflake client is logged in
    When no parameters are bound and IPD record 1 is queried
    Then the record should not be found (SQL_NO_DATA per spec; old driver returns SQL_ERROR)

  @odbc_e2e
  Scenario: should return error for negative descriptor record number.
    Given Snowflake client is logged in
    When APD is queried with negative record number
    Then SQL_ERROR should be returned

  @odbc_e2e
  Scenario: should return error for unknown descriptor field identifier.
    Given Snowflake client is logged in and a parameter is bound
    When APD is queried with an unknown field identifier
    Then SQL_ERROR should be returned

  @odbc_e2e
  Scenario: should return error for header-only field on record index greater than zero.
    Given Snowflake client is logged in and a parameter is bound
    When APD is queried with SQL_DESC_ARRAY_SIZE (header field) on record 1
    Then SQL_SUCCESS — per ODBC spec, header fields ignore RecNumber

  @odbc_e2e
  Scenario: should report correct APD and IPD count for multiple parameters.
    Given Snowflake client is logged in
    When three parameters are bound
    Then APD and IPD should both report count 3

  @odbc_e2e
  Scenario: should reset APD count to zero after SQL_RESET_PARAMS.
    Given Snowflake client is logged in
    When a parameter is bound and then bindings are reset
    Then APD count should be zero

  @odbc_e2e
  Scenario: should report APD count zero when no parameters are bound.
    Given Snowflake client is logged in
    When APD header count is queried before any binding
    Then count should be 0

