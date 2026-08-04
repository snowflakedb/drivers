"""
Verify that connector and sf_core logs propagate correctly.

Tests cover two concerns:
1. Logs from ``snowflake.connector`` and ``snowflake.connector._core`` are
   visible to pytest's ``caplog`` fixture (propagation to root must be ON).
2. ``setup_logging()`` does not produce duplicate output (propagation is
   turned OFF once a dedicated handler is attached).

This directly addresses the snowflake-sqlalchemy failures in
test_outer_lateral_join / test_lateral_join_without_condition, where
assertions on ``caplog.text`` failed because ``propagate = False`` on the
``snowflake.connector`` logger blocked logs from reaching caplog.
"""

import io
import logging

import pytest

from snowflake.connector._internal.logging import setup_logging


# ── helpers ──────────────────────────────────────────────────────────────────


def _child_logger():
    """Return a logger in the snowflake.connector subtree, like cursor._base."""
    return logging.getLogger("snowflake.connector.cursor._base")


# ── 1.  caplog captures connector logs (the SA-test scenario) ────────────────


class TestCaplogCapture:
    """Connector debug logs must propagate to the root logger so that
    pytest's ``caplog`` fixture can capture them."""

    def test_child_logger_captured_by_caplog(self, caplog):
        """A child logger (e.g. cursor._base) should appear in caplog.text."""
        child = _child_logger()
        with caplog.at_level(logging.DEBUG):
            child.debug("query: [SELECT 1]")
        assert "query: [SELECT 1]" in caplog.text

    def test_connector_logger_captured_by_caplog(self, caplog):
        """The snowflake.connector logger itself should appear in caplog.text."""
        logger = logging.getLogger("snowflake.connector")
        with caplog.at_level(logging.DEBUG):
            logger.debug("connector level message")
        assert "connector level message" in caplog.text

    def test_sf_core_logger_captured_by_caplog(self, caplog):
        """The sf_core logger should appear in caplog.text."""
        logger = logging.getLogger("snowflake.connector._core")
        with caplog.at_level(logging.DEBUG):
            logger.debug("sf_core message")
        assert "sf_core message" in caplog.text


# ── 2.  FFI callback → caplog (sf_core Rust bridge) ─────────────────────────


class TestSfCoreCallbackPropagation:
    """The Rust→Python FFI logger callback must produce records that reach
    caplog via propagation, just like any other Python logger."""

    def test_ffi_callback_record_captured_by_caplog(self, caplog):
        """Simulate the FFI callback path and verify caplog receives the record.

        This exercises the same code path as ``_logger_callback`` in
        ``api_client``: ``makeRecord`` + ``handle`` on the sf_core logger.
        """
        sf_core_logger = logging.getLogger("snowflake.connector._core")

        with caplog.at_level(logging.DEBUG):
            record = sf_core_logger.makeRecord(
                sf_core_logger.name,
                logging.WARNING,
                "http_client.rs",
                42,
                "Simulated Rust warning from sf_core",
                (),
                None,
                func="execute_request",
            )
            sf_core_logger.handle(record)

        assert "Simulated Rust warning from sf_core" in caplog.text
        matching = [r for r in caplog.records if "Simulated Rust warning" in r.message]
        assert len(matching) == 1
        assert matching[0].name == "snowflake.connector._core"
        assert matching[0].filename == "http_client.rs"


# ── 3.  setup_logging() must not cause duplicate output ──────────────────────


class TestNoDuplicateLogsWithSetupLogging:
    """When the user explicitly calls ``setup_logging()``, messages must
    appear exactly once in its stream -- even though propagation was on
    before the call."""

    @pytest.fixture(autouse=True)
    def _restore_loggers(self):
        """Snapshot and restore handler lists so tests are isolated."""
        conn = logging.getLogger("snowflake.connector")
        core = logging.getLogger("snowflake.connector._core")
        orig = {
            "conn_handlers": list(conn.handlers),
            "conn_level": conn.level,
            "conn_propagate": conn.propagate,
            "core_handlers": list(core.handlers),
            "core_level": core.level,
            "core_propagate": core.propagate,
        }
        yield
        conn.handlers = orig["conn_handlers"]
        conn.setLevel(orig["conn_level"])
        conn.propagate = orig["conn_propagate"]
        core.handlers = orig["core_handlers"]
        core.setLevel(orig["core_level"])
        core.propagate = orig["core_propagate"]

    def test_setup_logging_no_duplicates(self):
        """After setup_logging(), each message appears exactly once."""
        stream = io.StringIO()
        setup_logging(level=logging.DEBUG, stream=stream)

        child = _child_logger()
        child.debug("unique-token-12345")

        lines = [line for line in stream.getvalue().splitlines() if "unique-token-12345" in line]
        assert len(lines) == 1, f"Expected 1 occurrence, got {len(lines)}: {lines}"

    def test_setup_logging_with_root_handler_no_duplicates(self):
        """Even if the root logger also has a handler, setup_logging()'s
        stream should still only get each message once."""

        # Simulate a root-level handler (e.g. basicConfig or a framework).
        root_stream = io.StringIO()
        root_handler = logging.StreamHandler(root_stream)
        root_handler.setLevel(logging.DEBUG)
        root_logger = logging.getLogger()
        root_logger.addHandler(root_handler)
        old_root_level = root_logger.level
        root_logger.setLevel(logging.DEBUG)

        try:
            our_stream = io.StringIO()
            setup_logging(level=logging.DEBUG, stream=our_stream)

            child = _child_logger()
            child.debug("dedup-token-99999")

            our_lines = [line for line in our_stream.getvalue().splitlines() if "dedup-token-99999" in line]
            assert len(our_lines) == 1, f"setup_logging stream got {len(our_lines)} copies: {our_lines}"
        finally:
            root_logger.removeHandler(root_handler)
            root_logger.setLevel(old_root_level)

    def test_setup_logging_ffi_callback_no_duplicates(self):
        """After setup_logging(), sf_core FFI callback records appear
        exactly once in the setup_logging stream."""
        stream = io.StringIO()
        setup_logging(level=logging.DEBUG, sf_core_level=logging.DEBUG, stream=stream)

        sf_core_logger = logging.getLogger("snowflake.connector._core")
        record = sf_core_logger.makeRecord(
            sf_core_logger.name,
            logging.WARNING,
            "retry.rs",
            100,
            "ffi-dedup-token-77777",
            (),
            None,
        )
        sf_core_logger.handle(record)

        lines = [line for line in stream.getvalue().splitlines() if "ffi-dedup-token-77777" in line]
        assert len(lines) == 1, f"Expected 1 occurrence, got {len(lines)}: {lines}"
