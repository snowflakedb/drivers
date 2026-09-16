"""SQL API auth: PAT from parameters, else KEYPAIR_JWT from the existing private key."""

from __future__ import annotations

import base64
import hashlib
import os
import time
from pathlib import Path

from client import TOKEN_TYPE_JWT, TOKEN_TYPE_PAT
from conn_params import load_connection

__all__ = ["load_connection", "resolve_token", "generate_jwt"]


def resolve_token(conn: dict) -> tuple[str, str]:
    """Return (token, X-Snowflake-Authorization-Token-Type). PAT wins if present."""
    pat = (conn.get("pat") or os.getenv("SNOWFLAKE_TEST_PAT") or "").strip()
    if pat:
        return pat, TOKEN_TYPE_PAT
    key_file = conn.get("private_key_file")
    if key_file:
        return generate_jwt(conn["account"], conn["user"], key_file), TOKEN_TYPE_JWT
    raise RuntimeError(
        "SQL API auth requires SNOWFLAKE_TEST_PAT (or SNOWFLAKE_TEST_SQLAPI_PAT) "
        "or the existing SNOWFLAKE_TEST_PRIVATE_KEY_FILE / _CONTENTS used by drivers. "
        "Do not reuse the connector password against /api/v2."
    )


def generate_jwt(account: str, user: str, private_key_file: str, lifetime_s: int = 3540) -> str:
    import jwt
    from cryptography.hazmat.primitives import serialization

    if not account or not user:
        raise RuntimeError("SQL API JWT needs account and user")
    pem = Path(private_key_file).read_bytes()
    private_key = serialization.load_pem_private_key(pem, password=None)
    public_der = private_key.public_key().public_bytes(
        encoding=serialization.Encoding.DER,
        format=serialization.PublicFormat.SubjectPublicKeyInfo,
    )
    fingerprint = "SHA256:" + base64.b64encode(hashlib.sha256(public_der).digest()).decode(
        "utf-8"
    )
    account_id = account.replace(".snowflakecomputing.com", "").upper()
    user_id = user.upper()
    now = int(time.time())
    payload = {
        "iss": f"{account_id}.{user_id}.{fingerprint}",
        "sub": f"{account_id}.{user_id}",
        "iat": now,
        "exp": now + lifetime_s,
    }
    return jwt.encode(payload, private_key, algorithm="RS256")
