from __future__ import annotations

import logging

import pytest


CONNECTOR_LOGGER_NAME = "snowflake.connector"
SF_CORE_LOGGER_NAME = "snowflake.connector._core"


def _wrapper_records(records):
    return [
        r for r in records if r.name.startswith(CONNECTOR_LOGGER_NAME) and not r.name.startswith(SF_CORE_LOGGER_NAME)
    ]


def _core_records(records):
    return [r for r in records if r.name.startswith(SF_CORE_LOGGER_NAME)]


@pytest.fixture
def cursor_with_query_text_logging(connection_factory):
    with connection_factory(log_query_text=True) as conn:
        with conn.cursor() as cursor:
            yield cursor


@pytest.mark.skip_reference(reason="Universal driver core logging bridge")
class TestLoggingLevels:
    @pytest.fixture(autouse=True)
    def _restore_log_levels(self):
        connector = logging.getLogger(CONNECTOR_LOGGER_NAME)
        core = logging.getLogger(SF_CORE_LOGGER_NAME)
        orig = (connector.level, core.level)
        yield
        connector.setLevel(orig[0])
        core.setLevel(orig[1])

    def test_should_emit_info_logs_at_default_levels(self, cursor_with_query_text_logging, caplog):
        # Given Default logging levels
        logging.getLogger(CONNECTOR_LOGGER_NAME).setLevel(logging.INFO)
        logging.getLogger(SF_CORE_LOGGER_NAME).setLevel(logging.INFO)

        with caplog.at_level(logging.DEBUG):
            # When Query "SELECT 1 AS value" is executed
            cursor_with_query_text_logging.execute("SELECT 1 AS value")
            cursor_with_query_text_logging.fetchone()

        # Then Core logger emits an INFO log
        assert any(r.levelno == logging.INFO for r in _core_records(caplog.records))

        # And Wrapper logger emits an INFO log
        assert any(r.levelno == logging.INFO for r in _wrapper_records(caplog.records))

    def test_should_emit_core_debug_when_core_log_level_is_debug(self, cursor_with_query_text_logging, caplog):
        # Given Logging is configured with wrapper log level INFO and core log level DEBUG
        logging.getLogger(CONNECTOR_LOGGER_NAME).setLevel(logging.INFO)
        logging.getLogger(SF_CORE_LOGGER_NAME).setLevel(logging.DEBUG)

        with caplog.at_level(logging.DEBUG):
            # When Query "SELECT 1 AS value" is executed
            cursor_with_query_text_logging.execute("SELECT 1 AS value")
            cursor_with_query_text_logging.fetchone()

        # Then Core logger emits a DEBUG log
        assert any(r.levelno == logging.DEBUG for r in _core_records(caplog.records))

        # And Wrapper logger does not emit a DEBUG log but emits INFO log
        wrapper = _wrapper_records(caplog.records)
        assert not any(r.levelno == logging.DEBUG for r in wrapper)
        assert any(r.levelno == logging.INFO for r in wrapper)

    def test_should_emit_wrapper_debug_without_core_debug_when_wrapper_log_level_is_debug(self, cursor, caplog):
        # Given Logging is configured with wrapper log level DEBUG and core log level INFO
        logging.getLogger(CONNECTOR_LOGGER_NAME).setLevel(logging.DEBUG)
        logging.getLogger(SF_CORE_LOGGER_NAME).setLevel(logging.INFO)

        with caplog.at_level(logging.DEBUG):
            # When Query "SELECT 1 AS value" is executed
            cursor.execute("SELECT 1 AS value")
            cursor.fetchone()

        # Then Wrapper logger emits a DEBUG log
        assert any(r.levelno == logging.DEBUG for r in _wrapper_records(caplog.records))

        # And Core logger does not emit a DEBUG log but emits INFO log
        core = _core_records(caplog.records)
        assert not any(r.levelno == logging.DEBUG for r in core)
        assert any(r.levelno == logging.INFO for r in core)

    def test_should_emit_wrapper_and_core_debug_when_both_levels_are_debug(self, cursor, caplog):
        # Given Logging is configured with wrapper log level DEBUG and core log level DEBUG
        logging.getLogger(CONNECTOR_LOGGER_NAME).setLevel(logging.DEBUG)
        logging.getLogger(SF_CORE_LOGGER_NAME).setLevel(logging.DEBUG)

        with caplog.at_level(logging.DEBUG):
            # When Query "SELECT 1 AS value" is executed
            cursor.execute("SELECT 1 AS value")
            cursor.fetchone()

        # Then Wrapper logger emits a DEBUG log
        assert any(r.levelno == logging.DEBUG for r in _wrapper_records(caplog.records))

        # And Core logger emits a DEBUG log
        assert any(r.levelno == logging.DEBUG for r in _core_records(caplog.records))
