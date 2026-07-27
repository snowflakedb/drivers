"""
Fixtures for authentication E2E tests.

Marker contract:
    @pytest.mark.requires_browser
        Test needs the headless browser Docker container (Chromium + Playwright).
        - Outside the container (SF_TEST_HEADLESS_BROWSER not set): test is SKIPPED.
        - Inside the container with missing credentials: test FAILS.
    @pytest.mark.requires_no_mfa
        Uses parameters_aws_local.json. Skipped unless SF_TEST_NO_MFA=true.
"""

import os

import pytest

from ...config import get_test_parameters
from .auth_helpers import clean_browser_processes


def is_browser_env():
    return os.environ.get("SF_TEST_HEADLESS_BROWSER") == "true"


def is_no_mfa_env():
    return os.environ.get("SF_TEST_NO_MFA") == "true"


def pytest_runtest_setup(item):
    if item.get_closest_marker("requires_browser") and not is_browser_env():
        pytest.skip("Requires headless browser container (SF_TEST_HEADLESS_BROWSER=true)")
    if item.get_closest_marker("requires_no_mfa") and not is_no_mfa_env():
        pytest.skip("Requires parameters_aws_local.json (SF_TEST_NO_MFA=true)")


@pytest.fixture(autouse=True)
def browser_cleanup(request):
    """Kill stray Chromium between browser tests so a stale page from a prior
    test can't make the next test's CDP attach pick up the wrong context."""
    if not request.node.get_closest_marker("requires_browser"):
        yield
        return
    clean_browser_processes()
    yield
    clean_browser_processes()


def require_auth_params(*keys: str) -> dict[str, str]:
    """Fetch required auth parameters, failing once with every missing key listed.

    Only credentials belong here; connection coordinates (host/account/...) come
    from the default test parameters via ``connection_factory``.
    """
    params = get_test_parameters()
    values = {key: params.get(key) for key in keys}
    missing = [key for key, value in values.items() if not value]
    if missing:
        pytest.fail(f"Auth test credentials missing from parameters.json: {', '.join(missing)}")
    return values
