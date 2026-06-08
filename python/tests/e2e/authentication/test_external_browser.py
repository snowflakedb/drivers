"""
External browser authentication E2E test.

Requires the snowdrivers-test-external-browser-universal-driver Docker container
(headless Chromium + /externalbrowser/provideBrowserCredentials.js).

Run locally:
  ./ci/auth/run_auth_browser_python.sh
"""

import pytest

from .auth_helpers import (
    clean_browser_processes,
    connect_with_browser_automation,
    verify_simple_query_execution,
)


@pytest.fixture(autouse=True)
def browser_cleanup():
    clean_browser_processes()
    yield
    clean_browser_processes()


@pytest.mark.requires_browser
class TestExternalBrowserAuthentication:
    def test_should_authenticate_with_external_browser_via_okta_idp(self, connection_factory, browser_params):
        # Given External browser authentication is configured with valid Okta user
        connect_params = {
            "host": browser_params["host"],
            "account": browser_params["account"],
            "user": browser_params["browser_user"],
            "authenticator": "externalbrowser",
            "role": browser_params["role"],
            "database": browser_params["database"],
            "schema": browser_params["schema"],
            "warehouse": browser_params["warehouse"],
            "client_store_temporary_credential": False,
        }

        # When Trying to Connect with headless browser providing valid credentials
        connection = connect_with_browser_automation(
            connect_fn=lambda: connection_factory(**connect_params),
            scenario="success",
            login=browser_params["okta_login"],
            password=browser_params["okta_password"],
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)
