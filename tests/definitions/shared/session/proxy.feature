@python @odbc
Feature: HTTP proxy support

  The driver should route requests through an HTTP proxy when configured
  via connection parameters or environment variables.

  # ===========================================================================
  #                    Connection-parameter-driven routing
  # ===========================================================================

  @python_e2e
  Scenario: should route request through proxy when proxy_host is configured
    Given a forward-proxy WireMock serving a canned login response
    When the driver connects with proxy_host and proxy_port pointing at the proxy
    Then the proxy received the login request

  @python_e2e
  Scenario: should route login through proxy using legacy ODBC PROXY URL
    Given a forward-proxy WireMock serving a canned login response
    When the driver connects with PROXY pointing at the proxy
    Then the proxy received the login request

  @odbc_e2e
  Scenario: should route login through forward proxy via PROXY URL
    Given a forward-proxy WireMock serving a canned login response
    When SQLDriverConnect is invoked with PROXY pointing at the proxy
    Then the connect succeeds and the proxy received exactly one login request

  @python_e2e
  Scenario: should bypass proxy when no_proxy matches the target host
    Given a forward-proxy WireMock serving a canned login response
    When the driver connects with proxy_host and no_proxy matching the target
    Then the connect fails and the proxy received no requests

  @odbc_e2e
  Scenario: should disable proxy when PROXY is empty and AllowEmptyProxy is true
    Given a forward-proxy WireMock serving a canned login response
    When SQLDriverConnect is invoked with empty PROXY and AllowEmptyProxy=true
    Then the connect fails and the proxy received no requests

  # ===========================================================================
  #                    Environment-variable-driven routing
  # ===========================================================================

  @python_e2e
  Scenario: should route request through proxy when use_proxy_env is true
    Given HTTP_PROXY env var points at a forward-proxy WireMock
    When the driver connects with use_proxy_env=True
    Then the proxy received the login request

  @python_e2e
  Scenario: should ignore HTTP_PROXY env var by default
    Given HTTP_PROXY env var points at a forward-proxy WireMock
    When the driver connects without use_proxy_env
    Then the connect fails and the proxy received no requests

  @odbc_e2e
  Scenario: should ignore HTTP_PROXY env var when USE_PROXY_ENV is not set
    Given HTTP_PROXY env var points at a forward-proxy WireMock
    When SQLDriverConnect is invoked without USE_PROXY_ENV
    Then the connect fails and the proxy received no requests

  @odbc_e2e
  Scenario: should pick up HTTP_PROXY env var when USE_PROXY_ENV is true
    Given HTTP_PROXY env var points at a forward-proxy WireMock
    When SQLDriverConnect is invoked with USE_PROXY_ENV=true
    Then the connect succeeds and the proxy received exactly one login request

  # ===========================================================================
  #                    Precedence: params vs env vars
  # ===========================================================================

  @python_e2e
  Scenario: should prefer explicit proxy_host over HTTP_PROXY env var
    Given two forward-proxy WireMock instances are running
    And HTTP_PROXY env var points at the second proxy
    When the driver connects with proxy_host pointing at the first proxy
    Then only the first proxy received the login request
