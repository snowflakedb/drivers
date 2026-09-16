"""Connection-parameter helpers shared by the SQL API and ADBC perf drivers."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path


def sql_object_name(name: str | None) -> str | None:
    """Uppercase unquoted identifiers so they match connector / Go driver lookup.

    SQL API looks up ``role`` / ``warehouse`` / ``database`` / ``schema``
    case-sensitively, so ``testrole_universal_perf`` becomes 390189 even when
    the same parameters file works for the Python driver. Quoted names keep
    their inner case.
    """
    if name is None:
        return None
    raw = str(name).strip()
    if not raw:
        return None
    if len(raw) >= 2 and raw.startswith('"') and raw.endswith('"'):
        return raw[1:-1]
    return raw.upper()


def load_connection(params_json: str) -> dict:
    params = json.loads(params_json)
    conn = params.get("testconnection", {})
    account = conn.get("SNOWFLAKE_TEST_ACCOUNT") or conn.get("account")
    host = conn.get("SNOWFLAKE_TEST_HOST") or conn.get("host")
    if not host and account:
        host = f"{account}.snowflakecomputing.com"
    if host and host.lower().startswith(("https://", "http://")):
        host = host.split("://", 1)[1].split("/", 1)[0]
    return {
        "account": account,
        "host": host,
        "user": conn.get("SNOWFLAKE_TEST_USER") or conn.get("user"),
        "database": conn.get("SNOWFLAKE_TEST_DATABASE") or conn.get("database"),
        "schema": conn.get("SNOWFLAKE_TEST_SCHEMA") or conn.get("schema"),
        "warehouse": conn.get("SNOWFLAKE_TEST_WAREHOUSE") or conn.get("warehouse"),
        "role": conn.get("SNOWFLAKE_TEST_ROLE") or conn.get("role"),
        "pat": (
            conn.get("SNOWFLAKE_TEST_PAT")
            or conn.get("SNOWFLAKE_TEST_SQLAPI_PAT")
            or conn.get("pat")
        ),
        "private_key_file": private_key_file(conn),
        "raw": conn,
    }


def private_key_file(conn: dict) -> str | None:
    path = conn.get("SNOWFLAKE_TEST_PRIVATE_KEY_FILE")
    if path:
        if not Path(path).is_file():
            raise RuntimeError(f"Private key file '{path}' does not exist")
        return path
    contents = conn.get("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS") or []
    if not contents:
        return None
    key_path = Path(tempfile.gettempdir()) / "perf_test_private_key.p8"
    key_path.write_text("\n".join(contents) + "\n")
    return str(key_path)
