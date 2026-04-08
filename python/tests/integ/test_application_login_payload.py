"""
Integration tests: application parameter mapping in login request payload.

In the old connector, `application` and `internal_application_name` are separate:
  - CLIENT_APP_ID            ← internal_application_name (always "PythonConnector")
  - CLIENT_ENVIRONMENT.APPLICATION ← application (user-customisable)

These tests verify that the new connector preserves this separation by
inspecting the raw login-request JSON body via Wiremock's request journal.
"""

from __future__ import annotations

import json

import pytest
import requests

from tests.wiremock_client import WiremockClient


def _get_login_request_body(wiremock: WiremockClient) -> dict:
    """Return the parsed JSON body of the most recent login-request captured by Wiremock."""
    journal_url = f"{wiremock.http_url()}/__admin/requests"
    resp = requests.get(journal_url, timeout=5)
    assert resp.status_code == 200, f"Failed to fetch request journal: {resp.text}"

    all_requests = resp.json().get("requests", [])
    login_requests = [
        r for r in all_requests if "/session/v1/login-request" in r.get("request", {}).get("url", "")
    ]
    assert login_requests, "No login-request captured by Wiremock"

    body_text = login_requests[0]["request"]["body"]
    return json.loads(body_text)


class TestApplicationLoginPayload:
    """Verify CLIENT_APP_ID and CLIENT_ENVIRONMENT.APPLICATION in the login request."""

    def test_default_application_sends_python_connector_everywhere(self, int_test_connection_factory):
        """Without an explicit application param, both CLIENT_APP_ID and
        CLIENT_ENVIRONMENT.APPLICATION should be 'PythonConnector'."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            conn = int_test_connection_factory(server_url=wiremock.http_url())
            try:
                body = _get_login_request_body(wiremock)
                data = body["data"]

                assert data["CLIENT_APP_ID"] == "PythonConnector"
                assert data["CLIENT_ENVIRONMENT"]["APPLICATION"] == "PythonConnector"
            finally:
                conn.close()

    def test_custom_application_only_affects_client_environment(self, int_test_connection_factory):
        """When application='SNOWCLI.STAGE.COPY', CLIENT_APP_ID must stay
        'PythonConnector' while CLIENT_ENVIRONMENT.APPLICATION is customised."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            conn = int_test_connection_factory(
                server_url=wiremock.http_url(),
                application="SNOWCLI.STAGE.COPY",
            )
            try:
                body = _get_login_request_body(wiremock)
                data = body["data"]

                assert data["CLIENT_APP_ID"] == "PythonConnector", (
                    "CLIENT_APP_ID must always be the driver name, not the user's application"
                )
                assert data["CLIENT_ENVIRONMENT"]["APPLICATION"] == "SNOWCLI.STAGE.COPY", (
                    "CLIENT_ENVIRONMENT.APPLICATION should reflect the user's application parameter"
                )
            finally:
                conn.close()

    def test_application_property_reflects_custom_value(self, int_test_connection_factory):
        """conn.application should return the user-supplied value."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            conn = int_test_connection_factory(
                server_url=wiremock.http_url(),
                application="MyCustomApp",
            )
            try:
                assert conn.application == "MyCustomApp"
            finally:
                conn.close()

    def test_application_property_defaults_to_python_connector(self, int_test_connection_factory):
        """conn.application should default to 'PythonConnector' when not specified."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            conn = int_test_connection_factory(server_url=wiremock.http_url())
            try:
                assert conn.application == "PythonConnector"
            finally:
                conn.close()
