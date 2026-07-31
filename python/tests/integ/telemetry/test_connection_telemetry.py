import gzip
import json

from pathlib import Path

import pytest

from cryptography.hazmat.primitives.serialization import Encoding, NoEncryption, PrivateFormat, load_pem_private_key

from tests.compatibility import is_new_driver
from tests.private_key_helper import get_test_private_key_path


def test_session_init_telemetry_sent_on_connection_open(int_test_connection_factory, wiremock):
    """Verify that a session_init telemetry event is sent when a connection is opened.

    This test uses Wiremock to intercept the POST to /telemetry/send and validates
    the request path, headers, and top-level payload structure. It runs against both
    the universal and old drivers for comparison.
    """
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
def test_api_usage_telemetry_sent_on_cursor_creation(int_test_connection_factory, wiremock):
    """Verify that an api_usage telemetry event is recorded when a cursor is created.

    ``Connection.cursor`` is decorated with ``@api_telemetry``, which calls
    ``TelemetryClient.send_api_usage('Connection.cursor')``. sf_core records
    this as an ``api_call`` span event with an ``api_method`` attribute, which
    the Snowflake exporter then serializes into a log entry on
    ``/telemetry/send`` with ``message.type == 'api_call'`` and
    ``message.api_method == 'Connection.cursor'``.
    """
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

    # Event attributes (from record_api_call) + bounded wrapper_api_usage
    # span attributes. `snowflake.session.id` is stamped on the span from
    # the login mapping (tests/wiremock/mappings/auth/login_success_jwt.json
    # -> sessionId). The `code.*`, `busy_ns`, `idle_ns`, `thread.id`
    # attributes are auto-populated by `tracing::info_span!` at the FFI
    # entry-point where the per-call wrapper_api_usage span is opened.
    # The `service.*` / `process.runtime.*` attributes come from the wrapper
    # identity stamped on the span by `record_wrapper_identity_on_span`.
    from snowflake.connector.version import __version__ as CONNECTOR_VERSION

    expected_exact = {
        "type": "api_call",
        "api_method": "Connection.cursor",
        "snowflake.session.id": 12345,
        "db.system": "snowflake",
        "event_kind": "event",
        "service.name": "PythonConnector",
        "service.version": CONNECTOR_VERSION,
    }
    for key, expected in expected_exact.items():
        assert message.get(key) == expected, (
            f"api_call message[{key!r}] expected {expected!r}, got {message.get(key)!r}. Full message: {message}"
        )

    # Verify code-location and timing attributes are present and well-typed.
    # We do NOT pin `code.filepath` / `code.namespace` to a specific path —
    # it's auto-populated by tracing and changes whenever the FFI entry-point
    # moves, which is purely a refactor concern unrelated to telemetry behavior.
    assert isinstance(message.get("code.filepath"), str) and message["code.filepath"], (
        f"code.filepath must be a non-empty string, got: {message.get('code.filepath')!r}"
    )
    assert isinstance(message.get("code.namespace"), str) and message["code.namespace"], (
        f"code.namespace must be a non-empty string, got: {message.get('code.namespace')!r}"
    )

    numeric_attrs = {"code.lineno", "busy_ns", "idle_ns", "thread.id"}
    for key in numeric_attrs:
        assert isinstance(message.get(key), int), (
            f"api_call message[{key!r}] expected int, got {type(message.get(key)).__name__}: {message.get(key)!r}"
        )

    # Verify wrapper identity fields are present and non-empty strings.
    # process.runtime.name and process.runtime.version are runtime-dependent
    # (Python implementation and version) so we type-check rather than pin.
    identity_string_attrs = {"process.runtime.name", "process.runtime.version"}
    for key in identity_string_attrs:
        assert isinstance(message.get(key), str) and message[key], (
            f"api_call message[{key!r}] expected non-empty string, got: {message.get(key)!r}. Full message: {message}"
        )

    # `thread.name` is present when the span runs on a named thread (e.g. a
    # tokio worker via the async FFI path) but absent when `block_on` runs on
    # the calling Python thread (sync FFI path). Accept both.
    # `process.runtime.compiler` is present when platform.python_compiler()
    # returns a non-empty string (the common case), absent otherwise.
    expected_keys = (
        set(expected_exact.keys()) | numeric_attrs | {"code.filepath", "code.namespace", *identity_string_attrs}
    )
    if "thread.name" in message:
        assert isinstance(message["thread.name"], str) and message["thread.name"], (
            f"thread.name must be a non-empty string when present, got: {message['thread.name']!r}"
        )
        expected_keys.add("thread.name")
    if "process.runtime.compiler" in message:
        compiler = message["process.runtime.compiler"]
        assert isinstance(compiler, str) and compiler, (
            f"process.runtime.compiler must be a non-empty string when present, got: {compiler!r}"
        )
        expected_keys.add("process.runtime.compiler")
    assert set(message.keys()) == expected_keys, f"Unexpected api_call message keys: {sorted(message.keys())}"


@pytest.mark.skip_reference(reason="api_usage telemetry is universal-driver only")
def test_api_usage_telemetry_records_constructor_arguments(int_test_connection_factory, wiremock):
    """Verify that the names of arguments passed to Connection.__init__ appear in api_call telemetry.

    ``Connection.__init__`` is decorated with ``@api_telemetry``, which fires
    post-call (after __init__ returns) so that _telemetry_client is initialized
    before the telemetry is sent.  The recorded ``api_arguments`` attribute must
    contain the names of every keyword argument the caller explicitly supplied —
    names only, never values.
    """
    wiremock.add_mapping("auth/login_success_jwt.json")
    wiremock.add_mapping("telemetry/telemetry_send_success.json")

    # The factory passes a fixed set of kwargs to Connection.__init__ via
    # connector.connect(**params). All of them land in **kwargs on __init__,
    # so _passed_argument_names expands them to their individual key names.
    connection = int_test_connection_factory(server_url=wiremock.http_url(), **_jwt_private_key_params())
    connection.close()

    telemetry_requests = wiremock.wait_for_requests("/telemetry/send", min_count=1, timeout=5.0)
    assert len(telemetry_requests) >= 1, "Expected at least one POST to /telemetry/send after connection open"

    log_entries = _collect_log_entries(telemetry_requests)

    init_entries = [
        entry
        for entry in log_entries
        if entry["message"].get("type") == "api_call" and entry["message"].get("api_method") == "Connection.__init__"
    ]
    assert len(init_entries) >= 1, (
        "Expected at least one api_call telemetry log entry with "
        f"api_method='Connection.__init__'. Got entries: {log_entries}"
    )

    message = init_entries[0]["message"]

    # api_arguments is a comma-joined string of argument names; split to check membership.
    raw_args = message.get("api_arguments", "")
    assert raw_args, (
        f"api_call message['api_arguments'] must not be empty for Connection.__init__. Full message: {message}"
    )
    recorded_args = set(raw_args.split(","))

    # These kwargs are always supplied by int_test_connection_factory.
    expected_args = {"account", "user", "database", "schema", "warehouse", "role", "authenticator", "private_key_file"}
    missing = expected_args - recorded_args
    assert not missing, (
        f"Expected constructor argument names {missing!r} to be present in api_arguments "
        f"{raw_args!r}. Full message: {message}"
    )

    # Argument values must never appear in telemetry.
    assert "test_account" not in json.dumps(message), f"argument value leaked into telemetry payload: {message}"
    assert "test_user" not in json.dumps(message), f"argument value leaked into telemetry payload: {message}"


@pytest.mark.skip_reference(reason="api_usage telemetry is universal-driver only")
def test_api_usage_telemetry_records_passed_arguments(int_test_connection_factory, wiremock):
    """Verify the names of explicitly-passed arguments reach the api_call event.

    ``Connection.cursor`` is decorated with ``@api_telemetry``. When the caller
    passes an argument explicitly (here ``cursor_class``), the decorator records
    its *name* (never its value) and threads it through to sf_core, which
    attaches it as a comma-joined ``api_arguments`` attribute on the ``api_call``
    event. Calling ``cursor()`` with no arguments (see
    :func:`test_api_usage_telemetry_sent_on_cursor_creation`) omits the
    attribute entirely.
    """
    from snowflake.connector.cursor import DictCursor

    wiremock.add_mapping("auth/login_success_jwt.json")
    wiremock.add_mapping("telemetry/telemetry_send_success.json")

    connection = int_test_connection_factory(server_url=wiremock.http_url(), **_jwt_private_key_params())
    try:
        # Pass cursor_class explicitly so the decorator captures its name.
        cursor = connection.cursor(cursor_class=DictCursor)
        cursor.close()
    finally:
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

    message = cursor_entries[0]["message"]

    # The captured argument name — names only, never the value (DictCursor).
    assert message.get("api_arguments") == "cursor_class", (
        f"api_call message['api_arguments'] expected 'cursor_class', got {message.get('api_arguments')!r}. "
        f"Full message: {message}"
    )
    # The class object passed as the value must never appear in telemetry.
    assert "DictCursor" not in json.dumps(message), f"argument value leaked into telemetry payload: {message}"


@pytest.mark.skip_reference(reason="wrapper_error telemetry is universal-driver only")
def test_wrapper_error_telemetry_sent_on_execute_failure(int_test_connection_factory, wiremock):
    """Verify that a wrapper_error telemetry event is recorded when a decorated public
    method raises.

    ``SnowflakeCursor.execute`` is wrapped by ``ErrorHandlerMixin`` (see errorhandler.py),
    which on failure calls ``TelemetryClient.send_wrapper_error(type(exc).__name__,
    'SnowflakeCursor.execute')``. sf_core records this as an ``exception`` span event with
    ``exception.type`` / ``exception.source`` attributes, which the Snowflake exporter then
    serializes into a log entry on ``/telemetry/send`` with ``message.type == 'exception'``.
    """
    wiremock.add_mapping("auth/login_success_jwt.json")
    wiremock.add_mapping("telemetry/telemetry_send_success.json")
    wiremock.add_mapping("session/query_500_always.json")

    connection = int_test_connection_factory(server_url=wiremock.http_url(), **_jwt_private_key_params())
    try:
        with pytest.raises(Exception) as excinfo:
            with connection.cursor() as cursor:
                cursor.execute("SELECT 1")
    finally:
        # Closing the connection flushes pending telemetry spans via
        # SimpleSpanProcessor, so it must happen before we poll wiremock.
        connection.close()

    telemetry_requests = wiremock.wait_for_requests("/telemetry/send", min_count=1, timeout=5.0)
    assert len(telemetry_requests) >= 1, "Expected at least one POST to /telemetry/send after the failed execute"

    log_entries = _collect_log_entries(telemetry_requests)

    exception_entries = [
        entry
        for entry in log_entries
        if entry["message"].get("type") == "exception"
        and entry["message"].get("exception.source") == "SnowflakeCursor.execute"
    ]
    assert len(exception_entries) == 1, (
        "Expected exactly one exception telemetry log entry with "
        f"exception.source='SnowflakeCursor.execute'. Got entries: {log_entries}"
    )

    message = exception_entries[0]["message"]
    expected_type = type(excinfo.value).__name__
    assert message.get("exception.type") == expected_type, (
        f"exception.type expected {expected_type!r}, got {message.get('exception.type')!r}. Full message: {message}"
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
