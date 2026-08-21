import pytest
from catalog import get_sql
from runner.test_types import PerfTestType

ITERATIONS = 3
WARMUP_ITERATIONS = 0


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_string_50M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql("string", 50_000_000),
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_number_50M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql("number", 50_000_000),
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_date_50M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql("date", 50_000_000),
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_15columns_50M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql("15columns", 50_000_000),
    )
