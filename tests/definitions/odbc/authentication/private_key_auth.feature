@odbc
Feature: Private Key Authentication (ODBC-specific)

  @odbc_e2e
  Scenario: should authenticate using PRIV_KEY_PWD as alias for private key password
    Given Authentication is set to JWT with encrypted key file and PRIV_KEY_PWD parameter
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @odbc_int
  Scenario: should forward private key content set via SQLSetConnectAttr to core
    Given A connection handle is allocated and PRIV_KEY_CONTENT is set via SQLSetConnectAttr
    When Trying to Connect
    Then The private key is forwarded to core and used for JWT authentication

  @odbc_int
  Scenario: should forward base64 private key set via SQLSetConnectAttr to core
    Given A connection handle is allocated and PRIV_KEY_BASE64 is set via SQLSetConnectAttr
    When Trying to Connect
    Then The private key is forwarded to core and used for JWT authentication

  @odbc_int
  Scenario: should forward private key password set via SQLSetConnectAttr to core
    Given A connection handle is allocated and PRIV_KEY_PASSWORD is set via SQLSetConnectAttr
    When Trying to Connect
    Then The private key password is forwarded to core and used for JWT authentication
