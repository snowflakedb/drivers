# ============================================================================
# SnowflakeConnectionPoolDataSource configuration & DataSource API surface
#
# BDD feature for the offline configuration / API surface of the pooled data
# source (SnowflakeConnectionPoolDataSource, implemented by
# SnowflakePooledConnectionDataSource, which extends SnowflakeBasicDataSource).
# Every scenario is @jdbc_int and is mirrored by a test method in
# jdbc/src/test/java/net/snowflake/jdbc/integration/pooling/PoolDataSourceConfigurationTests.java
# These tests never open a Snowflake session; they assert what each setter
# stores/exposes on the pooled data source.
#
# Live pooling behavior (getPooledConnection / getConnection round-trips,
# borrowing logical connections, authentication) is covered separately by the
# e2e feature tests/definitions/shared/pooling/connection_pool.feature.
#
# In the storage Scenario Outlines the "property" column is the EXACT key the
# setter writes into getProperties() (i.e. SessionProperty.getKey()), so the
# mirrored @jdbc_int test asserts getProperties().getProperty(<property>).
#
# Related behavior differences: BD#5 (MFA credential-cache consolidation),
# BD#31 (DataSource setters wired to sf_core only).
# Authenticator auto-promotion when setBrowserResponseTimeout is set is
# deferred to SNOW-3595091 and is not documented as a behavior difference yet.
# ============================================================================

@jdbc
Feature: Connection pool data source configuration

  # ==========================================================================
  # ENDPOINT / URL CONFIGURATION
  # Methods: setUrl, getUrl, setServerName, setPortNumber, setAccount,
  #          setDatabaseName, setSchema, setWarehouse, setRole
  # ==========================================================================

  @jdbc_int
  Scenario: should return the URL that was explicitly set
    Given a new Snowflake connection pool data source
    When the URL is configured with setUrl
    Then getUrl returns the same URL

  @jdbc_int
  Scenario: should build the JDBC URL from the server name and port number
    Given a new Snowflake connection pool data source with no explicit URL
    When the server name is configured with setServerName and the port with setPortNumber
    Then getUrl returns a jdbc:snowflake URL that contains the server name and port

  @jdbc_int
  Scenario Outline: should store the <property> endpoint property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter          | property  |
      | setAccount      | account   |
      | setDatabaseName | database  |
      | setSchema       | schema    |
      | setWarehouse    | warehouse |
      | setRole         | role      |

  # ==========================================================================
  # AUTHENTICATION CONFIGURATION (storage only)
  # Methods: setAuthenticator, setToken, setPasscode, setPasscodeInPassword,
  #          setDisableSamlURLCheck, setSsl, setPrivateKey, setPrivateKeyFile,
  #          setPrivateKeyBase64, setClientStoreTemporaryCredential
  # ==========================================================================

  @jdbc_int
  Scenario Outline: should store the <property> authentication property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter                | property           |
      | setAuthenticator      | authenticator      |
      | setToken              | token              |
      | setPasscode           | passcode           |
      | setPasscodeInPassword | passcodeInPassword |
      | setDisableSamlURLCheck| disable_saml_url_check|
      | setSsl                | ssl                |

  @jdbc_int
  Scenario Outline: should store private key material configured via <setter>
    Given a new Snowflake connection pool data source
    When <setter> is called with the key material
    Then the corresponding private key configuration is stored in the data source

    Examples:
      | setter              |
      | setPrivateKey       |
      | setPrivateKeyFile   |
      | setPrivateKeyBase64 |

  @jdbc_int
  Scenario: should store the clientStoreTemporaryCredential property
    Given a new Snowflake connection pool data source
    When setClientStoreTemporaryCredential is called with true
    Then the clientStoreTemporaryCredential property is stored in the data source configuration

  # ==========================================================================
  # PROXY CONFIGURATION
  # Methods: setProxyHost, setProxyPort, setProxyUser, setProxyPassword,
  #          setNonProxyHosts
  # ==========================================================================

  @jdbc_int
  Scenario Outline: should store the <property> proxy property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter           | property       |
      | setProxyHost     | proxyHost      |
      | setProxyPort     | proxyPort      |
      | setProxyUser     | proxyUser      |
      | setProxyPassword | proxyPassword  |
      | setNonProxyHosts | nonProxyHosts  |

  # ==========================================================================
  # CLIENT BEHAVIOR
  # Methods: setApplication, setAllowUnderscoresInHost
  # ==========================================================================

  @jdbc_int
  Scenario Outline: should store the <property> client behavior property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter                    | property               |
      | setApplication            | application            |
      | setAllowUnderscoresInHost | allowUnderscoresInHost |

  # ==========================================================================
  # TIMEOUTS & RETRIES
  # Methods: setQueryTimeout, setMaxHttpRetries, setPutGetMaxRetries,
  #          setBrowserResponseTimeout, setLoginTimeout, getLoginTimeout
  # Note: authenticator auto-promotion when setBrowserResponseTimeout is set is
  # deferred to SNOW-3595091 (not documented as a BD yet).
  # ==========================================================================

  @jdbc_int
  Scenario Outline: should store the <property> timeout or retry property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter                    | property                 |
      | setQueryTimeout           | queryTimeoutSeconds      |
      | setMaxHttpRetries         | maxHttpRetries           |
      | setPutGetMaxRetries       | putGetMaxRetries         |
      | setBrowserResponseTimeout | browser_response_timeout |

  @jdbc_int
  Scenario: should round-trip the login timeout
    Given a new Snowflake connection pool data source
    When the login timeout is configured with setLoginTimeout
    Then getLoginTimeout returns the configured value

  @jdbc_int
  Scenario: should not auto-promote the authenticator when the browser response timeout is set
    Given a new Snowflake connection pool data source
    When setBrowserResponseTimeout is called
    Then the browser response timeout is stored and the authenticator is left unset

  # ==========================================================================
  # DIAGNOSTICS
  # Methods: setEnableDiagnostics, setDiagnosticsAllowlistFile
  # ==========================================================================

  @jdbc_int
  Scenario Outline: should store the <property> diagnostics property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter                    | property                 |
      | setEnableDiagnostics      | enableDiagnostics        |
      | setDiagnosticsAllowlistFile | diagnosticsAllowlistFile |

  # ==========================================================================
  # UNSUPPORTED CommonDataSource OPERATIONS
  # Methods: getLogWriter, setLogWriter, getParentLogger
  # ==========================================================================

  @jdbc_int
  Scenario Outline: should reject the unsupported operation <operation>
    Given a new Snowflake connection pool data source
    When <operation> is invoked on the data source
    Then a SQLFeatureNotSupportedException is thrown

    Examples:
      | operation       |
      | getLogWriter    |
      | setLogWriter    |
      | getParentLogger |

  # ==========================================================================
  # WRAPPER & PROPERTIES ACCESS
  # Methods: unwrap(Class), isWrapperFor(Class), getProperties
  # ==========================================================================

  @jdbc_int
  Scenario: should unwrap the data source to a supported interface
    Given a new Snowflake connection pool data source
    When isWrapperFor and unwrap are called with a supported interface
    Then isWrapperFor returns true and unwrap returns the data source instance

  @jdbc_int
  Scenario: should reject unwrapping to an unsupported interface
    Given a new Snowflake connection pool data source
    When isWrapperFor and unwrap are called with an unsupported interface
    Then isWrapperFor returns false and unwrap throws a SQLException

  @jdbc_int
  Scenario: should expose the configuration as a defensive copy of properties
    Given a Snowflake connection pool data source with configuration applied
    When getProperties is called and the returned map is mutated
    Then the data source's own configuration remains unchanged
