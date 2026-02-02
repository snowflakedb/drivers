"""
Tests for the snowflake.connector._internal.logging module.
"""

import io
import logging

import pytest

from snowflake.connector._internal.logging import (
    CONNECTOR_LOGGER_NAME,
    SF_CORE_LOGGER_NAME,
    get_connector_logger,
    get_sf_core_logger,
    setup_logging,
)


class TestLoggerConstants:
    """Test logger name constants."""

    def test_connector_logger_name(self):
        """Test that CONNECTOR_LOGGER_NAME is correctly defined."""
        assert CONNECTOR_LOGGER_NAME == "snowflake.connector"

    def test_sf_core_logger_name(self):
        """Test that SF_CORE_LOGGER_NAME is correctly defined."""
        assert SF_CORE_LOGGER_NAME == "snowflake.connector._core"


class TestGetLoggers:
    """Test logger getter functions."""

    def test_get_connector_logger_returns_logger(self):
        """Test that get_connector_logger returns a Logger instance."""
        logger = get_connector_logger()
        assert isinstance(logger, logging.Logger)
        assert logger.name == CONNECTOR_LOGGER_NAME

    def test_get_sf_core_logger_returns_logger(self):
        """Test that get_sf_core_logger returns a Logger instance."""
        logger = get_sf_core_logger()
        assert isinstance(logger, logging.Logger)
        assert logger.name == SF_CORE_LOGGER_NAME

    def test_get_connector_logger_same_instance(self):
        """Test that get_connector_logger returns the same instance each time."""
        logger1 = get_connector_logger()
        logger2 = get_connector_logger()
        assert logger1 is logger2

    def test_get_sf_core_logger_same_instance(self):
        """Test that get_sf_core_logger returns the same instance each time."""
        logger1 = get_sf_core_logger()
        logger2 = get_sf_core_logger()
        assert logger1 is logger2


class TestLoggerConfiguration:
    """Test that loggers are configured correctly at module load."""

    def test_sf_core_logger_propagate_disabled(self):
        """Test that sf_core logger has propagation disabled."""
        logger = get_sf_core_logger()
        assert logger.propagate is False

    def test_connector_logger_propagate_disabled(self):
        """Test that connector logger has propagation disabled."""
        logger = get_connector_logger()
        assert logger.propagate is False

    def test_sf_core_logger_has_null_handler(self):
        """Test that sf_core logger has a NullHandler."""
        logger = get_sf_core_logger()
        null_handlers = [h for h in logger.handlers if isinstance(h, logging.NullHandler)]
        assert len(null_handlers) >= 1

    def test_connector_logger_has_null_handler(self):
        """Test that connector logger has a NullHandler."""
        logger = get_connector_logger()
        null_handlers = [h for h in logger.handlers if isinstance(h, logging.NullHandler)]
        assert len(null_handlers) >= 1


class TestSetupLogging:
    """Test the setup_logging function."""

    @pytest.fixture(autouse=True)
    def cleanup_handlers(self):
        """Clean up handlers added during tests."""
        connector_logger = get_connector_logger()
        sf_core_logger = get_sf_core_logger()

        # Store original handlers
        original_connector_handlers = list(connector_logger.handlers)
        original_sf_core_handlers = list(sf_core_logger.handlers)
        original_connector_level = connector_logger.level
        original_sf_core_level = sf_core_logger.level

        yield

        # Restore original state
        connector_logger.handlers = original_connector_handlers
        sf_core_logger.handlers = original_sf_core_handlers
        connector_logger.setLevel(original_connector_level)
        sf_core_logger.setLevel(original_sf_core_level)

    def test_setup_logging_sets_connector_level(self):
        """Test that setup_logging sets the connector logger level."""
        setup_logging(level=logging.DEBUG)
        logger = get_connector_logger()
        assert logger.level == logging.DEBUG

    def test_setup_logging_sets_sf_core_level(self):
        """Test that setup_logging sets the sf_core logger level."""
        setup_logging(sf_core_level=logging.WARNING)
        logger = get_sf_core_logger()
        assert logger.level == logging.WARNING

    def test_setup_logging_default_levels(self):
        """Test that setup_logging uses INFO as default level."""
        setup_logging()
        connector_logger = get_connector_logger()
        sf_core_logger = get_sf_core_logger()
        assert connector_logger.level == logging.INFO
        assert sf_core_logger.level == logging.INFO

    def test_setup_logging_adds_stream_handler_to_connector(self):
        """Test that setup_logging adds a StreamHandler to connector logger."""
        setup_logging()
        logger = get_connector_logger()
        stream_handlers = [h for h in logger.handlers if isinstance(h, logging.StreamHandler)]
        assert len(stream_handlers) >= 1

    def test_setup_logging_adds_stream_handler_to_sf_core(self):
        """Test that setup_logging adds a StreamHandler to sf_core logger."""
        setup_logging()
        logger = get_sf_core_logger()
        stream_handlers = [h for h in logger.handlers if isinstance(h, logging.StreamHandler)]
        assert len(stream_handlers) >= 1

    def test_setup_logging_custom_stream(self):
        """Test that setup_logging uses custom stream."""
        stream = io.StringIO()
        setup_logging(stream=stream)

        logger = get_connector_logger()
        logger.info("test message")

        output = stream.getvalue()
        assert "test message" in output

    def test_setup_logging_custom_format_string(self):
        """Test that setup_logging uses custom format string."""
        stream = io.StringIO()
        custom_format = "CUSTOM: %(message)s"
        setup_logging(format_string=custom_format, stream=stream)

        logger = get_connector_logger()
        logger.info("test message")

        output = stream.getvalue()
        assert "CUSTOM: test message" in output

    def test_setup_logging_default_format_string(self):
        """Test that setup_logging uses default format when none provided."""
        stream = io.StringIO()
        setup_logging(stream=stream)

        logger = get_connector_logger()
        logger.info("test message")

        output = stream.getvalue()
        # Default format includes name and levelname
        assert "snowflake.connector" in output
        assert "INFO" in output
        assert "test message" in output

    def test_setup_logging_sf_core_writes_to_stream(self):
        """Test that sf_core logger writes to the configured stream."""
        stream = io.StringIO()
        setup_logging(stream=stream, sf_core_level=logging.DEBUG)

        logger = get_sf_core_logger()
        logger.debug("sf_core test message")

        output = stream.getvalue()
        assert "sf_core test message" in output

    def test_setup_logging_respects_level_filtering(self):
        """Test that setup_logging respects level filtering."""
        stream = io.StringIO()
        setup_logging(level=logging.WARNING, stream=stream)

        logger = get_connector_logger()
        logger.debug("debug message")
        logger.info("info message")
        logger.warning("warning message")

        output = stream.getvalue()
        assert "debug message" not in output
        assert "info message" not in output
        assert "warning message" in output
