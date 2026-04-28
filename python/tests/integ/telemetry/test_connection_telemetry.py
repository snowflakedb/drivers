import gzip
import json

from pathlib import Path

import pytest

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

        connection = int_test_connection_factory(server_url=wiremock.http_url(), **_jwt_private_key_params())
        connection.close()

        # Telemetry is exported synchronously on connection release via
        # SimpleSpanProcessor. Poll briefly in case of timing variability.
        telemetry_requests = wiremock.wait_for_requests("/telemetry/send", min_count=1, timeout=5.0)
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

        body = _decode_telemetry_body(request)
        assert "logs" in body, "Telemetry payload must contain 'logs' array"
        assert isinstance(body["logs"], list)
        assert len(body["logs"]) >= 1, "Expected at least one log entry"

        # Each log entry must have 'message' and 'timestamp'
        for entry in body["logs"]:
            assert "message" in entry, "Log entry must contain 'message'"
            assert "timestamp" in entry, "Log entry must contain 'timestamp'"
            assert isinstance(entry["message"], dict)
            assert isinstance(entry["timestamp"], str)


@pytest.mark.skip_reference(reason="api_usage telemetry is universal-driver only")
def test_api_usage_telemetry_sent_on_cursor_creation(int_test_connection_factory):
    """Verify that an api_usage telemetry event is recorded when a cursor is created.

    ``Connection.cursor`` is decorated with ``@api_telemetry``, which calls
    ``TelemetryClient.send_api_usage('Connection.cursor')``. sf_core records
    this as an ``api_call`` span event with an ``api_method`` attribute, which
    the Snowflake exporter then serializes into a log entry on
    ``/telemetry/send`` with ``message.type == 'api_call'`` and
    ``message.api_method == 'Connection.cursor'``.
    """
    with WiremockClient().start() as wiremock:
        wiremock.add_mapping("auth/login_success_jwt.json")
        wiremock.add_mapping("telemetry/telemetry_send_success.json")

        connection = int_test_connection_factory(server_url=wiremock.http_url(), **_jwt_private_key_params())
        try:
            cursor = connection.cursor()
            cursor.close()
        finally:
            # Closing the connection flushes pending telemetry spans via
            # SimpleSpanProcessor, so it must happen before we poll wiremock.
            connection.close()

        telemetry_requests = wiremock.wait_for_requests("/telemetry/send", min_count=1, timeout=5.0)
        assert len(telemetry_requests) >= 1, "Expected at least one POST to /telemetry/send after cursor creation"

        log_entries = _collect_log_entries(telemetry_requests)

        cursor_entries = [
            entry
            for entry in log_entries
            if entry["message"].get("type") == "api_call" and entry["message"].get("api_method") == "Connection.cursor"
        ]
        assert len(cursor_entries) == 1, (
            "Expected exactly one api_call telemetry log entry with "
            f"api_method='Connection.cursor'. Got entries: {log_entries}"
        )

        entry = cursor_entries[0]

        # Log envelope: {"message": {...}, "timestamp": "<epoch_millis>"}
        assert set(entry.keys()) == {"message", "timestamp"}, f"Unexpected log entry envelope keys: {set(entry.keys())}"
        assert isinstance(entry["timestamp"], str) and entry["timestamp"].isdigit(), (
            f"timestamp must be a decimal epoch-millis string, got: {entry['timestamp']!r}"
        )

        message = entry["message"]

        # Event attributes (from record_api_call) + inherited connection-span
        # attributes. `snowflake.session.id` comes from the login mapping
        # (tests/wiremock/mappings/auth/login_success_jwt.json -> sessionId).
        # The `code.*`, `busy_ns`, `idle_ns`, `thread.id` attributes are
        # auto-populated by `tracing::info_span!` at
        # sf_core/src/apis/database_driver_v1/connection.rs where the
        # "connection" span is created.
        expected_exact = {
            "type": "api_call",
            "api_method": "Connection.cursor",
            "snowflake.session.id": 12345,
            "code.filepath": "sf_core/src/apis/database_driver_v1/connection.rs",
            "code.namespace": "sf_core::apis::database_driver_v1::connection",
        }
        for key, expected in expected_exact.items():
            actual = message.get(key)
            # Normalize path separators for cross-platform compatibility (Windows uses backslashes)
            if key == "code.filepath" and isinstance(actual, str):
                actual = actual.replace("\\", "/")
            assert actual == expected, (
                f"api_call message[{key!r}] expected {expected!r}, got {message.get(key)!r}. Full message: {message}"
            )

        numeric_attrs = {"code.lineno", "busy_ns", "idle_ns", "thread.id"}
        for key in numeric_attrs:
            assert isinstance(message.get(key), int), (
                f"api_call message[{key!r}] expected int, got {type(message.get(key)).__name__}: {message.get(key)!r}"
            )

        # Guard against the event silently gaining or losing attributes: the
        # api_call message is the union of the exact and numeric attribute
        # sets above — nothing more, nothing less.
        assert set(message.keys()) == set(expected_exact.keys()) | numeric_attrs, (
            f"Unexpected api_call message keys: {sorted(message.keys())}"
        )


def _jwt_private_key_params() -> dict:
    """Return connection overrides needed for JWT auth against Wiremock.

    The old driver needs ``private_key`` as DER bytes; the universal driver
    accepts ``private_key_file`` directly.
    """
    if is_new_driver():
        return {}
    pem_data = Path(get_test_private_key_path()).read_bytes()
    pk = load_pem_private_key(pem_data, password=None)
    return {
        "private_key": pk.private_bytes(Encoding.DER, PrivateFormat.PKCS8, NoEncryption()),
        "private_key_file": None,
    }


def _decode_telemetry_body(request: dict) -> dict:
    """Decode the JSON body of a Wiremock-captured ``/telemetry/send`` request.

    Wiremock may transparently decompress gzip, so try both.
    """
    raw_body = request["body"]
    if isinstance(raw_body, str):
        raw_body = raw_body.encode("latin-1")
    try:
        return json.loads(gzip.decompress(raw_body))
    except gzip.BadGzipFile:
        return json.loads(raw_body)


def _collect_log_entries(telemetry_requests: list[dict]) -> list[dict]:
    """Flatten the ``logs`` arrays across every captured telemetry request."""
    entries: list[dict] = []
    for request in telemetry_requests:
        body = _decode_telemetry_body(request)
        for entry in body.get("logs", []):
            if isinstance(entry, dict) and isinstance(entry.get("message"), dict):
                entries.append(entry)
    return entries
