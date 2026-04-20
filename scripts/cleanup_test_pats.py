#!/usr/bin/env python3
"""Remove orphaned PATs created by this CI build.

Safety net for when in-process teardown (Rust Drop, Python finally,
C++ destructors) doesn't execute — e.g. when a build is aborted or
killed by a timeout.

Connects to Snowflake using parameters.json and removes any
PROGRAMMATIC ACCESS TOKENs whose name matches the UD_* prefix
for the given build tag (e.g. UD_RUST_JNK_988_a3f2b1c0).

Usage:
    python3 scripts/cleanup_test_pats.py \
        --parameter-path ./parameters.json \
        --build-tag JNK_988
"""

import argparse
import json
import logging
import os
import re
import sys

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
log = logging.getLogger("pat-cleanup")


def ensure_snowflake_connector():
    try:
        import snowflake.connector  # noqa: F401

        return
    except ImportError:
        pass
    import subprocess

    log.info("snowflake-connector-python not found, installing from PyPI...")
    subprocess.check_call(
        [sys.executable, "-m", "pip", "install", "snowflake-connector-python", "-q"],
    )


def load_parameters(path):
    with open(path) as f:
        data = json.load(f)
    return data.get("testconnection", data)


def build_connection_kwargs(params):
    kwargs = {
        "account": params.get("SNOWFLAKE_TEST_ACCOUNT"),
        "user": params.get("SNOWFLAKE_TEST_USER"),
        "role": params.get("SNOWFLAKE_TEST_ROLE"),
        "database": params.get("SNOWFLAKE_TEST_DATABASE"),
        "schema": params.get("SNOWFLAKE_TEST_SCHEMA"),
        "warehouse": params.get("SNOWFLAKE_TEST_WAREHOUSE"),
    }
    if params.get("SNOWFLAKE_TEST_HOST"):
        kwargs["host"] = params["SNOWFLAKE_TEST_HOST"]
    if params.get("SNOWFLAKE_TEST_PORT"):
        kwargs["port"] = params["SNOWFLAKE_TEST_PORT"]
    if params.get("SNOWFLAKE_TEST_PROTOCOL"):
        kwargs["protocol"] = params["SNOWFLAKE_TEST_PROTOCOL"]

    pk_contents = params.get("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")
    pk_file = params.get("SNOWFLAKE_TEST_PRIVATE_KEY_FILE")
    pk_password = params.get("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")

    if pk_contents:
        if isinstance(pk_contents, list):
            pem_text = "\n".join(pk_contents)
        else:
            pem_text = pk_contents
        kwargs["private_key"] = _load_pem_private_key(pem_text, pk_password)
        kwargs["authenticator"] = "SNOWFLAKE_JWT"
    elif pk_file:
        with open(pk_file) as f:
            pem_text = f.read()
        kwargs["private_key"] = _load_pem_private_key(pem_text, pk_password)
        kwargs["authenticator"] = "SNOWFLAKE_JWT"
    elif params.get("SNOWFLAKE_TEST_PASSWORD"):
        kwargs["password"] = params["SNOWFLAKE_TEST_PASSWORD"]

    return kwargs


def _load_pem_private_key(pem_text, password=None):
    from cryptography.hazmat.primitives import serialization

    pwd_bytes = password.encode() if password else None
    private_key = serialization.load_pem_private_key(
        pem_text.encode(), password=pwd_bytes
    )
    return private_key.private_bytes(
        encoding=serialization.Encoding.DER,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption(),
    )


def _sanitize(s):
    return re.sub(r"[^a-zA-Z0-9]", "", s)


def cleanup_pats(conn_kwargs, build_tag):
    import snowflake.connector

    user = conn_kwargs["user"]

    log.info("Connecting to Snowflake as %s...", user)
    with snowflake.connector.connect(**conn_kwargs) as conn:
        cur = conn.cursor()

        log.info("Listing PATs for user %s matching build tag '%s'...", user, build_tag)
        cur.execute(f"SHOW USER PROGRAMMATIC ACCESS TOKENS FOR USER {user}")
        rows = cur.fetchall()
        col_names = [desc[0].lower() for desc in cur.description]
        name_idx = col_names.index("name")

        removed = 0
        for row in rows:
            pat_name = row[name_idx]
            if not _matches_build_tag(pat_name, build_tag):
                continue
            log.info("  Removing PAT: %s", pat_name)
            try:
                cur.execute(
                    f"ALTER USER IF EXISTS {user} "
                    f"REMOVE PROGRAMMATIC ACCESS TOKEN {pat_name}"
                )
                removed += 1
            except Exception as e:
                log.warning("  Failed to remove %s: %s", pat_name, e)

        log.info("Cleanup complete: removed %d PAT(s)", removed)
        return removed


def _matches_build_tag(pat_name, build_tag):
    """Check if a PAT name matches UD_{DRIVER}_{build_tag}_{random}.

    PAT names are uppercased by Snowflake. The build_tag is already
    sanitized (alphanumeric only) by the test code that created it.
    """
    upper = pat_name.upper()
    tag_upper = build_tag.upper()
    if not upper.startswith("UD_"):
        return False
    parts = upper.split("_", 3)
    if len(parts) < 4:
        return False
    # parts = [UD, DRIVER, ...rest]. The rest starts with the CI tag.
    remainder = upper[len("UD_" + parts[1] + "_") :]
    return remainder.startswith(tag_upper + "_")


def main():
    parser = argparse.ArgumentParser(
        description="Remove orphaned PATs created by a CI build"
    )
    parser.add_argument(
        "--parameter-path",
        default=os.environ.get("PARAMETER_PATH", "parameters.json"),
        help="Path to decoded parameters.json (default: $PARAMETER_PATH or parameters.json)",
    )
    parser.add_argument(
        "--build-tag",
        help=(
            "CI build tag to match, e.g. JNK_988 or BK_1234. "
            "Auto-detected from environment if not provided."
        ),
    )
    args = parser.parse_args()

    build_tag = args.build_tag
    if not build_tag:
        bk = os.environ.get("BUILDKITE_BUILD_NUMBER")
        jnk = os.environ.get("BUILD_NUMBER")
        gha = os.environ.get("GITHUB_RUN_NUMBER")
        if bk:
            build_tag = f"BK_{_sanitize(bk)}"
        elif jnk:
            build_tag = f"JNK_{_sanitize(jnk)}"
        elif gha:
            build_tag = f"GHA_{_sanitize(gha)}"
        else:
            log.error(
                "Cannot determine build tag. Provide --build-tag or set "
                "BUILD_NUMBER / BUILDKITE_BUILD_NUMBER / GITHUB_RUN_NUMBER."
            )
            sys.exit(1)

    log.info("Build tag: %s", build_tag)

    if not os.path.exists(args.parameter_path):
        log.error("Parameters file not found: %s", args.parameter_path)
        sys.exit(1)

    ensure_snowflake_connector()

    params = load_parameters(args.parameter_path)
    conn_kwargs = build_connection_kwargs(params)
    cleanup_pats(conn_kwargs, build_tag)


if __name__ == "__main__":
    main()
