"""Fetch performance for mid-size result sets (Python driver).

10k / 100k rows sit between percentile customer sizes (1/15/400) and the 1M
throughput suite. Datatypes match the core 1M SELECT matrix.
Modes: fetchall, fetchone, pandas. Each case runs e2e and recorded_http.

FETCH_MODE is read only by the Python driver app — Core/ODBC/JDBC ignore it.
"""
import pytest
from catalog import get_sql
from runner.test_types import PerfTestType

ITERATIONS = 10
WARMUP_ITERATIONS = 2

ROW_COUNTS = [(10_000, "10k"), (100_000, "100k")]
TYPES = ("string", "number", "date", "timestamp_ntz", "15columns")
FETCH_MODES = ["fetchall", "fetchone", "pandas"]

CASES = [
    (row_count, dtype, fetch_mode)
    for row_count, _ in ROW_COUNTS
    for dtype in TYPES
    for fetch_mode in FETCH_MODES
]
IDS = [
    f"{dtype}_{label}_arrow_{fetch_mode}"
    for _, label in ROW_COUNTS
    for dtype in TYPES
    for fetch_mode in FETCH_MODES
]


def _size_label(row_count: int) -> str:
    return "10k" if row_count == 10_000 else "100k"


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("row_count,dtype,fetch_mode", CASES, ids=IDS)
def test_select_mid(perf_test, row_count, dtype, fetch_mode):
    label = _size_label(row_count)
    perf_test(
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        test_name=f"select_{dtype}_{label}_arrow_{fetch_mode}",
    )


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("row_count,dtype,fetch_mode", CASES, ids=IDS)
def test_select_mid_recorded_http(perf_test, row_count, dtype, fetch_mode):
    label = _size_label(row_count)
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        test_name=f"select_{dtype}_{label}_arrow_{fetch_mode}_recorded_http",
    )
