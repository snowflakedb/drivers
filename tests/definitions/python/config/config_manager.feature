@python
Feature: ConfigManager Python Wrapper

  @python_int
  Scenario: default value
    Given No configuration files exist
    When ConfigManager retrieves an option with default value
    Then The default value should be returned

  @python_int
  Scenario: choices validation
    Given A ConfigOption with choices and an invalid default
    When ConfigManager retrieves the option
    Then ConfigSourceError should be raised

  @python_int
  Scenario: config value from file
    Given A config.toml file with a section and option
    When ConfigManager retrieves the option
    Then The file value should be returned

  @python_int
  Scenario: config value from file no env override
    Given A config.toml with section.mykey and SNOWFLAKE_SECTION_MYKEY env var
    When ConfigManager retrieves the option
    Then The file value should be returned (env overrides are not applied)

  @python_int
  Scenario: custom env name
    Given A custom environment variable CUSTOM_ENV_VAR is set
    When ConfigManager retrieves option with env_name set to CUSTOM_ENV_VAR
    Then The custom env var value should be returned

  @python_int
  Scenario: custom env name with parse str
    Given A custom environment variable set to a numeric string
    When ConfigManager retrieves option with parse_str set to int
    Then The parsed integer value should be returned

  @python_int
  Scenario: add option
    Given A ConfigManager instance
    When An option is added
    Then The option should be in the manager's options dict

  @python_int
  Scenario: add submanager
    Given A parent ConfigManager and a child ConfigManager
    When The child is added as a submanager
    Then The child nest_path and root_manager should be updated

  @python_int
  Scenario: conflict detection
    Given A ConfigManager with an option named conflict_name
    When Trying to add a submanager named conflict_name
    Then ConfigManagerError should be raised

  @python_int
  Scenario: getitem returns option value
    Given A ConfigManager with an option having a default value
    When Accessing the option via bracket notation
    Then The default value should be returned

  @python_int
  Scenario: clear cache
    Given A ConfigManager with cached config
    When clear_cache is called
    Then Cache should be None

  @python_int
  Scenario: config parser alias
    When Importing CONFIG_PARSER from config_manager
    Then DeprecationWarning should be raised
    And CONFIG_PARSER should reference CONFIG_MANAGER

  @python_int
  Scenario: sub parsers property
    Given A ConfigManager instance with a submanager
    When Accessing _sub_parsers property
    Then DeprecationWarning should be raised
    And _sub_parsers should reference _sub_managers

  @python_int
  Scenario: add subparser method
    Given A ConfigManager instance and a child
    When Calling add_subparser method
    Then DeprecationWarning should be raised
    And The child should be in _sub_managers
