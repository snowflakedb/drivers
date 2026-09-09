"""Cold-start over a real Snowflake connection (no WireMock).

Each iteration is a fresh process: connect, SELECT 1, fetchone. Python and
Node.js also time module import (`load_s`). JDBC / ODBC / Core have no import
phase, so they emit e2e / connect / select1 only.
"""
import pytest
from runner.test_types import PerfTestType

ITERATIONS = 10


@pytest.mark.iterations(ITERATIONS)
def test_cold_start_select_1(perf_test):
    perf_test(
        test_type=PerfTestType.COLD_START,
        sql_command="SELECT 1",
    )
