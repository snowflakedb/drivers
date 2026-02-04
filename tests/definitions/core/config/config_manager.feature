@core
Feature: Config Manager Core (TOML Loading)

  @core_int
  Scenario: should load configuration from config.toml file
    Given A config.toml file with account setting
    When sf_core loads the configuration
    Then The account value should be read from the file

  @core_int
  Scenario: should load connections from connections.toml file
    Given A connections.toml file with test_connection defined
    When sf_core loads all sections
    Then The test_connection should be available under connections prefix

  @core_int
  Scenario: should merge connections from both config files
    Given A config.toml with connection having account setting
    And A connections.toml with same connection having user setting
    When sf_core loads the connection
    Then Both account and user settings should be present

  @core_int
  Scenario: should prioritize connections.toml over config.toml
    Given A config.toml with connection account set to config_account
    And A connections.toml with same connection account set to conn_account
    When sf_core loads the connection
    Then The account should be conn_account

  @core_int
  Scenario: should parse string setting type
    Given A config file with string value
    When sf_core parses the TOML
    Then Setting type should be String

  @core_int
  Scenario: should parse integer setting type
    Given A config file with integer value
    When sf_core parses the TOML
    Then Setting type should be Int

  @core_int
  Scenario: should parse float setting type
    Given A config file with float value
    When sf_core parses the TOML
    Then Setting type should be Double

  @core_int
  Scenario: should convert boolean to string setting
    Given A config file with boolean value
    When sf_core parses the TOML
    Then Setting type should be String with value "true" or "false"

  @core_int
  Scenario: should return error for non-existent connection
    Given No configuration files exist
    When sf_core loads connection named nonexistent
    Then ConnectionNotFound error should be returned

  @core_int
  Scenario: should load nested sections from config.toml
    Given A config.toml with nested section database.pool
    When sf_core loads section database.pool
    Then The nested section settings should be returned

  @core_int
  Scenario: should exclude connections from load_config_section
    Given A config.toml with connections section
    When sf_core loads section connections
    Then None should be returned

  @core_int
  Scenario: should override config value with SNOWFLAKE_SECTION_KEY environment variable
    Given A config.toml with key FOO in section BAR set to file_value
    And Environment variable SNOWFLAKE_BAR_FOO is set to env_value
    When sf_core loads all sections
    Then The value of FOO in section BAR should be env_value

  @core_int
  Scenario: should skip environment variable overrides when disabled
    Given A config.toml with key FOO in section BAR set to file_value
    And Environment variable SNOWFLAKE_BAR_FOO is set to env_value
    When sf_core loads all sections with apply_env_overrides=false
    Then The value of FOO in section BAR should be file_value
