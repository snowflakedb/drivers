"""Cold-start performance test with WireMock — measures process load → connect → SELECT 1."""
import pytest
from runner.test_types import PerfTestType

ITERATIONS = 10


# Recorded-HTTP cold-start stays Python-only. Live e2e is all drivers
# (tests/test_cold_start.py).
@pytest.mark.iterations(ITERATIONS)
@pytest.mark.supported_drivers("python")
def test_cold_start_select_1_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.COLD_START_RECORDED_HTTP,
        sql_command="SELECT 1",
    )
