"""Fetch performance for mid-size result sets.

10k / 100k rows sit between percentile customer sizes (1/15/400) and the 1M
throughput suite.
"""
import pytest
from catalog import get_sql
from matrix import cases
from runner.test_types import PerfTestType

SIZES = (
    (10_000, "10k"),
    (100_000, "100k"),
)
TYPES = ("string", "number", "date", "timestamp_ntz", "15columns")

# Suffixes into VARIANTS. Empty suffix = default fetch.
SUFFIXES = {
    "python": ("_fetchall", "_fetchone", "_pandas"),
    "jdbc": ("",),
    "odbc": ("",),
    "core": ("",),
}

CASES = cases(SIZES, SUFFIXES, infix="_arrow")


@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("dtype", TYPES)
@pytest.mark.parametrize("row_count,name,fetch_mode", CASES)
def test_select_mid(perf_test, dtype, row_count, name, fetch_mode):
    perf_test(
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        test_name=f"select_{dtype}_{name}",
    )


@pytest.mark.iterations(5)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("dtype", TYPES)
@pytest.mark.parametrize("row_count,name,fetch_mode", CASES)
def test_select_mid_recorded_http(perf_test, dtype, row_count, name, fetch_mode):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        test_name=f"select_{dtype}_{name}_recorded_http",
    )
