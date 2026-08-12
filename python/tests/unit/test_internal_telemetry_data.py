"""Unit tests for the backward-compatible ``TelemetryData``/``TelemetryField``
surface consolidated into ``_internal/telemetry.py``.

Real callers: Snowpark (``PCTelemetryData(message=..., timestamp=...)``,
``PCTelemetryData.TRUE``/``.FALSE``) and snowflake-cli
(``TelemetryData.from_telemetry_data_dict(from_dict=..., timestamp=...)``,
``.to_dict()["message"]``).
"""

from __future__ import annotations

import warnings

import pytest

from snowflake.connector._internal.backward_compatibility import _BACKWARD_COMPAT_WARNED
from snowflake.connector._internal.telemetry import TelemetryData, TelemetryField


@pytest.fixture(autouse=True)
def _reset_backward_compat_dedup_set():
    """Snapshot and restore the process-wide backward-compat dedup set.

    Without this, whichever test runs first would permanently consume the
    once-per-process warning slot for ``TelemetryData``/``TelemetryField``,
    hiding the warning from later tests in this module. See the identical
    fixture in ``test_backward_compatibility_warnings.py``.
    """
    snapshot = set(_BACKWARD_COMPAT_WARNED)
    _BACKWARD_COMPAT_WARNED.clear()
    try:
        yield
    finally:
        _BACKWARD_COMPAT_WARNED.clear()
        _BACKWARD_COMPAT_WARNED.update(snapshot)


class TestTelemetryDataConstruction:
    def test_defaults(self):
        data = TelemetryData()
        assert data.message is None
        assert data.timestamp == 0

    def test_stores_message_and_timestamp(self):
        data = TelemetryData(message={"type": "ct"}, timestamp=1700000000123)
        assert data.message == {"type": "ct"}
        assert data.timestamp == 1700000000123

    def test_true_false_sentinels(self):
        assert TelemetryData.TRUE == "true"
        assert TelemetryData.FALSE == "false"


class TestFromTelemetryDataDict:
    def test_wraps_dict_with_timestamp(self):
        message = {TelemetryField.KEY_TYPE.value: "session_created"}
        data = TelemetryData.from_telemetry_data_dict(from_dict=message, timestamp=42)
        assert data.message == message
        assert data.timestamp == 42

    def test_ignores_optional_connection_and_oob_args(self):
        message = {"a": 1}
        data = TelemetryData.from_telemetry_data_dict(
            from_dict=message, timestamp=1, connection=None, is_oob_telemetry=False
        )
        assert data.message == message
        assert data.timestamp == 1


class TestToDict:
    def test_round_trip_shape(self):
        data = TelemetryData(message={"type": "ct", "value": 1}, timestamp=1700000000123)
        result = data.to_dict()
        assert result == {"message": {"type": "ct", "value": 1}, "timestamp": "1700000000123"}

    def test_message_extractable_by_type_key(self):
        # Mirrors snowflake-cli's extract_first_telemetry_message_of_type helper.
        data = TelemetryData.from_telemetry_data_dict(
            from_dict={TelemetryField.KEY_TYPE.value: "cmd_execution"}, timestamp=1
        )
        assert data.to_dict()["message"].get(TelemetryField.KEY_TYPE.value) == "cmd_execution"


class TestTelemetryFieldEnum:
    def test_values(self):
        assert TelemetryField.KEY_SOURCE.value == "source"
        assert TelemetryField.KEY_TYPE.value == "type"
        assert TelemetryField.KEY_SFQID.value == "query_id"

    def test_exactly_three_members(self):
        assert {member.name for member in TelemetryField} == {"KEY_SOURCE", "KEY_TYPE", "KEY_SFQID"}


class TestBackwardCompatibilityWarning:
    """``TelemetryData``/``TelemetryField`` are old-driver surface re-added for
    Snowpark/snowflake-cli; first external access must warn exactly once."""

    @pytest.mark.parametrize("class_name", ["TelemetryData", "TelemetryField"])
    def test_warns_once_on_first_external_access(self, class_name):
        import snowflake.connector._internal.telemetry as telemetry_module

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            first = getattr(telemetry_module, class_name)
            second = getattr(telemetry_module, class_name)  # deduped
            assert first is second

        bc_warnings = [
            w for w in caught if issubclass(w.category, DeprecationWarning) and class_name in str(w.message)
        ]
        assert len(bc_warnings) == 1, [str(w.message) for w in caught]

    def test_telemetry_client_access_does_not_warn(self):
        """``TelemetryClient``/``AsyncTelemetryClient`` are real, active classes —
        module ``__getattr__`` must not intercept them."""
        import snowflake.connector._internal.telemetry as telemetry_module

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            _ = telemetry_module.TelemetryClient
            _ = telemetry_module.AsyncTelemetryClient

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert bc_warnings == []
