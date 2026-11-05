#!/usr/bin/env python3
"""Binds an auth-browser-tests stage to the MFA pool user that Jenkins'
Lockable Resources plugin assigned it, for both this stage's OAuth-flow test
and its own native password+MFA test — so the stage's whole run uses exactly
one exclusively-locked identity for its duration.

With two comma-separated resource names (a stage that locks two pool users), the
first user takes the password+MFA keys and the second takes the OAuth keys, so
the two flows never draw TOTP codes from one seed.

Only manipulates key *names* already present in the decoded parameters file;
never needs to know or print the actual secret values.

Usage: bind_pool_user.py <resource-name[,resource-name]> <python|odbc|jdbc|nodejs>
Example: bind_pool_user.py mfa-pool-03 python
Example: bind_pool_user.py mfa-pool-03,mfa-pool-04 nodejs
"""
import json
import re
import sys

PARAMS_FILE = "parameters_preprod.json"
POOL_SECTION = "testconnection-mfa-pool"

OAUTH_KEYS = (
    "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_USER",
    "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_PASSWORD",
    "SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_MFA_SEED",
)
MFA_KEYS = (
    "SNOWFLAKE_TEST_MFA_USER",
    "SNOWFLAKE_TEST_MFA_PASSWORD",
    "SNOWFLAKE_TEST_MFA_SEED",
)


def pool_credentials(data, resource_name):
    match = re.search(r"(\d+)$", resource_name)
    if not match:
        raise SystemExit(f"cannot parse a pool index out of resource name: {resource_name!r}")
    pool_index = match.group(1).zfill(2)

    pool_user = data.get(POOL_SECTION, {}).get(pool_index)
    if not pool_user:
        raise SystemExit(
            f"no entry for pool user '{pool_index}' in '{POOL_SECTION}' — "
            f"was the secrets file updated with all 6 pool users?"
        )

    user = pool_user.get("SNOWFLAKE_TEST_MFA_USER")
    password = pool_user.get("SNOWFLAKE_TEST_MFA_PASSWORD")
    seed = pool_user.get("SNOWFLAKE_TEST_MFA_SEED")
    assert user and password and seed, f"pool user '{pool_index}' is missing credentials"
    return pool_index, user, password, seed


def bind(section, keys, credentials):
    _, user, password, seed = credentials
    for key in keys:
        if key.endswith("_USER"):
            value = user
        elif key.endswith("_PASSWORD"):
            value = password
        else:
            value = seed
        if key in section:
            section[key] = value


def main():
    if len(sys.argv) != 3:
        raise SystemExit(f"usage: {sys.argv[0]} <resource-name[,resource-name]> <python|odbc|jdbc|nodejs>")
    resource_names = [name.strip() for name in sys.argv[1].split(",")]
    language = sys.argv[2]

    with open(PARAMS_FILE) as f:
        data = json.load(f)

    mfa_credentials = pool_credentials(data, resource_names[0])
    oauth_credentials = pool_credentials(data, resource_names[-1])

    base = data.setdefault("testconnection", {})
    lang_override = data.setdefault(f"testconnection-{language}", {})

    for section in (base, lang_override):
        bind(section, MFA_KEYS, mfa_credentials)
        bind(section, OAUTH_KEYS, oauth_credentials)

    with open(PARAMS_FILE, "w") as f:
        json.dump(data, f)

    print(
        f"MFA pool: {language} stage bound to MFA pool user '{mfa_credentials[0]}' "
        f"and OAuth pool user '{oauth_credentials[0]}' (resources: {', '.join(resource_names)})"
    )


if __name__ == "__main__":
    main()
