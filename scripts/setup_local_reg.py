#!/usr/bin/env python3
"""Bootstrap a local instance for integration tests.

Only runs against ``*.reg.local`` hosts; on any other host this is a no-op so
it's safe to invoke unconditionally from conftest. Steps:

1. Generate (or reuse) an unencrypted RSA keypair at
   ``~/.snowflake-reg/admin_rsa_key.p8``.
2. Connect with user/password and register the public key via
   ``ALTER USER``, so subsequent JWT auth works.
3. Provision the test database, warehouse, and schema referenced by
   ``parameters.json`` (``CREATE ... IF NOT EXISTS``).
4. Rewrite ``parameters.json`` with ``SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS``
   so the test suite's JWT auth path finds it.

Idempotent.
"""

from __future__ import annotations

import argparse
import json
import os
import sys

from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa

from snowflake import connector


DEFAULT_KEY_DIR = Path.home() / ".snowflake-reg"
DEFAULT_KEY_PATH = DEFAULT_KEY_DIR / "admin_rsa_key.p8"


def _generate_keypair(key_path: Path) -> tuple[str, str]:
    """Return (pem_private, public_key_b64_single_line)."""
    if key_path.exists():
        with open(key_path, "rb") as f:
            private_pem_bytes = f.read()
        private_key = serialization.load_pem_private_key(private_pem_bytes, password=None)
    else:
        key_path.parent.mkdir(parents=True, exist_ok=True)
        private_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
        private_pem_bytes = private_key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )
        key_path.write_bytes(private_pem_bytes)
        key_path.chmod(0o600)

    public_pem = private_key.public_key().public_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PublicFormat.SubjectPublicKeyInfo,
    ).decode()
    # ALTER USER wants the key body without the BEGIN/END lines.
    public_key_body = "".join(
        line for line in public_pem.splitlines() if not line.startswith("-----")
    )
    return private_pem_bytes.decode(), public_key_body


def _admin_connection(params: dict):
    return connector.connect(
        account=params.get("SNOWFLAKE_TEST_ACCOUNT"),
        host=params.get("SNOWFLAKE_TEST_HOST"),
        port=int(params["SNOWFLAKE_TEST_PORT"]) if params.get("SNOWFLAKE_TEST_PORT") else None,
        protocol=params.get("SNOWFLAKE_TEST_PROTOCOL"),
        user=params.get("SNOWFLAKE_TEST_USER") or "admin",
        password=params["SNOWFLAKE_TEST_PASSWORD"],
        role=params.get("SNOWFLAKE_TEST_ROLE") or "accountadmin",
    )


def _configure_reg_instance(params: dict, public_key_body: str) -> None:
    user = params.get("SNOWFLAKE_TEST_USER") or "admin"
    database = params.get("SNOWFLAKE_TEST_DATABASE")
    schema = params.get("SNOWFLAKE_TEST_SCHEMA")
    warehouse = params.get("SNOWFLAKE_TEST_WAREHOUSE")

    with _admin_connection(params) as conn, conn.cursor() as cur:
        cur.execute(f"ALTER USER {user} SET RSA_PUBLIC_KEY = '{public_key_body}'")
        if database:
            cur.execute(f"CREATE DATABASE IF NOT EXISTS {database}")
        if database and schema:
            cur.execute(f"CREATE SCHEMA IF NOT EXISTS {database}.{schema}")
        if warehouse:
            cur.execute(f"CREATE WAREHOUSE IF NOT EXISTS {warehouse}")


def bootstrap(
    parameters_path: Path = Path("parameters.json"),
    key_path: Path = DEFAULT_KEY_PATH,
) -> bool:
    """Bootstrap the local reg instance. Returns True if any work was done."""
    with open(parameters_path) as f:
        payload = json.load(f)
    params = payload.get("testconnection", {})
    host = (params.get("SNOWFLAKE_TEST_HOST") or "").strip()
    if not host.endswith(".reg.local"):
        return False

    private_pem, public_body = _generate_keypair(key_path)
    _configure_reg_instance(params, public_body)

    params["SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS"] = private_pem.rstrip("\n").split("\n")
    params.pop("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", None)
    payload["testconnection"] = params
    with open(parameters_path, "w") as f:
        json.dump(payload, f, indent=2)
        f.write("\n")
    return True


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--parameters",
        type=Path,
        default=Path(os.environ.get("PARAMETER_PATH", "parameters.json")),
    )
    parser.add_argument("--key-path", type=Path, default=DEFAULT_KEY_PATH)
    args = parser.parse_args(argv)
    if not bootstrap(parameters_path=args.parameters, key_path=args.key_path):
        print(
            f"Skipped: {args.parameters} does not target a *.reg.local host.",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
