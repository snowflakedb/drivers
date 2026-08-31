@core @python
Feature: Workload Identity Federation Authentication

  @core_e2e
  Scenario: should authenticate with cloud identity using WIF
    Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is configured
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e
  Scenario: should authenticate with WIF using lowercase authenticator
    Given Authentication is set to workload_identity (lowercase) and a valid provider is configured
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e
  Scenario: should authenticate with WIF using impersonation
    Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_IMPERSONATION_PATH is configured
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e
  Scenario: should authenticate AWS WIF using pre-signed GetCallerIdentity by default
    Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
    And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is not set
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e
  Scenario: should authenticate AWS WIF using GetWebIdentityToken when opted in
    Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
    And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is set to true
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e
  Scenario: should authenticate AWS WIF using GetWebIdentityToken when workload_identity_aws_use_outbound_token is true
    Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
    And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is not set
    And workload_identity_aws_use_outbound_token is set to true
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e @python_int
  Scenario: should fail WORKLOAD_IDENTITY when provider is missing
    Given Authentication is set to WORKLOAD_IDENTITY but WORKLOAD_IDENTITY_PROVIDER is absent
    When Trying to Connect
    Then Connection fails with a missing-parameter error citing workload_identity_provider

  @core_e2e @python_int
  Scenario: should fail WORKLOAD_IDENTITY when provider is an invalid value
    Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is an invalid value
    When Trying to Connect
    Then Connection fails with an invalid-parameter error citing workload_identity_provider

  @core_e2e @python_int
  Scenario: should fail OIDC WIF when token is missing
    Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC but token is absent
    When Trying to Connect
    Then Connection fails with a missing-parameter error citing token

  @core_e2e @python_int
  Scenario: should fail OIDC WIF when token is malformed
    Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC
    And Token is set to a malformed value that is not a valid JWT
    When Trying to Connect
    Then Connection fails with an attestation error indicating a malformed token
