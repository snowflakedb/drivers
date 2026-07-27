"""Tests for round-trip logging via CoreLogger."""

from __future__ import annotations

import inspect
import logging

from unittest.mock import patch

import pytest

from snowflake.connector._internal.logging import get_logger
from snowflake.connector._internal.logging.core_logger import CoreLogger


_SEND = "snowflake.connector._internal.logging.core_logger.sf_core_log_event"


class TestCoreLogger:
    def test_get_logger_returns_core_logger(self) -> None:
        logger = get_logger("snowflake.connector.test_module")
        assert isinstance(logger, CoreLogger)
        assert logger.name == "snowflake.connector.test_module"

    def test_send_log_event_when_core_available(self) -> None:
        logger = get_logger("snowflake.connector.test_round_trip")
        with patch(_SEND, return_value=0) as send_mock:
            logger.info("round trip %s", "payload")
        send_mock.assert_called_once()
        kwargs = send_mock.call_args.kwargs
        assert kwargs["level"] == 2  # INFO in FFI callback encoding
        assert kwargs["message"] == "round trip payload"
        assert kwargs["logger_name"] == "snowflake.connector.test_round_trip"
        assert kwargs["function"] == "test_send_log_event_when_core_available"

    def test_falls_back_to_stdlib_when_core_unavailable(self, caplog: pytest.LogCaptureFixture) -> None:
        """When the FFI reports the pipeline is down, the record is emitted on
        the stdlib logger and points at the caller — not at CoreLogger internals.
        """
        logger = get_logger("snowflake.connector.test_fallback")
        with caplog.at_level(logging.INFO, logger="snowflake.connector.test_fallback"):
            with patch(_SEND, return_value=1):
                expected_line = inspect.currentframe().f_lineno + 1
                logger.info("early import message")

        matching = [r for r in caplog.records if r.message == "early import message"]
        assert len(matching) == 1
        record = matching[0]
        assert record.filename == "test_core_logger.py"
        assert record.funcName == "test_falls_back_to_stdlib_when_core_unavailable"
        assert record.lineno == expected_line

    def test_respects_python_log_level_before_ffi(self) -> None:
        logger = get_logger("snowflake.connector.test_level_gate")
        py_logger = logging.getLogger("snowflake.connector.test_level_gate")
        py_logger.setLevel(logging.WARNING)
        try:
            with patch(_SEND, return_value=0) as send_mock:
                logger.debug("filtered out")
            send_mock.assert_not_called()
        finally:
            py_logger.setLevel(logging.NOTSET)

    def test_ffi_failure_does_not_raise(self, caplog: pytest.LogCaptureFixture) -> None:
        logger = get_logger("snowflake.connector.test_ffi_failure")
        with caplog.at_level(logging.INFO, logger="snowflake.connector.test_ffi_failure"):
            with patch(_SEND, side_effect=RuntimeError("ffi down")):
                logger.info("should not raise")
        assert any(r.message == "should not raise" for r in caplog.records)

    def test_bad_format_string_does_not_raise(self) -> None:
        """A mismatched format string / args is a caller bug and must not raise
        into user code; the record is still emitted with the raw args attached.
        """
        logger = get_logger("snowflake.connector.test_bad_format")
        with patch(_SEND, return_value=0) as send_mock:
            logger.info("no placeholder here", "unexpected", 42)
        send_mock.assert_called_once()
        assert send_mock.call_args.kwargs["message"] == "no placeholder here ('unexpected', 42)"


class TestNativeExtensionLogger:
    def test_get_native_extension_logger_wraps_core_logger(self) -> None:
        from snowflake.connector._internal.logging.native_extension_logger import (
            NativeExtensionLogger,
            get_native_extension_logger,
        )

        logger = get_native_extension_logger("snowflake.connector.nanoarrow.test")
        assert isinstance(logger, NativeExtensionLogger)
        assert isinstance(logger._logger, CoreLogger)
        assert logger._logger.name == "snowflake.connector.nanoarrow.test"

    def test_routes_through_core_with_cpp_location(self) -> None:
        from snowflake.connector._internal.logging.native_extension_logger import (
            get_native_extension_logger,
        )

        logger = get_native_extension_logger("snowflake.connector.nanoarrow.round_trip")
        with patch(_SEND, return_value=0) as send_mock:
            logger.log(
                logging.INFO,
                "native %s",
                "payload",
                path_name="arrow_reader.cpp",
                func_name="read_batch",
                line_num=128,
            )
        send_mock.assert_called_once()
        kwargs = send_mock.call_args.kwargs
        assert kwargs["level"] == 2
        assert kwargs["message"] == "native payload"
        assert kwargs["file"] == "arrow_reader.cpp"
        assert kwargs["line"] == 128
        assert kwargs["function"] == "read_batch"
        assert kwargs["logger_name"] == "snowflake.connector.nanoarrow.round_trip"

    def test_falls_back_to_stdlib_with_cpp_location(self, caplog: pytest.LogCaptureFixture) -> None:
        from snowflake.connector._internal.logging.native_extension_logger import (
            get_native_extension_logger,
        )

        logger = get_native_extension_logger("snowflake.connector.nanoarrow.fallback")
        with caplog.at_level(logging.INFO, logger="snowflake.connector.nanoarrow.fallback"):
            with patch(_SEND, return_value=1):
                logger.log(
                    logging.INFO,
                    "early native message",
                    path_name="arrow_reader.cpp",
                    func_name="init",
                    line_num=10,
                )
        matching = [r for r in caplog.records if r.message == "early native message"]
        assert len(matching) == 1
        assert matching[0].filename == "arrow_reader.cpp"
        assert matching[0].funcName == "init"
        assert matching[0].lineno == 10

    def test_respects_python_log_level_before_ffi(self) -> None:
        from snowflake.connector._internal.logging.native_extension_logger import (
            get_native_extension_logger,
        )

        logger = get_native_extension_logger("snowflake.connector.nanoarrow.level_gate")
        py_logger = logging.getLogger("snowflake.connector.nanoarrow.level_gate")
        py_logger.setLevel(logging.WARNING)
        try:
            with patch(_SEND, return_value=0) as send_mock:
                logger.log(
                    logging.DEBUG,
                    "filtered out",
                    path_name="arrow_reader.cpp",
                    func_name="debug_fn",
                    line_num=1,
                )
            send_mock.assert_not_called()
        finally:
            py_logger.setLevel(logging.NOTSET)


class TestLoggerCallbackDispatch:
    def test_wrapper_logger_name_dispatched_to_module_logger(self, caplog: pytest.LogCaptureFixture) -> None:
        from snowflake.connector._internal.api_client.c_api._init import logger_callback

        with caplog.at_level(logging.INFO):
            logger_callback(
                2,
                b"wrapper originated",
                b"cursor.py",
                10,
                b"execute",
                b"snowflake.connector.cursor._base",
            )
        matching = [r for r in caplog.records if r.message == "wrapper originated"]
        assert len(matching) == 1
        assert matching[0].name == "snowflake.connector.cursor._base"
        assert matching[0].filename == "cursor.py"
        assert matching[0].funcName == "execute"

    def test_empty_logger_name_dispatched_to_sf_core_logger(self, caplog: pytest.LogCaptureFixture) -> None:
        from snowflake.connector._internal.api_client.c_api._init import logger_callback

        with caplog.at_level(logging.DEBUG, logger="snowflake.connector._core"):
            logger_callback(
                3,
                b"core originated",
                b"http_client.rs",
                42,
                b"execute_request",
                b"",
            )
        matching = [r for r in caplog.records if r.message == "core originated"]
        assert len(matching) == 1
        assert matching[0].name == "snowflake.connector._core"

    def test_level_finer_than_debug_is_delivered_as_debug(self, caplog: pytest.LogCaptureFixture) -> None:
        """Core wire levels 3+ (including legacy TRACE=4) map to stdlib DEBUG."""
        from snowflake.connector._internal.api_client.c_api._init import logger_callback

        with caplog.at_level(logging.DEBUG, logger="snowflake.connector._core"):
            result = logger_callback(4, b"trace level event", b"detail.rs", 1, b"trace_fn", b"")
        assert result == 0
        matching = [r for r in caplog.records if r.message == "trace level event"]
        assert len(matching) == 1
        assert matching[0].levelno == logging.DEBUG
