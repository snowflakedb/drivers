import gzip
import json

from pathlib import Path

from cryptography.hazmat.primitives.serialization import Encoding, NoEncryption, PrivateFormat, load_pem_private_key

from tests.compatibility import is_new_driver
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
        if not is_new_driver():
            pem_data = Path(get_test_private_key_path()).read_bytes()
            pk = load_pem_private_key(pem_data, password=None)
            extra_params["private_key"] = pk.private_bytes(Encoding.DER, PrivateFormat.PKCS8, NoEncryption())
            extra_params["private_key_file"] = None

        connection = int_test_connection_factory(server_url=wiremock.http_url(), **extra_params)
        connection.close()

        # Telemetry is sent asynchronously via a background task, so poll until it arrives.
        telemetry_requests = wiremock.wait_for_requests("/telemetry/send", min_count=1, timeout=2.0)
        assert len(telemetry_requests) >= 1, "Expected at least one POST to /telemetry/send after connection open"

        request = telemetry_requests[0]

        # Validate request path and method
        assert request["method"] == "POST"
        assert request["url"] == "/telemetry/send"

        # Validate required headers. HTTP header names are case-insensitive and
        # clients/libraries may add transport headers, so normalize and check subset.
        headers = request["headers"]
        normalized_headers = {name.lower(): value for name, value in headers.items()}
        required_header_names = {
            "authorization",
            "accept",
            "user-agent",
            "content-type",
            "host",
            "content-length",
            "content-encoding",
            "accept-encoding",
        }
        missing_headers = required_header_names - set(normalized_headers.keys())
        assert not missing_headers, f"Missing required headers: {missing_headers}"
        assert normalized_headers["authorization"].startswith("Snowflake Token=")
        assert normalized_headers["content-type"] == "application/json"
        assert normalized_headers["accept"] == "application/json"
        assert normalized_headers["user-agent"] is not None

        # Validate top-level payload structure.
        # Wiremock may transparently decompress gzip, so try both.
        raw_body = request["body"]
        if isinstance(raw_body, str):
            raw_body = raw_body.encode("latin-1")
        try:
            body = json.loads(gzip.decompress(raw_body))
        except gzip.BadGzipFile:
            body = json.loads(raw_body)
        assert "logs" in body, "Telemetry payload must contain 'logs' array"
        assert isinstance(body["logs"], list)
        assert len(body["logs"]) >= 1, "Expected at least one log entry"

        # Each log entry must have 'message' and 'timestamp'
        for entry in body["logs"]:
            assert "message" in entry, "Log entry must contain 'message'"
            assert "timestamp" in entry, "Log entry must contain 'timestamp'"
            assert isinstance(entry["message"], dict)
            assert isinstance(entry["timestamp"], str)
