# ============================================================================
# SnowflakeConnectionPoolDataSource configuration & DataSource API surface
#
# SPEC-ONLY feature file: scenarios are intentionally left UNTAGGED (TODO) so
# they carry no test-method requirement yet. The format validator reports them
# as TODO without failing CI. The scenario-level @jdbc_int tags and the mirrored
# JUnit tests (PoolDataSourceConfigurationTests) land in the implementation PR.
#
# Covers the offline configuration / API surface of the pooled data source
# (SnowflakeConnectionPoolDataSource, implemented by
# SnowflakePooledConnectionDataSource, which extends SnowflakeBasicDataSource):
# the inherited DataSource / CommonDataSource / java.sql.Wrapper methods and
# every SnowflakeDataSource configuration setter. Live pooling behavior
# (getPooledConnection / getConnection round-trips, borrowing logical
# connections, authentication) is covered separately by the e2e feature
# tests/definitions/shared/pooling/connection_pool.feature.
#
# In the storage Scenario Outlines the "property" column is the EXACT key the
# setter writes into getProperties() (i.e. SessionProperty.getKey()), so a
# future @jdbc_int test can assert getProperties().getProperty(<property>).
#
# Related behavior differences: BD#5 / BD#44 (MFA credential-cache
# consolidation), BD#42 (setters stored but not yet consumed by sf_core),
# BD#43 (HTTP header customizers not wired), BD#45 (browser response timeout
# does not auto-promote the authenticator).
# ============================================================================

Feature: Connection pool data source configuration

  # TODO: Intentional coverage gaps (error paths deferred to follow-up scenarios):
  #   - connection creation with invalid credentials (@jdbc_e2e; see connection_pool.feature)
  #   - getConnection / getPooledConnection with unset or blank URL
  #   - setUrl with a malformed URL
  #   - null-argument handling for other nullable/object setters not yet listed below

  # ==========================================================================
  # ENDPOINT / URL CONFIGURATION
  # Methods: setUrl, getUrl, setServerName, setPortNumber, setAccount,
  #          setDatabaseName, setSchema, setWarehouse, setRole
  # ==========================================================================

  Scenario: should return the URL that was explicitly set
    Given a new Snowflake connection pool data source
    When the URL is configured with setUrl
    Then getUrl returns the same URL

  Scenario: should build the JDBC URL from the server name and port number
    Given a new Snowflake connection pool data source with no explicit URL
    When the server name is configured with setServerName and the port with setPortNumber
    Then getUrl returns a jdbc:snowflake URL that contains the server name and port

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
  #          setPrivateKeyBase64, setEnableClientRequestMfaToken,
  #          setEnableClientStoreTemporaryCredential
  # ==========================================================================

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

  Scenario Outline: should store private key material configured via <setter>
    Given a new Snowflake connection pool data source
    When <setter> is called with the key material
    Then the corresponding private key configuration is stored in the data source

    Examples:
      | setter              |
      | setPrivateKey       |
      | setPrivateKeyFile   |
      | setPrivateKeyBase64 |

  Scenario Outline: should consolidate the legacy credential-cache setter <setter>
    Given a new Snowflake connection pool data source
    When <setter> is called
    Then the consolidated clientStoreTemporaryCredential property is stored
    And the legacy property name is not written to the configuration

    Examples:
      | setter                                 |
      | setEnableClientRequestMfaToken         |
      | setEnableClientStoreTemporaryCredential|

  # ==========================================================================
  # PROXY CONFIGURATION
  # Methods: setUseProxy, setProxyHost, setProxyPort, setProxyUser,
  #          setProxyPassword, setProxyProtocol, setNonProxyHosts,
  #          setDisableSocksProxy
  # ==========================================================================

  Scenario Outline: should store the <property> proxy property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter              | property          |
      | setUseProxy         | useProxy          |
      | setProxyHost        | proxyHost         |
      | setProxyPort        | proxyPort         |
      | setProxyUser        | proxyUser         |
      | setProxyPassword    | proxyPassword     |
      | setProxyProtocol    | proxyProtocol     |
      | setNonProxyHosts    | nonProxyHosts     |
      | setDisableSocksProxy| ignoreJvmSocksProxy |

  # ==========================================================================
  # CLIENT BEHAVIOR & FORMAT TOGGLES
  # (some stored-only until sf_core support lands, BD#42)
  # Methods: setTracing, setApplication, setClientConfigFile,
  #          setAllowUnderscoresInHost, setDisableGcsDefaultCredentials,
  #          setArrowTreatDecimalAsInt, setStringsQuotedForColumnDef,
  #          setEnablePutGet, setEnablePatternSearch, setOcspFailOpen,
  #          setJDBCDefaultFormatDateWithTimezone, setGetDateUseNullTimezone
  # ==========================================================================

  Scenario Outline: should store the <property> client behavior property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter                       | property                  |
      | setTracing                   | tracing                   |
      | setApplication               | application               |
      | setClientConfigFile          | clientConfigFile          |
      | setAllowUnderscoresInHost    | allowUnderscoresInHost    |
      | setDisableGcsDefaultCredentials | isGcsDefaultCredentialsDisabled |
      | setArrowTreatDecimalAsInt    | treatDecimalAsInt         |
      | setStringsQuotedForColumnDef | stringsQuotedForColumnDef |
      | setEnablePutGet              | enablePutGet              |
      | setEnablePatternSearch       | enablePatternSearch       |
      | setOcspFailOpen              | ocspFailOpen              |

  Scenario Outline: should store the nullable <property> when <setter> is called with a non-null value
    Given a new Snowflake connection pool data source
    When <setter> is called with a non-null Boolean value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter                              | property                         |
      | setJDBCDefaultFormatDateWithTimezone| JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE|
      | setGetDateUseNullTimezone           | getDateUseNullTimezone           |

  Scenario Outline: should remove the nullable <property> when <setter> is called with null
    Given a Snowflake connection pool data source with the nullable <property> previously stored
    When <setter> is called with null
    Then the <property> is removed from the data source configuration

    Examples:
      | setter                              | property                         |
      | setJDBCDefaultFormatDateWithTimezone| JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE|
      | setGetDateUseNullTimezone           | getDateUseNullTimezone           |

  # ==========================================================================
  # TIMEOUTS & RETRIES
  # Methods: setNetworkTimeout, setQueryTimeout, setMaxHttpRetries,
  #          setPutGetMaxRetries, setBrowserResponseTimeout,
  #          setLoginTimeout, getLoginTimeout
  # Note: setBrowserResponseTimeout does not auto-promote the authenticator (BD#45).
  # ==========================================================================

  Scenario Outline: should store the <property> timeout or retry property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter                   | property               |
      | setNetworkTimeout        | networkTimeoutSeconds  |
      | setQueryTimeout          | queryTimeoutSeconds    |
      | setMaxHttpRetries        | maxHttpRetries         |
      | setPutGetMaxRetries      | putGetMaxRetries       |
      | setBrowserResponseTimeout| BROWSER_RESPONSE_TIMEOUT |

  Scenario: should round-trip the login timeout
    Given a new Snowflake connection pool data source
    When the login timeout is configured with setLoginTimeout
    Then getLoginTimeout returns the configured value

  Scenario: should not auto-promote the authenticator when the browser response timeout is set
    Given a new Snowflake connection pool data source
    When setBrowserResponseTimeout is called
    Then the browser response timeout is stored and the authenticator is left unset

  # ==========================================================================
  # DIAGNOSTICS
  # Methods: setEnableDiagnostics, setDiagnosticsAllowlistFile
  # ==========================================================================

  Scenario Outline: should store the <property> diagnostics property
    Given a new Snowflake connection pool data source
    When <setter> is called with a value
    Then the <property> is stored in the data source configuration

    Examples:
      | setter                    | property                 |
      | setEnableDiagnostics      | enableDiagnostics        |
      | setDiagnosticsAllowlistFile | diagnosticsAllowlistFile |

  # ==========================================================================
  # HTTP HEADER CUSTOMIZERS
  # Methods: setHttpHeadersCustomizers
  # Note: customizers are retained on the DataSource but not forwarded to
  # sf_core today (BD#43).
  # ==========================================================================

  Scenario: should retain HTTP header customizers on the data source
    Given a new Snowflake connection pool data source
    When setHttpHeadersCustomizers is called with a list of customizers
    Then the customizers are retained on the data source without being forwarded to sf_core

  # ==========================================================================
  # UNSUPPORTED CommonDataSource OPERATIONS
  # Methods: getLogWriter, setLogWriter, getParentLogger
  # ==========================================================================

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

  Scenario: should unwrap the data source to a supported interface
    Given a new Snowflake connection pool data source
    When isWrapperFor and unwrap are called with a supported interface
    Then isWrapperFor returns true and unwrap returns the data source instance

  Scenario: should reject unwrapping to an unsupported interface
    Given a new Snowflake connection pool data source
    When isWrapperFor and unwrap are called with an unsupported interface
    Then isWrapperFor returns false and unwrap throws a SQLException

  Scenario: should expose the configuration as a defensive copy of properties
    Given a Snowflake connection pool data source with configuration applied
    When getProperties is called and the returned map is mutated
    Then the data source's own configuration remains unchanged
