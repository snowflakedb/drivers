"""
Unit tests for Connection numpy parameter.
"""

from unittest.mock import patch

import pytest

from snowflake.connector._internal.errorcode import ER_NO_NUMPY
from snowflake.connector._internal.extras import MissingOptionalDependency
from snowflake.connector.errors import ProgrammingError
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


def make_connection(mock_db_api, **kwargs):
    from snowflake.connector.connection import Connection

    return Connection(user="test_user", account="test_account", **kwargs)


class TestConnectionNumpyParameter:
    def test_default_numpy_is_none(self, mock_db_api):
        conn = make_connection(mock_db_api)
        assert conn.config.numpy is None

    def test_numpy_true_sets_attribute(self, mock_db_api):
        with patch("snowflake.connector.connection.check_dependency"):
            conn = make_connection(mock_db_api, numpy=True)
        assert conn.config.numpy is True

    def test_numpy_true_without_numpy_installed_raises(self, mock_db_api):
        missing = MissingOptionalDependency("numpy")
        with patch("snowflake.connector.connection.np", missing):
            with pytest.raises(ProgrammingError) as exc_info:
                make_connection(mock_db_api, numpy=True)
        assert exc_info.value.errno == ER_NO_NUMPY

    def test_numpy_false_without_numpy_installed_succeeds(self, mock_db_api):
        missing = MissingOptionalDependency("numpy")
        with patch("snowflake.connector.connection.np", missing):
            conn = make_connection(mock_db_api, numpy=False)
        assert conn.config.numpy is False

    def test_numpy_does_not_leak_to_rust_core(self, mock_db_api):
        with patch("snowflake.connector.connection.check_dependency"):
            make_connection(mock_db_api, numpy=True)
        for call in mock_db_api.connection_set_option_int.call_args_list:
            request = call.args[0] if call.args else call.kwargs.get("request")
            if request is not None:
                assert getattr(request, "key", None) != "numpy"
        for call in mock_db_api.connection_set_option_string.call_args_list:
            request = call.args[0] if call.args else call.kwargs.get("request")
            if request is not None:
                assert getattr(request, "key", None) != "numpy"
