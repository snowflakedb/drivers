"""Fetch performance for mid-size result sets.

10k / 100k rows sit between percentile customer sizes (1/15/400) and the 1M
throughput suite.
"""
import pytest
from catalog import NODEJS_UNSUPPORTED_TYPES, get_sql
from matrix import cases
from runner.test_types import PerfTestType

SIZES = (
    (10_000, "10k"),
    (100_000, "100k"),
)

TYPES = ("string", "number", "date", "timestamp_ntz", "15columns")

SUFFIXES = {
    "python": ("_fetchall", "_fetchone", "_pandas"),
    "jdbc": ("",),
    "odbc": ("",),
    "core": ("",),
}

# Separate from SUFFIXES: nodejs has no WireMock/recorded_http support, and
# this dict also feeds test_select_mid_recorded_http, so nodejs must never
# enter it. `dtype` (TYPES) is parametrized independently of `cases()`'s
# supported_drivers marking here, so the timestamp_ntz/nodejs gap (see
# NODEJS_UNSUPPORTED_TYPES) is handled with a runtime skip in the test body
# instead — there's no per-type marker to attach it to.
NODEJS_SUFFIXES = {"nodejs": ("",)}

CASES = cases(SIZES, SUFFIXES, infix="_arrow")
NODEJS_CASES = cases(SIZES, NODEJS_SUFFIXES, infix="_arrow", id_suffix="_nodejs")


@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("dtype", TYPES)
@pytest.mark.parametrize("row_count,name,fetch_mode,bind_mode", CASES + NODEJS_CASES)
def test_select_mid(perf_test, dtype, row_count, name, fetch_mode, bind_mode, driver):
    if driver == "nodejs" and dtype in NODEJS_UNSUPPORTED_TYPES:
        pytest.skip(f"nodejs_bridge has no {dtype} decoder yet")
    perf_test(
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{dtype}_{name}",
    )


@pytest.mark.iterations(5)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("dtype", TYPES)
@pytest.mark.parametrize("row_count,name,fetch_mode,bind_mode", CASES)
def test_select_mid_recorded_http(perf_test, dtype, row_count, name, fetch_mode, bind_mode):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{dtype}_{name}_recorded_http",
    )
