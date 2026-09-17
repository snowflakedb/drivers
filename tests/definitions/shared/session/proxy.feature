@python @odbc @jdbc
Feature: HTTP proxy support

  The driver should route requests through an HTTP proxy when configured
  via connection parameters or environment variables.

  Scenarios are worded in driver-neutral terms so one scenario covers every
  wrapper that implements the behavior. Each wrapper spells the settings its
  own way — `proxy_host`/`proxy_port` in Python, `useProxy`/`proxyHost`/
  `proxyPort` in JDBC, `PROXY` in ODBC — and the per-wrapper spelling lives in
  the test, not in the scenario name.

  # ===========================================================================
  #                    Connection-parameter-driven routing
  # ===========================================================================

  @python_e2e @jdbc_e2e
  Scenario: should route request through proxy when proxy host and port are configured
    Given a forward-proxy WireMock serving a canned login response
    When the driver connects with the proxy host and port pointing at the proxy
    Then the connect succeeds and the proxy received the login request

  @python_e2e @odbc_e2e
  Scenario: should route request through proxy when proxy url is configured
    Given a forward-proxy WireMock serving a canned login response
    When the driver connects with the proxy url pointing at the proxy
    Then the connect succeeds and the proxy received the login request

  @python_e2e @jdbc_e2e
  Scenario: should bypass proxy when the target host is excluded from proxying
    Given a forward-proxy WireMock serving a canned login response
    When the driver connects with a proxy and the target host excluded from proxying
    Then the connect fails and the proxy received no requests

  # AllowEmptyProxy has no counterpart outside the ODBC DSN surface.
  @odbc_e2e
  Scenario: should disable proxy when PROXY is empty and AllowEmptyProxy is true
    Given a forward-proxy WireMock serving a canned login response
    When SQLDriverConnect is invoked with empty PROXY and AllowEmptyProxy=true
    Then the connect fails and the proxy received no requests

  # ===========================================================================
  #                    Environment-variable-driven routing
  # ===========================================================================

  @python_e2e @odbc_e2e
  Scenario: should route request through proxy when proxy env vars are enabled
    Given HTTP_PROXY env var points at a forward-proxy WireMock
    When the driver connects with proxy env vars enabled
    Then the connect succeeds and the proxy received the login request

  @python_e2e @odbc_e2e
  Scenario: should ignore proxy env vars by default
    Given HTTP_PROXY env var points at a forward-proxy WireMock
    When the driver connects without proxy env vars enabled
    Then the connect fails and the proxy received no requests

  # ===========================================================================
  #                    Precedence: params vs env vars
  # ===========================================================================

  @python_e2e
  Scenario: should prefer explicit proxy_host over HTTP_PROXY env var
    Given two forward-proxy WireMock instances are running
    And HTTP_PROXY env var points at the second proxy
    When the driver connects with proxy_host pointing at the first proxy
    Then only the first proxy received the login request
