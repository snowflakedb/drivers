import pytest
from catalog import get_sql

ITERATIONS = 3
WARMUP_ITERATIONS = 0


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_15columns_50M_arrow(perf_test):
    perf_test(sql_command=get_sql("15columns", 50_000_000))


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_string_50M_ordered_arrow(perf_test):
    perf_test(sql_command=get_sql("string", 50_000_000, ordered=True))


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_number_50M_ordered_arrow(perf_test):
    perf_test(sql_command=get_sql("number", 50_000_000, ordered=True))
