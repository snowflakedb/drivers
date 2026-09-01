"""Concurrent fetchmany: N worker threads on one connection.

Burst wall time is written as fetch_s, throughput_rows_s is the extra series (total rows / burst wall).
"""
import pytest
from catalog import get_sql
from runner.test_types import PerfTestType

SIZES = (
    (10_000, "10k"),
    (100_000, "100k"),
)
WORKERS = (8, 64)


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(5)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,size", SIZES)
@pytest.mark.parametrize("worker_count", WORKERS)
def test_concur_fetchmany(perf_test, row_count, size, worker_count):
    perf_test(
        test_type=PerfTestType.CONCURRENT,
        sql_command=get_sql("number", row_count),
        worker_count=worker_count,
        fetch_mode="fetchmany",
        test_name=f"concur_{size}_N{worker_count}",
    )
