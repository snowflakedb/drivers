"""
Fixtures for authentication E2E tests.

Marker contract:
    @pytest.mark.requires_browser
        Test needs the headless browser Docker container (Chromium + Playwright).
        - Outside the container (SF_TEST_HEADLESS_BROWSER not set): test is SKIPPED.
        - Inside the container with missing credentials: test FAILS.
"""

import os

import pytest

from ...config import get_test_parameters


def is_browser_env():
    return os.environ.get("SF_TEST_HEADLESS_BROWSER") == "true"


def pytest_runtest_setup(item):
    if item.get_closest_marker("requires_browser") and not is_browser_env():
        pytest.skip("Requires headless browser container (SF_TEST_HEADLESS_BROWSER=true)")


@pytest.fixture(scope="module")
def browser_params():
    """Load external browser test parameters. Fails if credentials are missing."""
    params = get_test_parameters()
    browser_user = params.get("SNOWFLAKE_TEST_OKTA_USER")
    okta_password = params.get("SNOWFLAKE_TEST_OKTA_PASSWORD")
    host = params.get("SNOWFLAKE_TEST_OKTA_HOST")
    account = params.get("SNOWFLAKE_TEST_OKTA_ACCOUNT")

    missing = []
    if not browser_user:
        missing.append("SNOWFLAKE_TEST_OKTA_USER")
    if not okta_password:
        missing.append("SNOWFLAKE_TEST_OKTA_PASSWORD")
    if not host:
        missing.append("SNOWFLAKE_TEST_OKTA_HOST")
    if not account:
        missing.append("SNOWFLAKE_TEST_OKTA_ACCOUNT")

    if missing:
        pytest.fail(f"Browser auth test credentials missing from parameters.json: {', '.join(missing)}")

    return {
        "host": host,
        "account": account,
        "browser_user": browser_user,
        "okta_login": browser_user,
        "okta_password": okta_password,
        "role": "PUBLIC",
        "database": params.get("SNOWFLAKE_TEST_DATABASE"),
        "schema": params.get("SNOWFLAKE_TEST_SCHEMA"),
        "warehouse": params.get("SNOWFLAKE_TEST_WAREHOUSE"),
    }
