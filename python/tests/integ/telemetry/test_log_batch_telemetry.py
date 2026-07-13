"""End-to-end: caller-produced log telemetry reaches ``/telemetry/send``.

Drives ``connection._telemetry`` (the path Snowpark uses) against Wiremock and
asserts the caller's own ``message`` leaves the process in the ``{"logs": [...]}``
body with a string timestamp. Universal-driver only: ``send_log_batch`` is the UD
flush entrypoint (the legacy driver named it ``send_batch``).
"""

from __future__ import annotations

import pytest

from snowflake.connector.telemetry import TelemetryData
from tests.integ.telemetry._telemetry_helpers import collect_log_entries, jwt_private_key_params


@pytest.mark.skip_reference(reason="log-batch egress via send_log_batch is universal-driver only")
def test_log_batch_reaches_telemetry_endpoint(int_test_connection_factory, wiremock):
    wiremock.add_mapping("auth/login_success_jwt.json")
    wiremock.add_mapping("telemetry/telemetry_send_success.json")

    connection = int_test_connection_factory(server_url=wiremock.http_url(), **jwt_private_key_params())
    try:
        # connection._telemetry is the live client that Snowpark reaches for.
        telemetry = connection._telemetry
        telemetry.try_add_log_to_batch(
            TelemetryData(
                message={"type": "snowpark_log_batch_probe", "value": 42},
                timestamp=1700000000123,
            )
        )
        telemetry.send_log_batch()

        requests = wiremock.wait_for_requests("/telemetry/send", min_count=1, timeout=5.0)
        entries = collect_log_entries(requests)
        probes = [entry for entry in entries if entry["message"].get("type") == "snowpark_log_batch_probe"]
        assert len(probes) == 1, f"expected exactly one probe log entry; got: {entries}"

        probe = probes[0]
        assert probe["message"]["value"] == 42  # nested JSON preserved verbatim
        assert probe["timestamp"] == "1700000000123"
        assert isinstance(probe["timestamp"], str), "timestamp must be a JSON string"
    finally:
        connection.close()
