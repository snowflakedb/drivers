"""Query execution and performance measurement."""

import time
import statistics


def execute_query(cursor, sql):
    """Execute a single query and collect metrics.
    
    Returns:
        dict: Dictionary with query_time_s, fetch_time_s, and row_count
    """
    query_start = time.time()
    cursor.execute(sql)
    query_time = time.time() - query_start
    
    fetch_start = time.time()
    row_count = 0
    for _ in cursor:
        row_count += 1
    fetch_time = time.time() - fetch_start
    
    return {
        "query_time_s": query_time,
        "fetch_time_s": fetch_time,
        "row_count": row_count,
    }


def run_warmup(cursor, sql, warmup_iterations):
    """Run warmup iterations."""
    if warmup_iterations == 0:
        return
    
    for _ in range(warmup_iterations):
        execute_query(cursor, sql)


def run_test_iterations(cursor, sql, iterations):
    """Run test iterations and return results."""
    results = []
    
    for i in range(iterations):
        result = execute_query(cursor, sql)
        results.append(result)
    
    return results


def print_statistics(results):
    """Print summary statistics for test results."""
    query_times = [r['query_time_s'] for r in results]
    fetch_times = [r['fetch_time_s'] for r in results]
    
    print(f"\nSummary:")
    print(f"  Query: median={statistics.median(query_times):.3f}s  "
          f"min={min(query_times):.3f}s  max={max(query_times):.3f}s")
    print(f"  Fetch: median={statistics.median(fetch_times):.3f}s  "
          f"min={min(fetch_times):.3f}s  max={max(fetch_times):.3f}s")

