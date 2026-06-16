"""
Unit tests for SnowflakeRestful's raw HTTP failure handling.

When ``fetch(..., raise_raw_http_failure=True)`` (or ``request()``) hits a
>= 400 response, the universal driver raises ``OperationalError`` carrying the
HTTP status on a dedicated ``http_status`` attribute. Snowflake CLI depends on
this attribute to branch on status codes (e.g. treat 404 as "does not exist")
without parsing the message string.
"""

import pytest

from snowflake.connector._internal.snowflake_restful import SnowflakeRestful
from snowflake.connector.errors import OperationalError


@pytest.mark.parametrize("status_code", [400, 401, 404, 409, 500, 503])
def test_from_http_response_sets_http_status(status_code):
    err = OperationalError.from_http_response(status_code, response_body=b"boom")

    assert isinstance(err, OperationalError)
    assert err.http_status == status_code


def test_from_http_response_includes_status_in_message():
    err = OperationalError.from_http_response(404, response_body=b"not found")

    assert "HTTP 404" in str(err)


def test_from_http_response_truncates_large_body():
    err = OperationalError.from_http_response(500, response_body=b"x" * 1000)

    # Body is truncated to 200 chars to avoid flooding the message.
    assert len(str(err)) < 400
    assert err.http_status == 500


class _FakeInfo:
    def __init__(self, user_agent):
        self.user_agent = user_agent


class _FakeConnection:
    def __init__(self, user_agent):
        self._info = _FakeInfo(user_agent)

    def _get_connection_info(self):
        return self._info


def test_get_user_agent_returns_core_value():
    rest = SnowflakeRestful(_FakeConnection("PythonConnector/5.0.0 (Linux-x86_64)"))
    assert rest.get_user_agent() == "PythonConnector/5.0.0 (Linux-x86_64)"


@pytest.mark.parametrize("empty", ["", None])
def test_get_user_agent_returns_none_when_unset(empty):
    rest = SnowflakeRestful(_FakeConnection(empty))
    assert rest.get_user_agent() is None
