"""Same-connection concurrent SELECT bursts via snowflake.connector.aio."""

import asyncio
import time

from common import get_peak_rss_mb
from concurrent_execution import _print_statistics, _validate_row_counts
from query_execution import _FETCH_BATCH_SIZE
from resource_monitor import ResourceMonitor


async def _fetch_many_chunks(cursor):
    row_count = 0
    while True:
        rows = await cursor.fetchmany(_FETCH_BATCH_SIZE)
        if not rows:
            break
        row_count += len(rows)
    return row_count


async def _run_burst(conn, sql, worker_count):
    cpu_start = time.process_time()
    barrier = asyncio.Barrier(worker_count)

    async def worker():
        await barrier.wait()
        async with conn.cursor() as cursor:
            await cursor.execute(sql)
            return await _fetch_many_chunks(cursor)

    burst_wall_start = time.perf_counter()
    worker_results = await asyncio.gather(*(worker() for _ in range(worker_count)))
    burst_wall_s = time.perf_counter() - burst_wall_start
    cpu_time_s = time.process_time() - cpu_start
    total_rows = sum(worker_results)
    per_worker_rows = set(worker_results)
    if len(per_worker_rows) != 1:
        raise RuntimeError(
            f"Workers returned unequal row counts: {sorted(per_worker_rows)}"
        )

    return {
        "timestamp": int(time.time() * 1000),
        "query_time_s": burst_wall_s,
        "fetch_time_s": burst_wall_s,
        "row_count": total_rows,
        "cpu_time_s": cpu_time_s,
        "peak_rss_mb": get_peak_rss_mb(),
        "worker_count": worker_count,
        "throughput_rows_s": total_rows / burst_wall_s if burst_wall_s > 0 else 0.0,
    }


async def _execute_concurrent_aio_test(
    conn,
    sql_command,
    warmup_iterations,
    iterations,
    worker_count,
):
    print("\n=== Executing Concurrent aio SELECT Test ===")
    print(f"Query: {sql_command}")
    print(f"Workers: {worker_count} concurrent tasks on one aio connection")
    print("Fetch mode: aio (async fetchmany)")

    for i in range(warmup_iterations):
        print(f"  Warmup burst {i + 1}/{warmup_iterations}")
        await _run_burst(conn, sql_command, worker_count)

    monitor = ResourceMonitor(interval_s=0.1)
    monitor.start()

    results = []
    for i in range(iterations):
        results.append(await _run_burst(conn, sql_command, worker_count))
        print(
            f"  Iteration {i + 1}/{iterations}: "
            f"burst={results[-1]['query_time_s']:.3f}s  "
            f"throughput={results[-1]['throughput_rows_s']:.0f} rows/s  "
            f"rows={results[-1]['row_count']}"
        )

    memory_timeline = monitor.stop()
    _validate_row_counts(results, worker_count)
    _print_statistics(results)
    print(f"  Memory timeline: {len(memory_timeline)} samples collected")
    return results, memory_timeline


def execute_concurrent_aio_test(
    conn,
    sql_command,
    warmup_iterations,
    iterations,
    worker_count,
    fetch_mode="aio",
):
    """Same signature as execute_concurrent_test; runs bursts on the aio event loop."""
    return asyncio.get_event_loop().run_until_complete(
        _execute_concurrent_aio_test(
            conn, sql_command, warmup_iterations, iterations, worker_count
        )
    )
