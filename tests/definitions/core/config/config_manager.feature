@core
Feature: Config Manager Core (TOML Loading)

  @core_int
  Scenario: connection load from config basic
    Given A connections.toml file with test_connection defined
    When sf_core loads the connection config
    Then The connection settings should be loaded

  @core_int
  Scenario: explicit setting overrides config
    Given A connections.toml with connection having account setting
    And An explicit account setting on the connection
    When sf_core loads the connection config
    Then The explicit setting should take precedence

  @core_int
  Scenario: connection not found in config
    Given No configuration files exist
    When sf_core loads connection named nonexistent
    Then ConnectionNotFound error should be returned

  @core_int
  Scenario: config precedence
    Given A config.toml with connection account set to config_account
    And A connections.toml with same connection account set to connections_account
    When sf_core loads the connection config
    Then connections.toml values should override config.toml

  @core_int
  Scenario: insecure permissions rejected
    Given A connections.toml file with insecure permissions
    When sf_core loads the connection config
    Then An insecure permissions error should be returned

  @core_int
  Scenario: multiple data types
    Given A connections.toml with string, integer, float, and boolean values
    When sf_core loads the connection config
    Then Each value should be parsed to the correct Setting type

  @core_int
  Scenario: empty config files
    Given Empty config.toml and connections.toml files
    When sf_core loads connection named testconn
    Then ConnectionNotFound error should be returned

  @core_int
  Scenario: load log section
    Given A config.toml with a log section
    When sf_core loads the log section
    Then The log settings should be returned

  @core_int
  Scenario: load multiple sections
    Given A config.toml with log, proxy, and retry sections
    When sf_core loads all config sections
    Then All sections should be returned including connections

  @core_int
  Scenario: connections toml does not override log section
    Given A config.toml with log section and a connections.toml with log section
    When sf_core loads the log config section
    Then The config.toml log values should be used

  @core_int
  Scenario: load nonexistent section
    Given A config.toml with a log section
    When sf_core loads a nonexistent section
    Then None should be returned

  @core_int
  Scenario: cannot load connections via load config section
    Given A config.toml with connections section
    When sf_core loads section connections
    Then None should be returned

  @core_int
  Scenario: load nested config section
    Given A config.toml with nested sections like database.connection
    When sf_core loads a nested section by dotted path
    Then The nested section settings should be returned

  @core_int
  Scenario: nested connections blocked
    Given A config.toml with connections.dev and connections.prod
    When sf_core loads section connections.dev
    Then None should be returned

  @core_int
  Scenario: nonexistent nested section
    Given A config.toml with database.connection section
    When sf_core loads section database.pool
    Then None should be returned
