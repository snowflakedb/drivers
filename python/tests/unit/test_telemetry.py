"""Unit tests for the top-level backward-compatibility telemetry stub.

``snowflake.connector.telemetry`` is what external callers (Snowpark,
snowflake-cli) actually import; it is not wired to the real client in
``_common/telemetry.py``. ``TelemetryClient.try_add_log_to_batch`` still
raises ``NotImplementedError`` unconditionally, but the first call per
process from outside ``snowflake.connector.*`` must also warn callers
toward the real client.
"""

from __future__ import annotations

import warnings

import pytest

import snowflake.connector.telemetry as telemetry_module
from snowflake.connector.telemetry import TelemetryClient


@pytest.fixture(autouse=True)
def _reset_warned_flag():
    """Reset the module-level one-shot flag so tests don't interfere with each other."""
    original = telemetry_module._TRY_ADD_LOG_TO_BATCH_WARNED
    telemetry_module._TRY_ADD_LOG_TO_BATCH_WARNED = False
    yield
    telemetry_module._TRY_ADD_LOG_TO_BATCH_WARNED = original


class TestTryAddLogToBatchBackwardCompatWarning:
    def test_first_external_call_warns_once_and_still_raises(self):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            with pytest.raises(NotImplementedError):
                TelemetryClient().try_add_log_to_batch()

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert len(bc_warnings) == 1, [str(w.message) for w in caught]
        assert "_common.telemetry" in str(bc_warnings[0].message)
        # stacklevel must attribute the warning to *this* call site, not into
        # telemetry.py or backward_compatibility.py internals.
        assert bc_warnings[0].filename == __file__

    def test_second_external_call_in_same_process_does_not_warn_again(self):
        with pytest.raises(NotImplementedError):
            TelemetryClient().try_add_log_to_batch()

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            with pytest.raises(NotImplementedError):
                TelemetryClient().try_add_log_to_batch()

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert bc_warnings == []

    def test_internal_caller_is_silently_exempt(self):
        # Impersonate an internal caller by executing the call inside a module
        # whose __name__ starts with "snowflake.connector" (same idiom as
        # test_backward_compatibility_warnings.py).
        ns: dict = {"TelemetryClient": TelemetryClient, "__name__": "snowflake.connector.fake_internal"}

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            with pytest.raises(NotImplementedError):
                exec("TelemetryClient().try_add_log_to_batch()", ns)

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert bc_warnings == []
