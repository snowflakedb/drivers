import pytest

import snowflake.connector

from tests.config import running_in_stored_proc


@pytest.fixture(autouse=True)
def assert_version(request):
    """Fail unless the procedure imported the connector run_stored_proc.py staged.

    Outside a stored procedure there is no staged wheel to compare against, so
    these tests are skipped.
    """
    if not running_in_stored_proc():
        pytest.skip("runs only inside a stored procedure; see tests/stored_proc/run_stored_proc.py")
    expected = request.config.getoption("--expected-connector-version")
    assert expected, "--expected-connector-version is required inside a stored procedure"
    assert snowflake.connector.__version__ == expected, (
        f"Stored procedure imported connector {snowflake.connector.__version__} from "
        f"{snowflake.connector.__file__}, expected {expected}"
    )
