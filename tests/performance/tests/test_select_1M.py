"""Fetch performance for 1M-row result sets.

Bind-mode matrix (ODBC):
  * no suffix / existing names — SQL_C_CHAR (to_string); historical BenchDash baselines
  * `_default` suffix           — SQL_C_DEFAULT (driver-chosen C type); separate charts

BenchDash `test_name` stays `select_{name}` / `select_{name}_recorded_http` so
existing charts keep their series. Pytest node ids are parametrized
(`test_select_1M[string_1M_arrow]`).
"""
import pytest
from catalog import TYPE_KEYS, get_sql
from matrix import cases
from runner.test_types import PerfTestType

SIZES = ((1_000_000, "1M"),)

SUFFIXES = {
    "python": ("", "_fetchall", "_pandas", "_arrow_batches"),
    "jdbc": ("",),
    "odbc": ("", "_default"),
    "core": ("",),
}

CASES = cases(SIZES, SUFFIXES, infix="_arrow", types=TYPE_KEYS)

ORDERED_SUFFIXES = {
    "python": ("", "_fetchall", "_arrow_batches"),
    "jdbc": ("",),
    "odbc": ("",),
    "core": ("",),
}

ORDERED_CASES = cases(
    SIZES, ORDERED_SUFFIXES, infix="_ordered_arrow", types=("string", "number"),
)


@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", CASES)
def test_select_1M(perf_test, row_count, dtype, name, fetch_mode, bind_mode):
    perf_test(
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}",
    )


@pytest.mark.skip(reason="ORDER BY SELECT cases disabled for now")
@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", ORDERED_CASES)
def test_select_1M_ordered(perf_test, row_count, dtype, name, fetch_mode, bind_mode):
    perf_test(
        sql_command=get_sql(dtype, row_count, ordered=True),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}",
    )


@pytest.mark.iterations(5)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", CASES)
def test_select_1M_recorded_http(perf_test, row_count, dtype, name, fetch_mode, bind_mode):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}_recorded_http",
    )


@pytest.mark.skip(reason="ORDER BY SELECT cases disabled for now")
@pytest.mark.iterations(5)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", ORDERED_CASES)
def test_select_1M_ordered_recorded_http(
    perf_test, row_count, dtype, name, fetch_mode, bind_mode
):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql(dtype, row_count, ordered=True),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}_recorded_http",
    )
