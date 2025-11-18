@core
Feature: HTTP retry helper

  @core_int
  Scenario: should retry GET after transient failure
    Given a server that fails once then succeeds
    When the helper executes the request
    Then it should have retried once and returned the successful body

  @core_int
  Scenario: should fail when Retry-After exceeds deadline
    Given a retry policy with a tight deadline and a server that responds with a Retry-After that is after the deadline
    When the helper executes the request
    Then it should return a Retry-After exceeded error

  @core_int
  Scenario: should retry idempotent PUT after transient failure
    Given an idempotent PUT request that fails once then succeeds
    When the helper executes the request
    Then it should have retried once and returned the successful body

  @core_int
  Scenario: should fail after reaching max attempts
    Given a server that always fails with a retryable status
    When the helper executes the request
    Then it should return a max attempts error

