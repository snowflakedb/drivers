"""Query execution and performance measurement."""
import os
import time
from common import run_warmup, run_test_iterations, print_timing_stats

_FETCH_BATCH_SIZE = 1024


def execute_fetch_test(cursor, sql_command, warmup_iterations, iterations):
    """
    Execute a complete SELECT test: warmup, iterations, and statistics.
    
    Returns:
        list: Test results for CSV output
    """
    print("\n=== Executing SELECT Test ===")
    print(f"Query: {sql_command}")
    
    run_warmup(_execute_query, cursor, sql_command, warmup_iterations)
    results = run_test_iterations(_execute_query, cursor, sql_command, iterations)
    _validate_row_counts(results)
    _print_statistics(results)
    
    return results


def _validate_row_counts(results):
    """Validate that all iterations returned the same number of rows."""
    
    if not results:
        return

    expected_count, start_idx = _get_expected_row_count(results)
    
    for i, result in enumerate(results[start_idx:], start=start_idx):
        _check_row_count_match(result['row_count'], expected_count, i)
    
    print(f"✓ All {len(results)} iterations returned {expected_count} rows")


def _print_statistics(results):
    """Print summary statistics for test results."""
    query_times = [r['query_time_s'] for r in results]
    fetch_times = [r['fetch_time_s'] for r in results]
    
    print(f"\nSummary:")
    print_timing_stats("Query", query_times)
    print_timing_stats("Fetch", fetch_times)


def _get_expected_row_count(results):
    """
    Get expected row count from environment or first iteration.
    
    Returns:
        tuple: (expected_count, start_idx) where start_idx is the first index to validate
    """
    expected_from_recording = os.getenv("EXPECTED_ROW_COUNT")
    if expected_from_recording:
        expected_count = int(expected_from_recording)
        print(f"Row count baseline: {expected_count} rows (from recording phase)")
        return expected_count, 0  # Validate all iterations including first
    else:
        # Use first iteration as baseline
        expected_count = results[0]['row_count']
        print(f"Row count baseline: {expected_count} rows (from first iteration)")
        return expected_count, 1  # Skip first iteration since it's the baseline


def _check_row_count_match(actual_count, expected_count, iteration):
    """
    Validate a single row count matches expected.
    
    Raises:
        RuntimeError: If actual count doesn't match expected count
    """
    if actual_count != expected_count:
        raise RuntimeError(
            f"Row count mismatch: iteration {iteration} returned {actual_count} rows, "
            f"expected {expected_count} rows"
        )


def _execute_query(cursor, sql):
    """Execute a single query and collect metrics.
    
    Returns:
        dict: Dictionary with timestamp, query_time_s, fetch_time_s, and row_count
    """
    query_start = time.time()
    cursor.execute(sql)
    query_time = time.time() - query_start
    
    fetch_start = time.time()
    row_count = 0
    while True:
        rows = cursor.fetchmany(_FETCH_BATCH_SIZE)
        if not rows:
            break
        row_count += len(rows)
    fetch_time = time.time() - fetch_start

    timestamp = int(time.time())
    
    return {
        "timestamp": timestamp,
        "query_time_s": query_time,
        "fetch_time_s": fetch_time,
        "row_count": row_count,
    }

