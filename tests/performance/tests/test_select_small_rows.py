"""Fetch performance for small result sets.

Row counts approximate p50/p90/p95 of typical customer query result sizes:
1 row (p50), 15 rows (p90), 400 rows (p95).
"""
import pytest
from catalog import NODEJS_UNSUPPORTED_TYPES, TYPE_KEYS, get_sql
from matrix import cases
from runner.test_types import PerfTestType

SIZES = (
    (1, "1_row"),
    (15, "15_rows"),
    (400, "400_rows"),
)

SUFFIXES = {
    "python": ("_fetchall", "_fetchone", "_pandas"),
    "jdbc": ("",),
    "odbc": ("",),
    "core": ("",),
}

# Separate from SUFFIXES: nodejs has no WireMock/recorded_http support, and
# CASES below also feeds test_select_small_recorded_http, so nodejs must
# never enter that shared dict.
NODEJS_SUFFIXES = {"nodejs": ("",)}

CASES = cases(SIZES, SUFFIXES)
NODEJS_CASES = cases(SIZES, NODEJS_SUFFIXES, id_suffix="_nodejs")


@pytest.mark.supports_json
@pytest.mark.iterations(15)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("dtype", TYPE_KEYS)
@pytest.mark.parametrize("row_count,name,fetch_mode,bind_mode", CASES + NODEJS_CASES)
def test_select_small(perf_test, dtype, row_count, name, fetch_mode, bind_mode, driver):
    if driver == "nodejs" and dtype in NODEJS_UNSUPPORTED_TYPES:
        pytest.skip(f"nodejs_bridge has no {dtype} decoder yet")
    perf_test(
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{dtype}_{name}",
    )


@pytest.mark.iterations(10)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("dtype", TYPE_KEYS)
@pytest.mark.parametrize("row_count,name,fetch_mode,bind_mode", CASES)
def test_select_small_recorded_http(perf_test, dtype, row_count, name, fetch_mode, bind_mode):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{dtype}_{name}_recorded_http",
    )
