"""Cold-start performance test with WireMock — measures process load → connect → SELECT 1."""
import pytest
from runner.test_types import PerfTestType

ITERATIONS = 10


@pytest.mark.iterations(ITERATIONS)
def test_cold_start_select_1_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.COLD_START_RECORDED_HTTP,
        sql_command="SELECT 1",
    )
