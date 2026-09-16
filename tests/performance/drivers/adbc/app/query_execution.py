"""ADBC SELECT: query_s = execute; fetch_s = iterate Arrow record batches."""

from __future__ import annotations

import os
import time

from common import get_peak_rss_mb, print_timing_stats, run_test_iterations, run_warmup
from resource_monitor import ResourceMonitor


def execute_fetch_test(cursor, sql_command, warmup_iterations, iterations):
    print("\n=== Executing SELECT Test (ADBC) ===")
    print(f"Query: {sql_command}")
    print("Fetch mode: arrow_batches (cursor.fetch_record_batch())")

    run_warmup(_execute_query, cursor, sql_command, warmup_iterations)
    monitor = ResourceMonitor(interval_s=0.1)
    monitor.start()
    results = run_test_iterations(_execute_query, cursor, sql_command, iterations)
    memory_timeline = monitor.stop()
    _validate_row_counts(results)
    query_times = [r["query_time_s"] for r in results]
    fetch_times = [r["fetch_time_s"] for r in results]
    print("\nSummary:")
    print_timing_stats("Query", query_times)
    print_timing_stats("Fetch", fetch_times)
    print(f"  Memory timeline: {len(memory_timeline)} samples collected")
    return results, memory_timeline


def _fetch_record_batches(cursor) -> int:
    """Consume Arrow batches without converting to Python rows. Returns row count."""
    reader = cursor.fetch_record_batch()
    row_count = 0
    for batch in reader:
        row_count += batch.num_rows
    return row_count


def _execute_query(cursor, sql):
    query_start = time.time()
    cursor.execute(sql)
    query_time = time.time() - query_start

    cpu_start = time.process_time()
    fetch_start = time.time()
    row_count = _fetch_record_batches(cursor)
    fetch_time = time.time() - fetch_start

    expected = os.getenv("EXPECTED_ROW_COUNT")
    if expected and row_count != int(expected):
        raise RuntimeError(
            f"ADBC row count mismatch: got {row_count}, expected {int(expected)}"
        )

    return {
        "timestamp": int(time.time() * 1000),
        "query_time_s": query_time,
        "fetch_time_s": fetch_time,
        "row_count": row_count,
        "cpu_time_s": time.process_time() - cpu_start,
        "peak_rss_mb": get_peak_rss_mb(),
    }


def _validate_row_counts(results):
    if not results:
        return
    expected = int(os.getenv("EXPECTED_ROW_COUNT") or results[0]["row_count"])
    if expected == 0:
        raise RuntimeError("Row count baseline is 0")
    for i, result in enumerate(results):
        if result["row_count"] != expected:
            raise RuntimeError(
                f"Row count mismatch: iteration {i} returned {result['row_count']}, "
                f"expected {expected}"
            )
    print(f"✓ All {len(results)} iterations returned {expected} rows")
