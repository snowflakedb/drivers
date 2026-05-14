import json
import platform

import pytest

from tests.compatibility import is_old_driver


pytestmark = pytest.mark.skipif(is_old_driver(), reason="Universal driver only")


def _login_request_data(wiremock) -> dict:
    login_requests = wiremock.get_requests("/session/v1/login-request.*")
    assert login_requests, "Expected at least one login request"
    return json.loads(login_requests[0]["body"])["data"]


def test_login_request_contains_correct_client_identity(int_test_connection_factory, wiremock):
    """Verify the Python wrapper sends correct client identity in the login request.

    Uses Wiremock to intercept the POST to /session/v1/login-request and validates
    CLIENT_APP_ID, CLIENT_APP_VERSION, User-Agent, and CLIENT_ENVIRONMENT fields
    match the expected legacy Python connector values.
    """
    wiremock.add_mapping("auth/login_success_jwt.json")

    connection = int_test_connection_factory(server_url=wiremock.http_url())
    connection.close()

    login_requests = wiremock.get_requests("/session/v1/login-request.*")
    assert len(login_requests) >= 1, "Expected at least one login request"

    request = login_requests[0]
    body = json.loads(request["body"])
    data = body["data"]

    # CLIENT_APP_ID must match the legacy Python connector value
    assert data["CLIENT_APP_ID"] == "PythonConnector"

    # CLIENT_APP_VERSION is stripped to digits-only for server compat;
    # CLIENT_APP_VERSION_FULL preserves the original (e.g. "5.0.0b1").
    from snowflake.connector.version import VERSION, __version__

    assert data["CLIENT_APP_VERSION_FULL"] == __version__
    # Stripped version must match the release components (no "dev", "rc", etc.)
    stripped = data["CLIENT_APP_VERSION"]
    assert stripped == ".".join(str(c) for c in VERSION[:-1])

    # CLIENT_ENVIRONMENT must contain correct OS and runtime fields
    env = data["CLIENT_ENVIRONMENT"]
    assert env["APPLICATION"] == "PythonConnector"
    assert env["OS"], "OS must not be empty"
    assert env["OS_VERSION"], "OS_VERSION must not be empty"
    # Values are trimmed by the Rust core before storing
    assert env["RUNTIME_NAME"] == platform.python_implementation().strip()
    assert env["RUNTIME_VERSION"] == platform.python_version().strip()
    assert env["COMPILER"] == platform.python_compiler().strip()

    # User-Agent header must identify the driver
    headers = {k.lower(): v for k, v in request["headers"].items()}
    user_agent = headers["user-agent"]
    assert user_agent.startswith(f"PythonConnector/{__version__}")
    assert platform.python_implementation() in user_agent
    assert platform.python_version() in user_agent


def test_custom_application_only_affects_client_environment_application(int_test_connection_factory, wiremock):
    """User-supplied ``application`` goes into CLIENT_ENVIRONMENT.APPLICATION.
    CLIENT_APP_ID must stay as the driver name so server-side feature gating
    tied to the client type keeps working (mirrors the old connector)."""
    wiremock.add_mapping("auth/login_success_jwt.json")

    connection = int_test_connection_factory(
        server_url=wiremock.http_url(),
        application="SNOWCLI.STAGE.COPY",
    )
    try:
        data = _login_request_data(wiremock)
        assert data["CLIENT_APP_ID"] == "PythonConnector"
        assert data["CLIENT_ENVIRONMENT"]["APPLICATION"] == "SNOWCLI.STAGE.COPY"
    finally:
        connection.close()
