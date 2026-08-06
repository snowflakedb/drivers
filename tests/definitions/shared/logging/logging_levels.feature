@python @jdbc
Feature: Driver logging through core

  Verifies that logging is set up correctly and wrapper/core log levels are applied independently.
  Wrapper loggers cover driver-layer activity; the core logger receives events bridged from sf_core.
  Each layer honors its own configured threshold.

  @python_e2e @jdbc_e2e
  Scenario: should emit INFO logs at default levels
    Given Default logging levels
    When Query "SELECT 1 AS value" is executed
    Then Core logger emits an INFO log
    And Wrapper logger emits an INFO log

  @python_e2e @jdbc_e2e
  Scenario: should emit core DEBUG when core log level is DEBUG
    Given Logging is configured with wrapper log level INFO and core log level DEBUG
    When Query "SELECT 1 AS value" is executed
    Then Core logger emits a DEBUG log
    And Wrapper logger does not emit a DEBUG log but emits INFO log

  @python_e2e @jdbc_e2e
  Scenario: should emit wrapper DEBUG without core DEBUG when wrapper log level is DEBUG
    Given Logging is configured with wrapper log level DEBUG and core log level INFO
    When Query "SELECT 1 AS value" is executed
    Then Wrapper logger emits a DEBUG log
    And Core logger does not emit a DEBUG log but emits INFO log

  @python_e2e @jdbc_e2e
  Scenario: should emit wrapper and core DEBUG when both levels are DEBUG
    Given Logging is configured with wrapper log level DEBUG and core log level DEBUG
    When Query "SELECT 1 AS value" is executed
    Then Wrapper logger emits a DEBUG log
    And Core logger emits a DEBUG log
