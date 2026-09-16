"""SNOWFLAKE_CONNECTIONS env var must parse as TOML.

Snowpark ``Session.builder.create()`` with no options calls
``_get_default_connection_params()`` (snowpark-python
``tests/integ/test_session.py::test_create_session_from_default_config_file``),
which indexes ``CONFIG_MANAGER["connections"]`` by connection name. Legacy
parses ``SNOWFLAKE_CONNECTIONS`` with ``tomlkit.parse`` so that value is a
mapping; a raw string raises ``TypeError: string indices must be integers``.
"""

from __future__ import annotations

import pytest
import tomlkit

from snowflake.connector.config_manager import CONFIG_MANAGER, _get_default_connection_params
from snowflake.connector.errors import Error


def _connections_toml(name: str, **fields: str) -> str:
    doc = tomlkit.document()
    table = tomlkit.table()
    for key, value in fields.items():
        table[key] = value
    doc[name] = table
    return tomlkit.dumps(doc)


@pytest.fixture
def isolated_connections_env(monkeypatch):
    monkeypatch.delenv("SNOWFLAKE_CONNECTIONS", raising=False)
    monkeypatch.delenv("SNOWFLAKE_DEFAULT_CONNECTION_NAME", raising=False)
    original_cache = CONFIG_MANAGER.conf_file_cache
    yield
    CONFIG_MANAGER.conf_file_cache = original_cache


class TestSnowflakeConnectionsEnvVar:
    def test_get_default_connection_params_from_env(self, isolated_connections_env, monkeypatch):
        """SNOWFLAKE_CONNECTIONS + default name yields a mapping of params."""
        monkeypatch.setenv(
            "SNOWFLAKE_CONNECTIONS",
            _connections_toml(
                "default",
                account="env_acct",
                user="env_user",
                password="env_pwd",
                host="env_host.snowflakecomputing.com",
                database="env_db",
            ),
        )
        monkeypatch.setenv("SNOWFLAKE_DEFAULT_CONNECTION_NAME", "default")

        params = _get_default_connection_params()

        assert params["account"] == "env_acct"
        assert params["user"] == "env_user"
        assert params["password"] == "env_pwd"
        assert params["host"] == "env_host.snowflakecomputing.com"
        assert params["database"] == "env_db"

    def test_honors_default_connection_name_env(self, isolated_connections_env, monkeypatch):
        """SNOWFLAKE_DEFAULT_CONNECTION_NAME selects a non-default profile from the env TOML."""
        monkeypatch.setenv(
            "SNOWFLAKE_CONNECTIONS",
            _connections_toml(
                "custom_connection_for_test",
                account="custom_acct",
                user="custom_user",
                password="custom_pwd",
            ),
        )
        monkeypatch.setenv("SNOWFLAKE_DEFAULT_CONNECTION_NAME", "custom_connection_for_test")

        params = _get_default_connection_params()

        assert params["account"] == "custom_acct"
        assert params["user"] == "custom_user"
        assert params["password"] == "custom_pwd"

    def test_connections_env_is_a_mapping(self, isolated_connections_env, monkeypatch):
        """CONFIG_MANAGER['connections'] from SNOWFLAKE_CONNECTIONS is keyed by name."""
        monkeypatch.setenv(
            "SNOWFLAKE_CONNECTIONS",
            _connections_toml("default", account="env_acct", user="env_user"),
        )

        connections = CONFIG_MANAGER["connections"]

        assert "default" in connections
        assert connections["default"]["account"] == "env_acct"
        assert connections["default"]["user"] == "env_user"

    def test_missing_named_connection_raises_error(self, isolated_connections_env, monkeypatch):
        monkeypatch.setenv(
            "SNOWFLAKE_CONNECTIONS",
            _connections_toml("other", account="other_acct"),
        )
        monkeypatch.setenv("SNOWFLAKE_DEFAULT_CONNECTION_NAME", "default")

        with pytest.raises(Error, match="Default connection with name 'default' cannot be found"):
            _get_default_connection_params()
