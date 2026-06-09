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

    def test_client_session_keep_alive_kwargs(self):
        config = ConnectionConfig.from_kwargs(
            client_session_keep_alive=True,
            client_session_keep_alive_heartbeat_frequency=600,
        )
        assert config.client_session_keep_alive is True
        assert config.client_session_keep_alive_heartbeat_frequency == 600

    def test_client_session_keep_alive_upper_case_alias(self):
        config = ConnectionConfig.from_kwargs(CLIENT_SESSION_KEEP_ALIVE=True)
        assert config.client_session_keep_alive is True

    def test_enable_stage_s3_privatelink_for_us_east_1_maps_with_warning(self):
        # The reference Python connector exposes this kwarg as
        # `enable_stage_s3_privatelink_for_us_east_1`. The universal driver
        # demotes it to a deprecated alias for the canonical
        # `use_s3_regional_url` name (which matches the StageInfo field).
        with pytest.warns(DeprecationWarning, match="enable_stage_s3_privatelink_for_us_east_1"):
            config = ConnectionConfig.from_kwargs(enable_stage_s3_privatelink_for_us_east_1=True)
        assert config.use_s3_regional_url is True

    def test_use_s3_regional_url_canonical_kwarg_no_warning(self):
        import warnings

        with warnings.catch_warnings():
            warnings.simplefilter("error", DeprecationWarning)
            config = ConnectionConfig.from_kwargs(use_s3_regional_url=True)
        assert config.use_s3_regional_url is True

    def test_use_s3_regional_url_legacy_does_not_override_canonical(self):
        with pytest.warns(DeprecationWarning):
            config = ConnectionConfig.from_kwargs(
                enable_stage_s3_privatelink_for_us_east_1=True,
                use_s3_regional_url=False,
            )
        assert config.use_s3_regional_url is False

    def test_proxy_kwargs(self):
        config = ConnectionConfig.from_kwargs(
            proxy_host="proxy.example.com",
            proxy_port=8080,
            proxy_user="puser",
            proxy_password="ppass",
            no_proxy="internal.example.com,*.local",
        )
        assert config.proxy_host == "proxy.example.com"
        assert config.proxy_port == 8080
        assert config.proxy_user == "puser"
        assert config.proxy_password == "ppass"
        assert config.no_proxy == "internal.example.com,*.local"

    def test_proxy_legacy_url_form_kwarg(self):
        # Legacy ODBC `PROXY` URL is its own canonical param now (NOT an alias
        # of proxy_host); URL is parsed by sf_core's ProxyConfig::from_settings.
        config = ConnectionConfig.from_kwargs(PROXY="http://user:pass@proxy.example.com:8080")
        assert config.proxy == "http://user:pass@proxy.example.com:8080"

    def test_proxy_uppercase_kwargs(self):
        # Resolution is case-insensitive; ODBC DSN strings deliver UPPERCASE keys.
        config = ConnectionConfig.from_kwargs(
            PROXY_HOST="proxy.example.com",
            PROXY_PORT=8080,
            NO_PROXY="internal.example.com",
        )
        assert config.proxy_host == "proxy.example.com"
        assert config.proxy_port == 8080
        assert config.no_proxy == "internal.example.com"

    def test_use_proxy_env_kwarg_aliases(self):
        # Legacy ODBC `PROXYWITHENV` aliases to `use_proxy_env`; default False.
        config = ConnectionConfig.from_kwargs(PROXYWITHENV=True)
        assert config.use_proxy_env is True

    def test_allow_empty_proxy_kwarg_aliases(self):
        # Legacy ODBC `ALLOWEMPTYPROXY` aliases to `allow_empty_proxy`; default True.
        config = ConnectionConfig.from_kwargs(ALLOWEMPTYPROXY=False)
        assert config.allow_empty_proxy is False

    def test_noproxy_kwarg_aliases(self):
        # Legacy ODBC `NOPROXY` (no underscore) aliases to `no_proxy`.
        config = ConnectionConfig.from_kwargs(NOPROXY="*.corp")
        assert config.no_proxy == "*.corp"


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

    def test_client_app_id_defaults_to_driver_name(self):
        """CLIENT_APP_ID should stay as the driver name regardless of the
        user-supplied ``application`` — the user's value remains in
        ``application`` (CLIENT_ENVIRONMENT.APPLICATION on the wire)."""
        config = ConnectionConfig.from_connection_args(user="u", application="MyApp")
        assert config.client_app_id == "PythonConnector"
        assert config.application == "MyApp"

    def test_client_app_id_kwarg_rejected(self):
        """End users must not be able to override CLIENT_APP_ID. Mirrors the
        old connector's treatment of ``internal_application_name``."""
        with pytest.raises(ProgrammingError, match="wrapper-internal parameter"):
            ConnectionConfig.from_connection_args(user="u", client_app_id="EvilApp")

    def test_client_app_version_kwarg_rejected(self):
        """End users must not be able to override CLIENT_APP_VERSION. Mirrors
        the old connector's treatment of ``internal_application_version``."""
        with pytest.raises(ProgrammingError, match="wrapper-internal parameter"):
            ConnectionConfig.from_connection_args(user="u", client_app_version="9.9.9")

    def test_internal_application_name_overrides_client_app_id(self):
        config = ConnectionConfig.from_connection_args(user="u", internal_application_name="SnowSQL")
        assert config.client_app_id == "SnowSQL"

    def test_internal_application_version_overrides_client_app_version(self):
        config = ConnectionConfig.from_connection_args(user="u", internal_application_version="1.2.3")
        assert config.client_app_version == "1.2.3"

    def test_client_app_version_defaults_to_driver_version(self):
        """When ``internal_application_version`` is omitted, ``client_app_version``
        falls back to the Python driver's own ``__version__`` — matching the
        old connector's wire behaviour."""
        from snowflake.connector.version import __version__

        config = ConnectionConfig.from_connection_args(user="u")
        assert config.client_app_version == __version__

    def test_internal_application_name_popped_from_extra(self):
        """internal_application_name is consumed by from_connection_args and
        must not leak into ``_extra`` (otherwise it would reach the Rust core
        as an unknown parameter)."""
        config = ConnectionConfig.from_connection_args(user="u", internal_application_name="SnowSQL")
        assert "internal_application_name" not in config._extra

    def test_internal_application_version_popped_from_extra(self):
        config = ConnectionConfig.from_connection_args(user="u", internal_application_version="1.2.3")
        assert "internal_application_version" not in config._extra

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

    def test_client_session_keep_alive_forwarded(self):
        config = ConnectionConfig(
            client_session_keep_alive=True,
            client_session_keep_alive_heartbeat_frequency=1200,
        )
        opts = config.to_options()
        assert opts["CLIENT_SESSION_KEEP_ALIVE"] is True
        assert opts["CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY"] == 1200

    def test_client_session_keep_alive_defaults(self):
        config = ConnectionConfig()
        opts = config.to_options()
        # Default is False (forwarded so the server sees the explicit choice).
        assert opts["CLIENT_SESSION_KEEP_ALIVE"] is False
        # Frequency stays None: the heartbeat scheduler computes the default
        # from master_token_validity at runtime.
        assert "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY" not in opts

    def test_proxy_fields_forwarded_with_canonical_names(self):
        config = ConnectionConfig(
            proxy_host="proxy.example.com",
            proxy_port=8080,
            proxy_user="puser",
            proxy_password="ppass",
            no_proxy="internal.example.com",
        )
        opts = config.to_options()
        # Python field names equal Rust canonical names for proxy_*; no remap.
        assert opts["proxy_host"] == "proxy.example.com"
        assert opts["proxy_port"] == 8080
        assert opts["proxy_user"] == "puser"
        assert opts["proxy_password"] == "ppass"
        assert opts["no_proxy"] == "internal.example.com"

    def test_proxy_fields_omitted_when_unset(self):
        config = ConnectionConfig(user="u")
        opts = config.to_options()
        for key in ("proxy", "proxy_host", "proxy_port", "proxy_user", "proxy_password", "no_proxy"):
            assert key not in opts

    def test_proxy_url_form_forwarded(self):
        # Legacy ODBC `proxy` URL is forwarded as its own option; sf_core
        # parses it and merges with individual fields when both are present.
        config = ConnectionConfig(proxy="http://user:pass@proxy.example.com:8080")
        opts = config.to_options()
        assert opts["proxy"] == "http://user:pass@proxy.example.com:8080"

    def test_use_proxy_env_forwarded_with_default_false(self):
        config = ConnectionConfig()
        opts = config.to_options()
        # Default is False — forwarded so sf_core sees the explicit choice.
        assert opts["use_proxy_env"] is False
        assert opts["allow_empty_proxy"] is True

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

    def test_proxy_password_redacted_but_other_proxy_fields_visible(self):
        config = ConnectionConfig(
            proxy_host="proxy.example.com",
            proxy_user="puser",
            proxy_password="ppass",
        )
        opts = config.redacted_options()
        assert opts["proxy_host"] == "proxy.example.com"
        assert opts["proxy_user"] == "puser"
        assert opts["proxy_password"] == "***"

    def test_proxy_url_redacted_because_may_contain_creds(self):
        # The legacy `proxy` URL may embed `user:pass@host:port`; the field
        # is marked sensitive in sf_core's param registry and must redact in
        # log output.
        config = ConnectionConfig(proxy="http://user:pass@proxy.example.com:8080")
        opts = config.redacted_options()
        assert opts["proxy"] == "***"


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

    def test_proxy_is_distinct_canonical_param_not_alias_of_proxy_host(self):
        # `PROXY` (legacy ODBC URL form) and `proxy_host` (legacy Python
        # hostname) have *different* value formats, so `proxy` must not be
        # an alias of `proxy_host`. They are merged later in sf_core's
        # `ProxyConfig::from_settings`.
        assert "proxy" not in ConnectionConfig._ALIAS_MAP

    def test_alias_map_contains_legacy_odbc_proxy_aliases(self):
        # ODBC connection-string conventions: PROXYWITHENV / NOPROXY /
        # ALLOWEMPTYPROXY (no underscores, uppercase).
        assert ConnectionConfig._ALIAS_MAP["proxywithenv"] == "use_proxy_env"
        assert ConnectionConfig._ALIAS_MAP["noproxy"] == "no_proxy"
        assert ConnectionConfig._ALIAS_MAP["allowemptyproxy"] == "allow_empty_proxy"

    def test_sensitive_params(self):
        assert "password" in ConnectionConfig._SENSITIVE_PARAMS
        assert "private_key" in ConnectionConfig._SENSITIVE_PARAMS
        assert "token" in ConnectionConfig._SENSITIVE_PARAMS
        # Both proxy_password (separate field) and proxy (URL may embed creds)
        # must be redacted.
        assert "proxy_password" in ConnectionConfig._SENSITIVE_PARAMS
        assert "proxy" in ConnectionConfig._SENSITIVE_PARAMS

    def test_python_only_fields(self):
        assert "numpy" in ConnectionConfig._PYTHON_ONLY
        assert "arrow_number_to_decimal" in ConnectionConfig._PYTHON_ONLY
        assert "session_parameters" in ConnectionConfig._PYTHON_ONLY
        assert "autocommit" in ConnectionConfig._PYTHON_ONLY

    def test_all_fields_superset_of_python_only(self):
        assert ConnectionConfig._PYTHON_ONLY.issubset(ConnectionConfig._all_field_names())
