import json

from pathlib import Path

from cryptography.hazmat.primitives.serialization import Encoding, NoEncryption, PrivateFormat, load_pem_private_key

from tests.compatibility import IS_UNIVERSAL_DRIVER
from tests.private_key_helper import get_test_private_key_path
from tests.wiremock_client import WiremockClient


def test_session_init_telemetry_sent_on_connection_open(int_test_connection_factory):
    """Verify that a session_init telemetry event is sent when a connection is opened.

    This test uses Wiremock to intercept the POST to /telemetry/send and validates
    the request path, headers, and top-level payload structure. It runs against both
    the universal and old drivers for comparison.
    """
    with WiremockClient().start() as wiremock:
        wiremock.add_mapping("auth/login_success_jwt.json")
        wiremock.add_mapping("telemetry/telemetry_send_success.json")

        # The old driver needs private_key as DER bytes; the universal driver uses private_key_file
        extra_params = {}
        if not IS_UNIVERSAL_DRIVER:
            pem_data = Path(get_test_private_key_path()).read_bytes()
            pk = load_pem_private_key(pem_data, password=None)
            extra_params["private_key"] = pk.private_bytes(Encoding.DER, PrivateFormat.PKCS8, NoEncryption())
            extra_params["private_key_file"] = None

        connection = int_test_connection_factory(server_url=wiremock.http_url(), **extra_params)
        connection.close()

        # Query Wiremock for captured telemetry requests
        telemetry_requests = wiremock.get_requests("/telemetry/send")
        assert len(telemetry_requests) >= 1, "Expected at least one POST to /telemetry/send after connection open"

        request = telemetry_requests[0]

        # Validate request path and method
        assert request["method"] == "POST"
        assert request["url"] == "/telemetry/send"

        # Validate headers — only driver-set headers plus standard transport headers
        headers = request["headers"]
        expected_header_names = {
            "Authorization",
            "Accept",
            "User-Agent",
            "Content-Type",
            "Host",
            "Content-Length",
            "Connection",
            "Content-Encoding",
            "Accept-Encoding",
        }
        assert set(headers.keys()) == expected_header_names, (
            f"Unexpected headers: {set(headers.keys()) - expected_header_names}, "
            f"Missing headers: {expected_header_names - set(headers.keys())}"
        )
        assert headers["Authorization"].startswith("Snowflake Token=")
        assert headers["Content-Type"] == "application/json"
        assert headers["Accept"] == "application/json"
        assert headers["User-Agent"] is not None

        # Validate top-level payload structure
        body = json.loads(request["body"])
        assert "logs" in body, "Telemetry payload must contain 'logs' array"
        assert isinstance(body["logs"], list)
        assert len(body["logs"]) >= 1, "Expected at least one log entry"

        # Each log entry must have 'message' and 'timestamp'
        for entry in body["logs"]:
            assert "message" in entry, "Log entry must contain 'message'"
            assert "timestamp" in entry, "Log entry must contain 'timestamp'"
            assert isinstance(entry["message"], dict)
            assert isinstance(entry["timestamp"], str)
