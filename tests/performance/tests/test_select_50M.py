"""Fetch performance for 50M-row result sets.

E2e coverage (do not expand without a deliberate BenchDash change):
  * unordered: 15columns only
  * ordered:   all types (skipped)

Recorded HTTP is a different set: unordered string/number/date only
(15columns dropped — e2e already covers it and recorded costs 35 min).
No ordered recorded.
"""
import pytest
from catalog import get_sql
from matrix import cases
from runner.test_types import PerfTestType

SIZES = ((50_000_000, "50M"),)

SUFFIXES = {
    "python": ("",),
    "jdbc": ("",),
    "odbc": ("",),
    "core": ("",),
}

# Separate from SUFFIXES: nodejs has no WireMock/recorded_http support, and
# this dict also feeds RECORDED_CASES below, so nodejs must never enter it.
NODEJS_SUFFIXES = {"nodejs": ("",)}

CASES = cases(SIZES, SUFFIXES, infix="_arrow", types=("15columns",))
NODEJS_CASES = cases(
    SIZES, NODEJS_SUFFIXES, infix="_arrow", types=("15columns",), id_suffix="_nodejs"
)

ORDERED_CASES = cases(
    SIZES, SUFFIXES, infix="_ordered_arrow", types=("string", "number")
)

RECORDED_CASES = cases(
    SIZES, SUFFIXES, infix="_arrow", types=("string", "number", "date")
)


@pytest.mark.iterations(2)
@pytest.mark.warmup_iterations(0)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", CASES + NODEJS_CASES)
def test_select_50M(perf_test, row_count, dtype, name, fetch_mode, bind_mode):
    perf_test(
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}",
    )


@pytest.mark.skip(reason="ORDER BY SELECT cases disabled for now")
@pytest.mark.iterations(2)
@pytest.mark.warmup_iterations(0)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", ORDERED_CASES)
def test_select_50M_ordered(perf_test, row_count, dtype, name, fetch_mode, bind_mode):
    perf_test(
        sql_command=get_sql(dtype, row_count, ordered=True),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}",
    )


@pytest.mark.iterations(2)
@pytest.mark.warmup_iterations(0)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", RECORDED_CASES)
def test_select_50M_recorded_http(
    perf_test, row_count, dtype, name, fetch_mode, bind_mode
):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}_recorded_http",
    )
