"""Shared fixtures for the unit-test suite."""

from __future__ import annotations

from unittest.mock import patch

import pytest


@pytest.fixture
def no_native_stream_ops():
    """Prevent QueryResult from touching real native memory in unit tests."""
    with (
        patch("snowflake.connector._internal.cursor.query_result.get_stream_ptr", return_value=0),
        patch("snowflake.connector._internal.cursor.query_result.release_arrow_stream"),
    ):
        yield
