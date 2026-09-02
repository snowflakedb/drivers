"""Concurrent SELECT bursts.

Python sync/aio: N workers on one connection. ODBC and JDBC: one connection per worker
(opened before timing; setup queries including Arrow run on each worker session).

Burst wall time is written as fetch_s, throughput_rows_s is the extra series (total rows / burst wall).
"""
import pytest
from catalog import get_sql
from runner.test_types import PerfTestType

SIZES = (
    (10_000, "10k"),
    (100_000, "100k"),
)
WORKERS = (2, 4, 8, 64)

HIGH_CONCUR = (
    (10_000, "10k", 512),
    (10_000, "10k", 1024),
)


@pytest.mark.supported_drivers("python", "odbc", "jdbc")
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


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(3)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,size,worker_count", HIGH_CONCUR)
def test_concur_fetchmany_high_workers(perf_test, row_count, size, worker_count):
    perf_test(
        test_type=PerfTestType.CONCURRENT,
        sql_command=get_sql("number", row_count),
        worker_count=worker_count,
        fetch_mode="fetchmany",
        test_name=f"concur_{size}_N{worker_count}",
    )


@pytest.mark.supported_drivers("python")
@pytest.mark.universal_only
@pytest.mark.iterations(5)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,size", SIZES)
@pytest.mark.parametrize("worker_count", WORKERS)
def test_concur_aio_fetchmany(perf_test, row_count, size, worker_count):
    perf_test(
        test_type=PerfTestType.CONCURRENT,
        sql_command=get_sql("number", row_count),
        worker_count=worker_count,
        fetch_mode="aio",
        test_name=f"concur_{size}_N{worker_count}_aio",
    )


@pytest.mark.supported_drivers("python")
@pytest.mark.universal_only
@pytest.mark.iterations(3)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,size,worker_count", HIGH_CONCUR)
def test_concur_aio_fetchmany_high_workers(perf_test, row_count, size, worker_count):
    perf_test(
        test_type=PerfTestType.CONCURRENT,
        sql_command=get_sql("number", row_count),
        worker_count=worker_count,
        fetch_mode="aio",
        test_name=f"concur_{size}_N{worker_count}_aio",
    )
