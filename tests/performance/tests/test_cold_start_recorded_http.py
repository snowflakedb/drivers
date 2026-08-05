"""Cold-start performance test with WireMock — measures process load → connect → SELECT 1."""
import pytest
from runner.test_types import PerfTestType

ITERATIONS = 10


# Cold-start is implemented only by the Python driver app; the ODBC/JDBC/Core apps
# have no cold-start executor and abort on the unknown TEST_TYPE. Add a driver here
# once its app gains a cold-start path.
@pytest.mark.iterations(ITERATIONS)
@pytest.mark.supported_drivers("python")
def test_cold_start_select_1_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.COLD_START_RECORDED_HTTP,
        sql_command="SELECT 1",
    )
