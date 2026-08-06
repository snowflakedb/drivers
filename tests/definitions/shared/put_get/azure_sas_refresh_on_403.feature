@core
Feature: Azure SAS refresh on 403

  An expired Azure SAS in a stage URL surfaces as HTTP 403. On ANY 403
  (status-only, so HTTP/2-safe) the driver re-issues the PUT/GET query for a
  fresh SAS, then retries the failed attempt.

  # Log severity tracks OUTCOME: recovered 403 -> debug; terminal failure ->
  # warn (non-token 403) or error (refresh itself failed), carrying status,
  # Azure <Code>, and a SAS-redacted URL.

  @core_int
  Scenario: should refresh SAS and succeed when Azure PUT returns 403 on the first attempt
    Given Snowflake client is logged in to an Azure-backed deployment
    And Stage SAS is configured to return HTTP 403 on the first PUT attempt
    When File is uploaded using PUT command
    Then The PUT query is re-issued to obtain a fresh stage credential
    And File should be uploaded successfully with the refreshed SAS
    And No warn-level log line is emitted for the recovered 403
    And The request body is rebuilt for the post-refresh attempt

  Scenario Outline: should recover when Azure PUT returns 403 regardless of the reason phrase
    # Any 403 triggers a SAS refresh; the reason phrase is irrelevant — the trigger is
    # status-only. Legacy Python only refreshes when the reason matches specific
    # token-expiry strings; UD refreshes on any 403 regardless of reason.
    Given Snowflake client is logged in to an Azure-backed deployment
    And Stage SAS is configured to return HTTP 403 with reason "<reason>"
    And the refresh returns a SAS that Azure accepts
    When File is uploaded using PUT command
    Then The PUT query is re-issued to obtain a fresh stage credential
    And The refreshed SAS is accepted
    And The upload succeeds

    Examples:
      | reason                                                |
      | Signature not valid in the specified time frame.     |
      | Server failed to authenticate the request.           |
      | Signature fields not well formed.                    |
      |                                                       |

  # Azure download is a single non-Range GET; a 403 surfaces before any body bytes.
  @core_int
  Scenario: should refresh SAS and re-drive the GET once when Azure returns 403
    Given Snowflake client is logged in to an Azure-backed deployment
    And File is uploaded to an Azure-backed stage
    And Stage SAS is configured to return HTTP 403 on the GET
    When File is downloaded using GET command
    Then The GET query is re-issued to obtain a fresh stage credential
    And The GET is re-driven exactly once carrying the refreshed SAS
    And File should be downloaded with correct content
    And No warn-level log line is emitted for the recovered 403

  @core_int
  Scenario: should surface terminal error when GET SAS refresh itself fails
    Given Snowflake client is logged in to an Azure-backed deployment
    And File is uploaded to an Azure-backed stage
    And Stage SAS is configured to return HTTP 403 on the GET
    And Snowflake GS is unreachable for the refresh query
    When File is downloaded using GET command
    Then The GET query is re-issued to obtain a fresh stage credential
    And An error is raised indicating SAS refresh failed
    And An error-level log line is emitted naming the refresh-failure reason

  @core_int
  Scenario: should retry then fail when Azure GET 403 is not caused by SAS expiry
    # UD's status-only trigger always fires one refresh, even for non-token 403s.
    # Legacy Python's reason-gate would skip refresh entirely on the GET path.
    # If the re-driven GET also returns 403 (bucket-policy denial), UD surfaces
    # the terminal 403 at warn — one GS round-trip more than legacy Python.
    Given Snowflake client is logged in to an Azure-backed deployment
    And File is uploaded to an Azure-backed stage
    And Stage SAS is configured to return HTTP 403 on the GET for a non-token reason
    When File is downloaded using GET command
    Then The GET query is re-issued to obtain a fresh stage credential
    And The re-driven GET is also rejected with HTTP 403
    And An error is raised indicating Azure storage returned HTTP 403
    And A warn-level log line is emitted at status 403
    And The warn log names the Azure error code
    And The warn log carries a SAS-redacted URL

  @core_int
  Scenario: should recover both concurrent PUTs via the shared refreshed SAS
    Given Snowflake client is logged in to an Azure-backed deployment
    And Two PUT operations are running in parallel against the same Azure stage
    And Stage SAS is configured to return HTTP 403 for both concurrent operations
    When Both PUT operations trigger SAS refresh concurrently
    Then Both PUTs carry the shared refreshed SAS at the wire
    And Both operations succeed with the shared refreshed SAS

  @core_int
  Scenario: should surface terminal error when PUT SAS refresh itself fails
    Given Snowflake client is logged in to an Azure-backed deployment
    And Stage SAS is configured to return HTTP 403 on the PUT
    And Snowflake GS is unreachable for the refresh query
    When File is uploaded using PUT command
    Then The PUT query is re-issued to obtain a fresh stage credential
    And An error is raised indicating SAS refresh failed
    And An error-level log line is emitted naming the refresh-failure reason

  @core_int
  Scenario: should retry then fail when Azure PUT 403 is not caused by SAS expiry
    # Any 403 triggers one refresh; if the new SAS still 403s (e.g. bucket-policy
    # denial), the terminal 403 surfaces and logs at warn (the contract-drift signal).
    Given Snowflake client is logged in to an Azure-backed deployment
    And Stage SAS is configured to return HTTP 403 for a non-token reason
    When File is uploaded using PUT command
    Then The PUT query is re-issued to obtain a fresh stage credential
    And The refreshed SAS is also rejected with HTTP 403
    And An error is raised indicating Azure storage returned HTTP 403
    And A warn-level log line is emitted at status 403
    And The warn log names the Azure error code
    And The warn log carries a SAS-redacted URL

  # Multi-chunk PUT mid-batch SAS resume (SNOW-3406384).
  Scenario: should resume a multi-chunk PUT from the failed chunk after a mid-batch SAS expiry
    Given Snowflake client is logged in to an Azure-backed deployment
    And A multi-chunk PUT upload is in progress against an Azure stage
    And Stage SAS is configured to return HTTP 403 on a chunk after the first
    When File is uploaded using PUT command
    Then The PUT resumes from the failed chunk carrying the refreshed SAS
    And File should be uploaded successfully

  # Coalescing beyond the single-cursor case (>2 parallel cursors, queued ops).
  Scenario: should coalesce SAS refresh across more than two parallel cursors
    Given Snowflake client is logged in to an Azure-backed deployment
    And More than two PUT operations are running in parallel against the same Azure stage
    And Stage SAS is configured to return HTTP 403 for all of them
    When They trigger SAS refresh concurrently
    Then Only one PUT-refresh query round-trip is made to Snowflake GS
    And All operations succeed with the shared refreshed SAS

  # Contract-drift telemetry event (deferred follow-up wave).
  Scenario: should emit a contract-drift telemetry event when a 403 is outside the token-expiry set
    Given Snowflake client is logged in to an Azure-backed deployment
    And Stage SAS is configured to return HTTP 403 whose Azure error code is outside the known token-expiry set
    And The 403 reason is also outside the known token-expiry set
    When File is uploaded using PUT command
    Then A structured contract-drift event is emitted, labelled by the parsed Azure error code
    And The terminal 403 still surfaces to the caller
