"""
Unit tests for ConnectionConfig.
"""

from unittest.mock import patch

import pytest

from snowflake.connector.connection_config import ConnectionConfig
from snowflake.connector.errors import ProgrammingError
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


class TestConnectionConfigDefaults:
    """Test default field values."""

    def test_all_fields_default_to_none(self):
        config = ConnectionConfig()
        assert config.account is None
        assert config.host is None
        assert config.user is None
        assert config.password is None
        assert config.database is None
        assert config.schema is None
        assert config.warehouse is None
        assert config.role is None
        assert config.numpy is None
        assert config.arrow_number_to_decimal is None

    def test_extra_defaults_to_empty_dict(self):
        config = ConnectionConfig()
        assert config._extra == {}


class TestFromKwargs:
    """Test ConnectionConfig.from_kwargs factory method."""

    def test_basic_fields(self):
        config = ConnectionConfig.from_kwargs(user="alice", account="acme", port=8080)
        assert config.user == "alice"
        assert config.account == "acme"
        assert config.port == 8080

    def test_case_insensitive_alias(self):
        config = ConnectionConfig.from_kwargs(SERVER="myhost.example.com")
        assert config.host == "myhost.example.com"

    def test_uid_alias(self):
        config = ConnectionConfig.from_kwargs(UID="bob")
        assert config.user == "bob"

    def test_pwd_alias(self):
        config = ConnectionConfig.from_kwargs(PWD="secret")
        assert config.password == "secret"

    def test_unknown_keys_go_to_extra(self):
        config = ConnectionConfig.from_kwargs(user="u", custom_param="value")
        assert config.user == "u"
        assert config._extra == {"custom_param": "value"}

    def test_legacy_rewrite_private_key_file_pwd(self):
        with pytest.warns(DeprecationWarning, match="private_key_file_pwd"):
            config = ConnectionConfig.from_kwargs(private_key_file_pwd="secret")
        assert config.private_key_password == "secret"

    def test_legacy_rewrite_client_request_mfa_token(self):
        with pytest.warns(DeprecationWarning, match="client_request_mfa_token"):
            config = ConnectionConfig.from_kwargs(client_request_mfa_token=True)
        assert config.client_store_temporary_credential is True

    def test_legacy_rewrite_does_not_override_canonical(self):
        with pytest.warns(DeprecationWarning):
            config = ConnectionConfig.from_kwargs(
                client_request_mfa_token=True,
                client_store_temporary_credential=False,
            )
        assert config.client_store_temporary_credential is False

    def test_client_fetch_threads_maps_to_prefetch_with_warning(self):
        with pytest.warns(DeprecationWarning, match="client_fetch_threads"):
            config = ConnectionConfig.from_kwargs(client_fetch_threads=8)
        assert config.client_prefetch_threads == 8

    def test_client_fetch_threads_does_not_override_canonical(self):
        with pytest.warns(DeprecationWarning):
            config = ConnectionConfig.from_kwargs(
                client_fetch_threads=8,
                client_prefetch_threads=2,
            )
        assert config.client_prefetch_threads == 2

    def test_client_fetch_use_mp_is_dropped_with_warning(self):
        with pytest.warns(DeprecationWarning, match="client_fetch_use_mp"):
            config = ConnectionConfig.from_kwargs(user="u", client_fetch_use_mp=True)
        assert config.user == "u"
        assert "client_fetch_use_mp" not in config._extra


class TestFromConnectionArgs:
    """Test ConnectionConfig.from_connection_args factory method."""

    def test_basic_kwargs(self):
        config = ConnectionConfig.from_connection_args(user="u", account="a")
        assert config.user == "u"
        assert config.account == "a"
        assert config.application == "PythonConnector"

    def test_connection_name_param(self):
        config = ConnectionConfig.from_connection_args(connection_name="myconn", user="u")
        assert config.connection_name == "myconn"

    def test_connections_file_path_param(self):
        config = ConnectionConfig.from_connection_args(connections_file_path="/path/to/file", user="u")
        assert config.connections_file_path == "/path/to/file"

    def test_config_and_kwargs_raises(self):
        existing = ConnectionConfig(user="u")
        with pytest.raises(ProgrammingError, match="Cannot pass both"):
            ConnectionConfig.from_connection_args(config=existing, account="a")

    def test_config_object_passthrough(self):
        existing = ConnectionConfig(user="u", account="a")
        config = ConnectionConfig.from_connection_args(config=existing)
        assert config.user == "u"
        assert config.account == "a"
        assert config.application == "PythonConnector"

    def test_application_default(self):
        config = ConnectionConfig.from_connection_args(user="u")
        assert config.application == "PythonConnector"

    def test_application_custom(self):
        config = ConnectionConfig.from_connection_args(user="u", application="MyApp")
        assert config.application == "MyApp"

    def test_application_empty_string_defaults(self):
        config = ConnectionConfig.from_connection_args(user="u", application="")
        assert config.application == "PythonConnector"

    def test_application_none_defaults(self):
        config = ConnectionConfig.from_connection_args(user="u", application=None)
        assert config.application == "PythonConnector"

    def test_application_invalid_raises(self):
        with pytest.raises(ProgrammingError, match="Invalid application name"):
            ConnectionConfig.from_connection_args(user="u", application="!invalid")

    def test_application_non_string_raises(self):
        with pytest.raises(ProgrammingError, match="Invalid application parameter"):
            ConnectionConfig.from_connection_args(user="u", application=123)

    def test_application_dotted_name_accepted(self):
        config = ConnectionConfig.from_connection_args(user="u", application="SNOWCLI.STAGE.COPY")
        assert config.application == "SNOWCLI.STAGE.COPY"

    def test_client_app_id_injected(self):
        config = ConnectionConfig.from_connection_args(user="u", application="MyApp")
        assert config._extra["client_app_id"] == "MyApp"

    def test_autocommit_true_injects_session_parameter(self):
        config = ConnectionConfig.from_connection_args(user="u", autocommit=True)
        assert config.session_parameters["AUTOCOMMIT"] == "true"

    def test_autocommit_false_injects_session_parameter(self):
        config = ConnectionConfig.from_connection_args(user="u", autocommit=False)
        assert config.session_parameters["AUTOCOMMIT"] == "false"

    def test_autocommit_none_does_not_inject(self):
        config = ConnectionConfig.from_connection_args(user="u")
        assert config.session_parameters is None

    def test_autocommit_non_bool_raises(self):
        with pytest.raises(ProgrammingError, match="Invalid autocommit parameter"):
            ConnectionConfig.from_connection_args(user="u", autocommit=1)

    def test_private_key_normalization(self):
        """Private key is normalized via normalize_private_key."""
        with patch(
            "snowflake.connector._internal.connection_config_mixin.normalize_private_key",
            return_value="normalized",
        ):
            config = ConnectionConfig.from_connection_args(user="u", private_key="raw_key")
        assert config.private_key == "normalized"


class TestToOptions:
    """Test to_options method."""

    def test_excludes_none_values(self):
        config = ConnectionConfig(user="u")
        opts = config.to_options()
        assert "account" not in opts
        assert "user" in opts

    def test_excludes_python_only(self):
        config = ConnectionConfig(user="u", numpy=True, arrow_number_to_decimal=True)
        opts = config.to_options()
        assert "numpy" not in opts
        assert "arrow_number_to_decimal" not in opts

    def test_maps_python_to_rust_name(self):
        config = ConnectionConfig(passcode_in_password=True)
        opts = config.to_options()
        assert "passcodeInPassword" in opts
        assert "passcode_in_password" not in opts

    def test_includes_extra(self):
        config = ConnectionConfig(user="u")
        config._extra = {"custom_param": "value"}
        opts = config.to_options()
        assert opts["custom_param"] == "value"


class TestRedactedOptions:
    """Test redacted_options method."""

    def test_sensitive_fields_redacted(self):
        config = ConnectionConfig(user="u", password="secret", token="tok")
        opts = config.redacted_options()
        assert opts["password"] == "***"
        assert opts["token"] == "***"
        assert opts["user"] == "u"


class TestToProtoOptions:
    """Test to_proto_options method."""

    def test_string_value(self):
        config = ConnectionConfig(user="alice")
        proto = config.to_proto_options()
        assert proto["user"].string_value == "alice"

    def test_int_value(self):
        config = ConnectionConfig(port=8080)
        proto = config.to_proto_options()
        assert proto["port"].int_value == 8080

    def test_bool_value(self):
        config = ConnectionConfig(passcode_in_password=True)
        proto = config.to_proto_options()
        assert proto["passcodeInPassword"].bool_value is True

    def test_excludes_python_only(self):
        config = ConnectionConfig(numpy=True)
        proto = config.to_proto_options()
        assert "numpy" not in proto


class TestClassVariables:
    """Test class-level metadata."""

    def test_alias_map_contains_server(self):
        assert ConnectionConfig._ALIAS_MAP["server"] == "host"

    def test_alias_map_contains_uid(self):
        assert ConnectionConfig._ALIAS_MAP["uid"] == "user"

    def test_sensitive_params(self):
        assert "password" in ConnectionConfig._SENSITIVE_PARAMS
        assert "private_key" in ConnectionConfig._SENSITIVE_PARAMS
        assert "token" in ConnectionConfig._SENSITIVE_PARAMS

    def test_python_only_fields(self):
        assert "numpy" in ConnectionConfig._PYTHON_ONLY
        assert "arrow_number_to_decimal" in ConnectionConfig._PYTHON_ONLY
        assert "session_parameters" in ConnectionConfig._PYTHON_ONLY
        assert "autocommit" in ConnectionConfig._PYTHON_ONLY

    def test_all_fields_superset_of_python_only(self):
        assert ConnectionConfig._PYTHON_ONLY.issubset(ConnectionConfig._all_field_names())
