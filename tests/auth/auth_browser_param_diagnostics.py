#!/usr/bin/env python3
"""Jenkins / local **diagnostics only** for the auth-browser job.

Prints which parameters.json sections define auth keys, without printing values.
This script is print-only forever: it never gates the job. Missing keys fail
in the tests that read them (they throw; they do not skip). Used so a human
can tell a missing overlay key from a failed decode.

Invoked from tests/auth/auth_browser_common.sh.
"""

import json
import os
import sys

# The auth-browser Jenkins agents run this with their system interpreter, which is
# Python 3.6, so annotations stay 3.6-compatible: no `from __future__ import
# annotations` and no PEP 585 builtin generics (`tuple[str, ...]`). Both are
# evaluated at import time there and abort the job before any diagnostics print.
from typing import Any, Dict, List, Tuple

# Keys the auth-browser suites actually read. A key absent from this tuple is
# absent from the report, so a missing overlay key looks the same as a decode
# failure. Extend here when a new flow (e.g. OAuth client-credentials external
# client id) adds a parameter to those tests; the
# `ud-auth-browser-diagnostics-keys` rule in
# .ai/review/universal-driver-auth-browser-diagnostics.yaml flags PRs that
# forget to.
KEYS: Tuple[str, ...] = (
    "SNOWFLAKE_TEST_ACCOUNT",
    "SNOWFLAKE_TEST_HOST",
    "SNOWFLAKE_TEST_ROLE",
    "SNOWFLAKE_TEST_OKTA_USER",
    "SNOWFLAKE_TEST_OKTA_PASSWORD",
    "SNOWFLAKE_TEST_OKTA_URL",
    "SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_ID",
    "SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_SECRET",
    "SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL",
    "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_USER",
    "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_PASSWORD",
    "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_MFA_SEED",
    "SNOWFLAKE_TEST_MFA_USER",
    "SNOWFLAKE_TEST_MFA_PASSWORD",
    "SNOWFLAKE_TEST_MFA_SEED",
)


def describe(value: Any) -> str:
    if isinstance(value, (str, list)):
        return "present" if value else "empty"
    return type(value).__name__


def format_diagnostics_report(sections: Dict[str, Any], keys: Tuple[str, ...] = KEYS) -> str:
    lines: List[str] = ["=== Auth-browser diagnostics: parameter sections ==="]
    for name, values in sorted(sections.items()):
        if isinstance(values, dict):
            lines.append(f"  {name}: {len(values)} keys")

    lines.append("=== Auth-browser diagnostics: key availability by section ===")
    for key in keys:
        found = [
            f"{name}={describe(values[key])}"
            for name, values in sorted(sections.items())
            if isinstance(values, dict) and key in values
        ]
        lines.append(f"  {key}: {', '.join(found) if found else 'absent'}")
    return "\n".join(lines) + "\n"


def main() -> int:
    # TODO(SNOW-3996212): skip-all while SF_TEST_HEADLESS_BROWSER is unset is a
    # green vitest; fail that in auth_browser_nodejs.sh after the run, not here.
    path = os.environ.get("PARAMETER_PATH")
    if not path:
        print("ERROR: PARAMETER_PATH is not set", file=sys.stderr)
        return 1
    with open(path) as handle:
        sections = json.load(handle)
    if not isinstance(sections, dict):
        print("ERROR: parameters file root must be a JSON object", file=sys.stderr)
        return 1
    sys.stdout.write(format_diagnostics_report(sections))
    return 0


if __name__ == "__main__":
    sys.exit(main())
