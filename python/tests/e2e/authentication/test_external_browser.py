"""
External browser authentication E2E test.

Requires the snowdrivers-test-external-browser-universal-driver Docker container
(headless Chromium + /externalbrowser/provideBrowserCredentials.js).

Run locally:
  ./tests/auth/run_auth_browser.sh python
"""

from dataclasses import dataclass

import pytest

from .auth_helpers import (
    connect_with_browser_automation,
    verify_simple_query_execution,
)
from .conftest import require_auth_params


@dataclass(frozen=True)
class BrowserCredentials:
    """External-browser (Okta IdP) credentials.

    ``user`` is the Snowflake login and also the value the browser automation types
    into the Okta form; ``password`` is the IdP password (entered by the browser, not
    a connection parameter).
    """

    user: str
    password: str

    def connect_params(self, **overrides) -> dict:
        params = {
            "user": self.user,
            "authenticator": "EXTERNALBROWSER",
            "role": "PUBLIC",
        }
        params.update(overrides)
        return params


@pytest.fixture(scope="module")
def browser_credentials() -> BrowserCredentials:
    """External browser test credentials. Fails if credentials are missing."""
    values = require_auth_params("SNOWFLAKE_TEST_OKTA_USER", "SNOWFLAKE_TEST_OKTA_PASSWORD")
    return BrowserCredentials(
        user=values["SNOWFLAKE_TEST_OKTA_USER"],
        password=values["SNOWFLAKE_TEST_OKTA_PASSWORD"],
    )


@pytest.mark.requires_browser
class TestExternalBrowserAuthentication:
    def test_should_authenticate_with_external_browser_via_okta_idp(self, connection_factory, browser_credentials):
        # Given External browser authentication is configured with valid Okta user
        connect_params = browser_credentials.connect_params(client_store_temporary_credential=False)

        # When Trying to Connect with headless browser providing valid credentials
        connection = connect_with_browser_automation(
            connect_fn=lambda: connection_factory(**connect_params),
            scenario="success",
            login=browser_credentials.user,
            password=browser_credentials.password,
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)
