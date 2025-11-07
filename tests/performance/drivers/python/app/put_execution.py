"""PUT/GET execution and performance measurement."""

import time
import os
import re
import shutil
from common import run_warmup, run_test_iterations, print_timing_stats


def execute_put_get_test(cursor, sql_command, warmup_iterations, iterations):
    """
    Execute a complete PUT/GET test: warmup, iterations, and statistics.
    
    Returns:
        list: Test results for CSV output
    """
    print("\n=== Executing PUT_GET Test ===")
    print(f"Query: {sql_command}")
    
    run_warmup(_execute_put_get, cursor, sql_command, warmup_iterations)
    results = run_test_iterations(_execute_put_get, cursor, sql_command, iterations)
    print_statistics(results)
    
    return results


def print_statistics(results):
    """Print summary statistics for test results."""
    query_times = [r['query_time_s'] for r in results]
    
    print(f"\nSummary:")
    print_timing_stats("Operation time", query_times)


def _execute_put_get(cursor, sql):
    """
    Execute a PUT or GET command and collect metrics.
    
    Returns:
        dict: Dictionary with timestamp and query_time_s
    """
    _create_get_target_directory(sql)
    
    query_start = time.time()
    cursor.execute(sql)
    query_time = time.time() - query_start
    
    timestamp = int(time.time())
    
    return {
        "timestamp": timestamp,
        "query_time_s": query_time,
    }


def _create_get_target_directory(sql):
    """
    Prepare target directory for GET commands.
    
    For GET commands:
    - Removes existing directory to ensure clean iteration
    - Creates fresh directory structure
    """
    if sql.strip().upper().startswith('GET'):
        match = re.search(r'file://([^\s]+)', sql)
        if match:
            target_path = match.group(1)
            if os.path.exists(target_path):
                shutil.rmtree(target_path)
            os.makedirs(target_path, exist_ok=True)
