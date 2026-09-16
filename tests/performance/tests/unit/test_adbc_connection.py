"""Unit tests for ADBC db_kwargs (no native driver import)."""
import json
import sys
from pathlib import Path

import pytest

_ROOT = Path(__file__).resolve().parents[2] / "drivers"
sys.path.insert(0, str(_ROOT / "_shared"))
sys.path.insert(0, str(_ROOT / "adbc" / "app"))

from connection import (  # noqa: E402
    AUTH_JWT,
    AUTH_PAT,
    build_db_kwargs,
    load_connection,
    sql_object_name,
)


def test_sql_object_name_uppercases_unquoted():
    assert sql_object_name("testrole_universal_perf") == "TESTROLE_UNIVERSAL_PERF"
    assert sql_object_name('"MixedCase"') == "MixedCase"
    assert sql_object_name(None) is None
    assert sql_object_name("  ") is None


def test_build_db_kwargs_jwt(tmp_path, monkeypatch):
    monkeypatch.delenv("SNOWFLAKE_TEST_PAT", raising=False)
    key = tmp_path / "key.p8"
    key.write_text("-----BEGIN PRIVATE KEY-----\nMIIB\n-----END PRIVATE KEY-----\n")
    conn = {
        "account": "xy12345",
        "host": "xy12345.snowflakecomputing.com",
        "user": "perf_user",
        "database": "db1",
        "schema": "public",
        "warehouse": "wh_perf",
        "role": "testrole_universal_perf",
        "pat": None,
        "private_key_file": str(key),
    }
    kwargs = build_db_kwargs(conn)
    assert kwargs["adbc.snowflake.sql.auth_type"] == AUTH_JWT
    assert kwargs["adbc.snowflake.sql.client_option.jwt_private_key"] == str(key)
    assert kwargs["username"] == "perf_user"
    assert kwargs["adbc.snowflake.sql.account"] == "xy12345"
    assert kwargs["adbc.snowflake.sql.uri.host"] == "xy12345.snowflakecomputing.com"
    assert kwargs["adbc.snowflake.sql.db"] == "DB1"
    assert kwargs["adbc.snowflake.sql.schema"] == "PUBLIC"
    assert kwargs["adbc.snowflake.sql.warehouse"] == "WH_PERF"
    assert kwargs["adbc.snowflake.sql.role"] == "TESTROLE_UNIVERSAL_PERF"
    assert "password" not in kwargs


def test_build_db_kwargs_pat_wins_over_jwt(tmp_path):
    key = tmp_path / "key.p8"
    key.write_text("k")
    conn = {
        "account": "xy12345",
        "user": "perf_user",
        "pat": " snowflake_pat_token ",
        "private_key_file": str(key),
    }
    kwargs = build_db_kwargs(conn)
    assert kwargs["adbc.snowflake.sql.auth_type"] == AUTH_PAT
    assert kwargs["password"] == "snowflake_pat_token"
    assert "jwt_private_key" not in str(kwargs)


def test_build_db_kwargs_requires_auth():
    with pytest.raises(RuntimeError, match="ADBC auth requires"):
        build_db_kwargs({"account": "acct", "user": "u", "pat": None, "private_key_file": None})


def test_load_connection_derives_host():
    params = json.dumps(
        {
            "testconnection": {
                "SNOWFLAKE_TEST_ACCOUNT": "xy12345",
                "SNOWFLAKE_TEST_USER": "perf_user",
            }
        }
    )
    conn = load_connection(params)
    assert conn["host"] == "xy12345.snowflakecomputing.com"
    assert conn["user"] == "perf_user"


def test_single_impl_includes_adbc():
    from runner.utils import is_single_impl_driver

    assert is_single_impl_driver("adbc")
    assert is_single_impl_driver("sqlapi")
    assert not is_single_impl_driver("python")
