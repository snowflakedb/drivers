import json
import platform

import pytest

from tests.compatibility import is_old_driver
from tests.wiremock_client import WiremockClient


pytestmark = pytest.mark.skipif(is_old_driver(), reason="Universal driver only")


def test_login_request_contains_correct_client_identity(int_test_connection_factory):
    """Verify the Python wrapper sends correct client identity in the login request.

    Uses Wiremock to intercept the POST to /session/v1/login-request and validates
    CLIENT_APP_ID, CLIENT_APP_VERSION, User-Agent, and CLIENT_ENVIRONMENT fields
    match the expected legacy Python connector values.
    """
    with WiremockClient().start() as wiremock:
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
