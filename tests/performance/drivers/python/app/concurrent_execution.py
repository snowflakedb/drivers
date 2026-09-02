"""Same-connection concurrent SELECT bursts.

One Snowflake connection, N worker threads, each with its own cursor.
"""

from concurrent.futures import ThreadPoolExecutor, as_completed
import statistics
import threading
import time

from common import get_peak_rss_mb, print_timing_stats
from query_execution import _FETCH_STRATEGIES
from resource_monitor import ResourceMonitor


def execute_concurrent_test(
    conn,
    sql_command,
    warmup_iterations,
    iterations,
    worker_count,
    fetch_mode="fetchmany",
):
    """Run warmup + measured bursts. Returns (results, memory_timeline)."""
    print("\n=== Executing Concurrent SELECT Test ===")
    print(f"Query: {sql_command}")
    print(f"Workers: {worker_count} threads on one connection")
    print(f"Fetch mode: {fetch_mode}")

    fetch_fn = _FETCH_STRATEGIES[fetch_mode]

    for i in range(warmup_iterations):
        print(f"  Warmup burst {i + 1}/{warmup_iterations}")
        _run_burst(conn, sql_command, worker_count, fetch_fn)

    monitor = ResourceMonitor(interval_s=0.1)
    monitor.start()

    results = []
    for i in range(iterations):
        results.append(_run_burst(conn, sql_command, worker_count, fetch_fn))
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


def _barrier_timeout_s(worker_count: int) -> float:
    return min(900.0, max(120.0, worker_count * 0.5))


def _run_burst(conn, sql, worker_count, fetch_fn):
    burst_start = {"t": None}
    barrier_timeout = _barrier_timeout_s(worker_count)

    def mark_start():
        burst_start["t"] = time.perf_counter()

    barrier = threading.Barrier(worker_count + 1, action=mark_start)
    cpu_start = time.process_time()

    def worker():
        barrier.wait(timeout=barrier_timeout)
        with conn.cursor() as cursor:
            cursor.execute(sql)
            return fetch_fn(cursor)

    with ThreadPoolExecutor(max_workers=worker_count) as pool:
        futures = [pool.submit(worker) for _ in range(worker_count)]
        barrier.wait(timeout=barrier_timeout)
        worker_results = []
        for future in as_completed(futures):
            worker_results.append(future.result())

    burst_wall_s = time.perf_counter() - burst_start["t"]
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


def _validate_row_counts(results, worker_count):
    if not results:
        return
    expected = results[0]["row_count"]
    if expected == 0:
        raise RuntimeError(
            "Row count baseline is 0 — refusing to use 0 as a concurrent-burst baseline."
        )
    if expected % worker_count != 0:
        raise RuntimeError(
            f"Total row count {expected} is not divisible by worker_count {worker_count}"
        )
    per_worker = expected // worker_count
    for i, result in enumerate(results):
        if result["row_count"] != expected:
            raise RuntimeError(
                f"Row count mismatch: iteration {i} returned {result['row_count']} rows, "
                f"expected {expected} ({worker_count} workers × {per_worker})"
            )
    print(
        f"✓ All {len(results)} bursts returned {expected} rows "
        f"({worker_count} × {per_worker})"
    )


def _print_statistics(results):
    burst_times = [r["query_time_s"] for r in results]
    throughputs = [r["throughput_rows_s"] for r in results]
    print("\nSummary:")
    print_timing_stats("Burst wall", burst_times)
    print(
        f"  Throughput: median={statistics.median(throughputs):.0f} rows/s  "
        f"min={min(throughputs):.0f}  max={max(throughputs):.0f}"
    )
