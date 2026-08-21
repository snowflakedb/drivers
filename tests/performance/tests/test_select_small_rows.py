"""Fetch performance for small result sets.

Row counts approximate p50/p90/p99 of typical customer query result sizes:
1 row (p50), 15 rows (p90), 400 rows (p99).
"""
import pytest
from catalog import get_sql
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

CASES = cases(SIZES, SUFFIXES)


@pytest.mark.iterations(15)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,name,fetch_mode,bind_mode", CASES)
def test_select_string(perf_test, row_count, name, fetch_mode, bind_mode):
    perf_test(
        sql_command=get_sql("string", row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_string_{name}",
    )


@pytest.mark.iterations(10)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,name,fetch_mode,bind_mode", CASES)
def test_select_string_recorded_http(perf_test, row_count, name, fetch_mode, bind_mode):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql("string", row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_string_{name}_recorded_http",
    )
