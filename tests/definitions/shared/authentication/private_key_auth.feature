@core @odbc @python @jdbc
Feature: Private Key Authentication

  @core_e2e @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should authenticate using private file with password
    Given Authentication is set to JWT and private file with password is provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should authenticate using unencrypted private key file
    Given Authentication is set to JWT and an unencrypted private key file is provided (no password)
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should fail JWT authentication when invalid private key provided
    Given Authentication is set to JWT and invalid private key file is provided
    When Trying to Connect
    Then There is error returned

  @core_int @odbc_int @python_int @jdbc_e2e
  Scenario: should fail JWT authentication when no private file provided
    Given Authentication is set to JWT
    When Trying to Connect with no private file provided
    Then There is error returned

  @core_e2e @python_e2e
  Scenario: should authenticate using private_key as bytes
    Given Authentication is set to JWT and private key is provided as bytes
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should authenticate using private_key as base64 string
    Given Authentication is set to JWT and private key is provided as base64-encoded string
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @python_e2e @jdbc_e2e
  Scenario: should authenticate using private_key as pem string
    Given Authentication is set to JWT and private key is provided as plaintext PEM
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @python_e2e
  Scenario: should authenticate using private_key as RSAPrivateKey object
    Given Authentication is set to JWT and private key is provided as RSAPrivateKey object
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @jdbc_e2e
  Scenario: should authenticate using private key object
    Given a PrivateKey object is provided directly
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @jdbc_e2e
  Scenario: should automatically update authenticator to JWT if key pair params present
    Given private key or private key file is provided and authenticator is not explicitly set
    When Trying to Connect
    Then Connector changes authenticator to JWT and login is successful and simple query can be executed

  @odbc_e2e
  Scenario: should authenticate using PRIV_KEY_PWD as alias for private key password
    Given Authentication is set to JWT with encrypted key file and PRIV_KEY_PWD parameter
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_int
  Scenario: should surface JWT credential rejection code
    Given Authentication is set to JWT and the backend is configured to reject the JWT as invalid
    When Trying to Connect
    Then the raw GS code surfaces in the error

