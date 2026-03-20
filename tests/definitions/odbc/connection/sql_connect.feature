@odbc
Feature: SQLConnect and SQLBrowseConnect

  @odbc_e2e
  Scenario: SQLConnect connects via DSN with all credentials in DSN
    Given A DSN is installed with all connection parameters
    When SQLConnect is called with the DSN name and no explicit credentials
    Then The connection succeeds and a simple query can be executed

  @odbc_e2e
  Scenario: SQLConnect returns IM002 for an unknown DSN
    Given No DSN named NonExistentDSN exists
    When SQLConnect is called with DSN NonExistentDSN
    Then SQL_ERROR is returned with SQLSTATE IM002

  @odbc_e2e
  Scenario: SQLBrowseConnect returns SQL_NEED_DATA when server info is missing
    Given A connection handle is allocated
    When SQLBrowseConnect is called with an empty connection string
    Then SQL_NEED_DATA is returned and the output contains a connection template
