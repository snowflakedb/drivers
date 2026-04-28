#!/usr/bin/env python3
"""Snowflake test resource cleanup using parameters.json.

**PAT cleanup (per-build)** — remove orphaned programmatic access tokens for
one CI build (when in-process teardown does not run, e.g. aborted job):

    python3 scripts/cleanup_env.py \\
        --parameter-path ./parameters.json --build-tag JNK_988

**Stale schema cleanup** — drop non-system schemas in the session database that
are older than ``--age-days`` and owned by the test login or ``SNOWFLAKE_TEST_ROLE``
(excludes ``INFORMATION_SCHEMA``, ``PUBLIC``, and ``SNOWFLAKE_TEST_SCHEMA`` when set):

    python3 scripts/cleanup_env.py \\
        --cleanup-stale-schemas --parameter-path ./parameters.json --age-days 2

**Stale PAT cleanup** — remove ``UD_*``-prefixed programmatic access tokens for
the test user that are older than ``--age-days`` (defense-in-depth for builds
whose per-build cleanup did not run):

    python3 scripts/cleanup_env.py \\
        --cleanup-stale-pats --parameter-path ./parameters.json --age-days 2

The two stale modes can be combined in a single invocation; ``--age-days`` and
``--dry-run`` apply to both.
"""

import argparse
import json
import logging
import os
import re
import sys
from datetime import datetime, timedelta, timezone

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
log = logging.getLogger("pat-cleanup")


def _snowflake_quote_ident(name: str) -> str:
    return '"' + name.replace('"', '""') + '"'


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


def _show_user_pats(cur, user):
    """Run ``SHOW USER PROGRAMMATIC ACCESS TOKENS FOR USER`` and return rows
    as a list of dicts keyed by lowercased column name."""
    cur.execute(f"SHOW USER PROGRAMMATIC ACCESS TOKENS FOR USER {user}")
    rows = cur.fetchall()
    col_names = [desc[0].lower() for desc in cur.description]
    return [dict(zip(col_names, r)) for r in rows]


def _remove_user_pat(cur, user, pat_name):
    """Issue ``ALTER USER IF EXISTS {user} REMOVE PROGRAMMATIC ACCESS TOKEN``."""
    cur.execute(
        f"ALTER USER IF EXISTS {user} "
        f"REMOVE PROGRAMMATIC ACCESS TOKEN {pat_name}"
    )


def cleanup_pats(conn_kwargs, build_tag):
    import snowflake.connector

    user = conn_kwargs["user"]

    log.info("Connecting to Snowflake as %s...", user)
    with snowflake.connector.connect(**conn_kwargs) as conn:
        cur = conn.cursor()

        log.info("Listing PATs for user %s matching build tag '%s'...", user, build_tag)
        rows = _show_user_pats(cur, user)

        removed = 0
        for row in rows:
            pat_name = row["name"]
            if not _matches_build_tag(pat_name, build_tag):
                continue
            log.info("  Removing PAT: %s", pat_name)
            try:
                _remove_user_pat(cur, user, pat_name)
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


def cleanup_stale_owned_schemas(conn_kwargs, params, age_days, dry_run):
    """Drop schemas in the session database that are old and owned by test user or role."""
    import snowflake.connector

    user = conn_kwargs.get("user")
    role = conn_kwargs.get("role")
    owner_parts = ["UPPER(TRIM(SCHEMA_OWNER)) = UPPER(TRIM(%s))"]
    owner_binds: list = [user]
    if role:
        owner_parts.append("UPPER(TRIM(SCHEMA_OWNER)) = UPPER(TRIM(%s))")
        owner_binds.append(role)
    owner_sql = "(" + " OR ".join(owner_parts) + ")"

    preserve = params.get("SNOWFLAKE_TEST_SCHEMA")
    preserve_sql = ""
    preserve_bind: tuple = ()
    if preserve and str(preserve).strip():
        preserve_sql = " AND UPPER(TRIM(SCHEMA_NAME)) <> UPPER(TRIM(%s))"
        preserve_bind = (str(preserve).strip(),)

    list_sql = (
        "SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA WHERE "
        + owner_sql
        + " AND CREATED < DATEADD(day, -%s, CURRENT_TIMESTAMP())"
        + " AND UPPER(TRIM(SCHEMA_NAME)) <> 'INFORMATION_SCHEMA'"
        + " AND UPPER(TRIM(SCHEMA_NAME)) <> 'PUBLIC'"
        + preserve_sql
        + " ORDER BY CREATED"
    )
    list_binds = tuple(owner_binds) + (age_days,) + preserve_bind

    log.info(
        "Listing schemas older than %s day(s) owned by user=%r role=%r "
        "(excluding INFORMATION_SCHEMA, PUBLIC%s)",
        age_days,
        user,
        role,
        f", default schema {preserve!r}" if preserve_bind else "",
    )

    with snowflake.connector.connect(**conn_kwargs) as conn:
        with conn.cursor() as cur:
            cur.execute(list_sql, list_binds)
            rows = [r[0] for r in cur.fetchall()]

        if not rows:
            log.info("No stale owned schemas found")
            return

        log.info("Found %d candidate schema(s)", len(rows))

        dropped = 0
        for name in rows:
            if dry_run:
                log.info("Would drop: %s", name)
                continue
            with conn.cursor() as cur:
                try:
                    qn = _snowflake_quote_ident(name)
                    cur.execute(f"DROP SCHEMA IF EXISTS {qn} CASCADE")
                    dropped += 1
                    log.info("Dropped %s", name)
                except Exception as e:
                    log.warning("Failed to drop %s: %s", name, e)

    if dry_run:
        log.info("Dry run complete (%d schema(s) in list)", len(rows))
    else:
        log.info("Schema cleanup done: dropped %d schema(s)", dropped)


def cleanup_stale_pats(conn_kwargs, age_days, dry_run):
    """Remove ``UD_*`` PATs for the test user older than ``age_days``.

    Defense-in-depth complement to per-build PAT cleanup: catches PATs left
    behind by builds whose teardown did not run (timeouts, aborts, killed
    workers). Only PATs whose name starts with ``UD_`` are eligible — same
    safety convention as :func:`_matches_build_tag`.
    """
    import snowflake.connector

    user = conn_kwargs["user"]
    cutoff = datetime.now(timezone.utc) - timedelta(days=age_days)

    log.info(
        "Listing UD_* PATs for user %s older than %s day(s) (cutoff: %s)...",
        user,
        age_days,
        cutoff.isoformat(),
    )

    with snowflake.connector.connect(**conn_kwargs) as conn:
        cur = conn.cursor()
        rows = _show_user_pats(cur, user)

        candidates = []
        skipped_non_ud = 0
        for row in rows:
            pat_name = row.get("name")
            if not pat_name or not pat_name.upper().startswith("UD_"):
                skipped_non_ud += 1
                continue
            created = row.get("created_on")
            if created is None:
                log.warning("  Skipping %s: no created_on value", pat_name)
                continue
            # Snowflake usually returns tz-aware datetimes; normalize defensively.
            if isinstance(created, datetime) and created.tzinfo is None:
                created = created.replace(tzinfo=timezone.utc)
            if created >= cutoff:
                continue
            candidates.append((pat_name, created))

        if skipped_non_ud:
            log.info(
                "Ignored %d non-UD_* PAT(s) (only UD_* tokens are managed here)",
                skipped_non_ud,
            )

        if not candidates:
            log.info("No stale UD_* PATs found")
            return

        log.info("Found %d stale PAT(s)", len(candidates))

        removed = 0
        for pat_name, created in candidates:
            if dry_run:
                log.info("Would remove: %s (created %s)", pat_name, created)
                continue
            try:
                _remove_user_pat(cur, user, pat_name)
                removed += 1
                log.info("Removed %s (created %s)", pat_name, created)
            except Exception as e:
                log.warning("Failed to remove %s: %s", pat_name, e)

    if dry_run:
        log.info("Dry run complete (%d PAT(s) in list)", len(candidates))
    else:
        log.info("Stale PAT cleanup done: removed %d PAT(s)", removed)


def main():
    parser = argparse.ArgumentParser(
        description="Clean up Snowflake test PATs and/or stale owned schemas (parameters.json)"
    )
    parser.add_argument(
        "--parameter-path",
        default=os.environ.get("PARAMETER_PATH", "parameters.json"),
        help="Path to decoded parameters.json (default: $PARAMETER_PATH or parameters.json)",
    )
    parser.add_argument(
        "--cleanup-stale-schemas",
        action="store_true",
        help=(
            "Drop schemas in the session database older than --age-days that are "
            "owned by the test user or role (not per-build PAT cleanup)"
        ),
    )
    parser.add_argument(
        "--cleanup-stale-pats",
        action="store_true",
        help=(
            "Remove UD_* programmatic access tokens for the test user older "
            "than --age-days (not per-build PAT cleanup). Can be combined "
            "with --cleanup-stale-schemas."
        ),
    )
    parser.add_argument(
        "--age-days",
        type=int,
        default=None,
        metavar="N",
        help=(
            "With --cleanup-stale-schemas / --cleanup-stale-pats: act on items "
            "older than N days (default: 2)"
        ),
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help=(
            "With --cleanup-stale-schemas / --cleanup-stale-pats: list items "
            "only, do not drop or remove"
        ),
    )
    parser.add_argument(
        "--build-tag",
        help=(
            "For per-build PAT cleanup: CI build tag, e.g. JNK_988 or BK_1234. "
            "Auto-detected from environment if not provided. Not compatible "
            "with --cleanup-stale-* flags."
        ),
    )
    args = parser.parse_args()

    stale_mode = args.cleanup_stale_schemas or args.cleanup_stale_pats

    if args.dry_run and not stale_mode:
        parser.error(
            "--dry-run is only valid with --cleanup-stale-schemas "
            "and/or --cleanup-stale-pats"
        )

    if args.age_days is not None and not stale_mode:
        parser.error(
            "--age-days requires --cleanup-stale-schemas "
            "and/or --cleanup-stale-pats"
        )

    if args.build_tag and stale_mode:
        parser.error("--build-tag is not compatible with --cleanup-stale-* flags")

    if not os.path.exists(args.parameter_path):
        log.error("Parameters file not found: %s", args.parameter_path)
        sys.exit(1)

    ensure_snowflake_connector()

    params = load_parameters(args.parameter_path)
    conn_kwargs = build_connection_kwargs(params)

    if stale_mode:
        age_days = 2 if args.age_days is None else args.age_days
        if age_days < 0:
            log.error("--age-days must be non-negative")
            sys.exit(1)
        if args.cleanup_stale_schemas:
            cleanup_stale_owned_schemas(conn_kwargs, params, age_days, args.dry_run)
        if args.cleanup_stale_pats:
            cleanup_stale_pats(conn_kwargs, age_days, args.dry_run)
        return

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
    cleanup_pats(conn_kwargs, build_tag)


if __name__ == "__main__":
    main()
