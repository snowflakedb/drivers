@python
Feature: ConfigManager Python Wrapper

  @python_int
  Scenario: should use default value when config file is missing
    Given No configuration files exist
    When ConfigManager retrieves an option with default value
    Then The default value should be returned

  @python_int
  Scenario: should use custom environment variable name
    Given Environment variable CUSTOM_VAR is set
    And Environment variable SNOWFLAKE_TEST_OPTION is also set
    When ConfigManager retrieves option with env_name set to CUSTOM_VAR
    Then The CUSTOM_VAR value should be returned

  @python_int
  Scenario: should ignore environment variable when env_name is False
    Given A config.toml file with test_option setting
    And Environment variable SNOWFLAKE_TEST_OPTION is set
    When ConfigManager retrieves option with env_name set to False
    Then The config file value should be returned without env override

  @python_int
  Scenario: should validate option value against choices
    Given Environment variable is set to invalid choice
    When ConfigManager retrieves option with choices validation
    Then ConfigSourceError should be raised

  @python_int
  Scenario: should accept valid choice value
    Given Environment variable is set to valid choice
    When ConfigManager retrieves option with choices validation
    Then The valid choice value should be returned

  @python_int
  Scenario: should parse environment variable string to integer
    Given Environment variable SNOWFLAKE_TIMEOUT is set to string "300"
    When ConfigManager retrieves option with parse_str set to int
    Then Integer value 300 should be returned

  @python_int
  Scenario: should parse environment variable string to JSON
    Given Environment variable is set to JSON string
    When ConfigManager retrieves option with parse_str set to json.loads
    Then Parsed dictionary should be returned

  @python_int
  Scenario: should create nested configuration managers
    Given A ConfigManager named root
    When A sub-manager named child is added
    Then The child nest_path should include parent path
    And The child root_manager should reference root

  @python_int
  Scenario: should detect naming conflicts between options and sub-managers
    Given A ConfigManager with option named conflict_name
    When Trying to add sub-manager named conflict_name
    Then ConfigManagerError should be raised

  @python_int
  Scenario: should raise error when default connection not found
    Given A connections.toml without default connection
    And default_connection_name is set to default
    When Getting default connection params
    Then Error should be raised with connection not found message

  @python_int
  Scenario: should emit deprecation warning for CONFIG_PARSER alias
    When Importing CONFIG_PARSER from config_manager
    Then DeprecationWarning should be raised
    And CONFIG_PARSER should reference CONFIG_MANAGER

  @python_int
  Scenario: should emit deprecation warning for _sub_parsers property
    Given A ConfigManager instance
    When Accessing _sub_parsers property
    Then DeprecationWarning should be raised
    And _sub_parsers should reference _sub_managers

  @python_int
  Scenario: should emit deprecation warning for add_subparser method
    Given A ConfigManager instance
    When Calling add_subparser method
    Then DeprecationWarning should be raised
    And Sub-manager should be added to _sub_managers

  @python_int
  Scenario: should decode base64 bytes setting from Rust response
    Given Rust sf_core returns base64 encoded bytes setting
    When Python parses the setting JSON
    Then Decoded bytes value should be returned

  @python_int
  Scenario: should generate correct option_name for nested options
    Given A ConfigManager hierarchy root -> level1 -> level2
    And An option named my_option at level2
    When Accessing option_name property
    Then It should return level1.level2.my_option

  @python_int
  Scenario: should generate correct default_env_name
    Given A ConfigManager hierarchy root -> database -> connection
    And An option named timeout at connection
    When Accessing default_env_name property
    Then It should return SNOWFLAKE_DATABASE_CONNECTION_TIMEOUT

  @python_int
  Scenario: should raise error for missing required option
    Given No configuration files exist
    And No environment variable is set
    When ConfigManager retrieves option without default
    Then MissingConfigOptionError should be raised

  @python_int
  Scenario: should access option value through getitem
    Given A ConfigManager with option test_option
    When Accessing manager using bracket notation
    Then The option value should be returned

  @python_int
  Scenario: should access sub-manager through getitem
    Given A ConfigManager with sub-manager child
    When Accessing manager["child"]
    Then The child ConfigManager should be returned

  @python_int
  Scenario: should raise error for non-existent item in getitem
    Given A ConfigManager with no options or sub-managers
    When Accessing manager["non_existent"]
    Then ConfigSourceError should be raised
