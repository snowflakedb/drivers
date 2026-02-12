@core @odbc @python
Feature: Private Key Authentication

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should authenticate using private file with password
    Given Authentication is set to JWT and private file with password is provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should authenticate using unencrypted private key file
    Given Authentication is set to JWT and an unencrypted private key file is provided (no password)
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should fail JWT authentication when invalid private key provided
    Given Authentication is set to JWT and invalid private key file is provided
    When Trying to Connect
    Then There is error returned

  @core_int @odbc_int @python_int
  Scenario: should fail JWT authentication when no private file provided
    Given Authentication is set to JWT
    When Trying to Connect with no private file provided
    Then There is error returned

  @core_e2e @python_e2e
  Scenario: should authenticate using private_key as bytes
    Given Authentication is set to JWT and private key is provided as bytes
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should authenticate using private_key as base64 string
    Given Authentication is set to JWT and private key is provided as base64-encoded string
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @odbc_e2e
  Scenario: should authenticate using PRIV_KEY_PWD as alias for private key password
    Given Authentication is set to JWT with encrypted key file and PRIV_KEY_PWD parameter
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @python_e2e
  Scenario: should authenticate using private_key as RSAPrivateKey object
    Given Authentication is set to JWT and private key is provided as RSAPrivateKey object
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e
  Scenario: should automatically update authenticator to JWT if key pair params present
    Given private key or private key file is provided and authenticator is not explicitly set
    When Trying to Connect
    Then Connector changes authenticator to JWT and login is successful and simple query can be executed

  @core_unit
  Scenario: should fail when both private_key and private_key_file are provided
    Given Both private_key and private_key_file parameters are set
    When Trying to Connect
    Then There is error returned indicating conflicting parameters

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
