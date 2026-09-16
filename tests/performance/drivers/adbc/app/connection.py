"""ADBC Snowflake connection: JWT key-pair (default) or PAT. No native import here."""

from __future__ import annotations

import os

from conn_params import load_connection, sql_object_name

AUTH_JWT = "auth_jwt"
AUTH_PAT = "auth_pat"

__all__ = [
    "AUTH_JWT",
    "AUTH_PAT",
    "load_connection",
    "sql_object_name",
    "build_db_kwargs",
    "connect",
]


def build_db_kwargs(conn: dict) -> dict[str, str]:
    """Build ADBC Snowflake db_kwargs. Unit-testable without the native driver."""
    account = conn.get("account")
    user = conn.get("user")
    if not account or not user:
        raise RuntimeError("ADBC needs account and user")

    kwargs: dict[str, str] = {
        "username": user,
        "adbc.snowflake.sql.account": account,
        "adbc.snowflake.sql.client_option.disable_telemetry": "true",
    }
    host = conn.get("host")
    if host:
        kwargs["adbc.snowflake.sql.uri.host"] = host
    for option, key in (
        ("adbc.snowflake.sql.db", "database"),
        ("adbc.snowflake.sql.schema", "schema"),
        ("adbc.snowflake.sql.warehouse", "warehouse"),
        ("adbc.snowflake.sql.role", "role"),
    ):
        value = sql_object_name(conn.get(key))
        if value:
            kwargs[option] = value

    pat = (conn.get("pat") or os.getenv("SNOWFLAKE_TEST_PAT") or "").strip()
    key_file = conn.get("private_key_file")
    if pat:
        kwargs["adbc.snowflake.sql.auth_type"] = AUTH_PAT
        kwargs["password"] = pat
    elif key_file:
        kwargs["adbc.snowflake.sql.auth_type"] = AUTH_JWT
        kwargs["adbc.snowflake.sql.client_option.jwt_private_key"] = key_file
    else:
        raise RuntimeError(
            "ADBC auth requires SNOWFLAKE_TEST_PAT (or SNOWFLAKE_TEST_SQLAPI_PAT) "
            "or the existing SNOWFLAKE_TEST_PRIVATE_KEY_FILE / _CONTENTS used by drivers."
        )
    return kwargs


def connect(params_json: str):
    """Open an ADBC Snowflake DBAPI connection. Imports the native driver here."""
    import adbc_driver_snowflake.dbapi as snowflake

    conn = load_connection(params_json)
    db_kwargs = build_db_kwargs(conn)
    print(f"ADBC auth: {db_kwargs.get('adbc.snowflake.sql.auth_type')}")
    print(f"ADBC host: {conn.get('host')}")
    return snowflake.connect(db_kwargs=db_kwargs)
