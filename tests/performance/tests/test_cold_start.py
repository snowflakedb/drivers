"""Cold-start performance test over a real Snowflake connection (no WireMock).

Real-network sibling of test_cold_start_recorded_http.py. COLD_START (unlike
SELECT) has zero setup queries -- see conftest.py's _prepare_setup_queries --
so it sidesteps the ARROW-format setup query that SELECT tests need. That
makes it useful for validating a driver's harness end-to-end before its Arrow
decoder coverage is complete: nodejs_bridge currently has no decoder for the
TEXT logicalType (see column_reader.rs), which fails the ARROW setup query
every SELECT test requires. nodejs also has no WireMock/recorded_http support
yet, so it can't use the existing recorded-HTTP cold-start test either -- this
file exists specifically to unblock it via real network instead.
"""
import pytest
from runner.test_types import PerfTestType

ITERATIONS = 10


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.supported_drivers("nodejs")
def test_cold_start_select_1(perf_test):
    perf_test(
        test_type=PerfTestType.COLD_START,
        sql_command="SELECT 1",
    )
