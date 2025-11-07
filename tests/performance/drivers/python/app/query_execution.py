"""Query execution and performance measurement."""

import time
from common import run_warmup, run_test_iterations, print_timing_stats


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
    print_statistics(results)
    
    return results


def print_statistics(results):
    """Print summary statistics for test results."""
    query_times = [r['query_time_s'] for r in results]
    fetch_times = [r['fetch_time_s'] for r in results]
    
    print(f"\nSummary:")
    print_timing_stats("Query", query_times)
    print_timing_stats("Fetch", fetch_times)


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
    for _ in cursor:
        row_count += 1
    fetch_time = time.time() - fetch_start
    
    timestamp = int(time.time())
    
    return {
        "timestamp": timestamp,
        "query_time_s": query_time,
        "fetch_time_s": fetch_time,
        "row_count": row_count,
    }

