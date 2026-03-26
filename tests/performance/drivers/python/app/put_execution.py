"""PUT/GET execution and performance measurement."""

import os
import re
import shutil
import time
from common import run_warmup, run_test_iterations, print_timing_stats, get_peak_rss_mb
from resource_monitor import ResourceMonitor


def execute_put_get_test(cursor, sql_command, warmup_iterations, iterations):
    """
    Execute a complete PUT/GET test: warmup, iterations, and statistics.
    
    Returns:
        tuple: (results list, memory_timeline list)
    """
    print("\n=== Executing PUT_GET Test ===")
    print(f"Query: {sql_command}")
    
    run_warmup(_execute_put_get, cursor, sql_command, warmup_iterations)

    monitor = ResourceMonitor(interval_s=0.1)
    monitor.start()

    results = run_test_iterations(_execute_put_get, cursor, sql_command, iterations)

    memory_timeline = monitor.stop()

    print_statistics(results)
    print(f"  Memory timeline: {len(memory_timeline)} samples collected")
    
    return results, memory_timeline


def print_statistics(results):
    """Print summary statistics for test results."""
    query_times = [r['query_time_s'] for r in results]
    
    print(f"\nSummary:")
    print_timing_stats("Operation time", query_times)


def _execute_put_get(cursor, sql):
    """
    Execute a PUT or GET command and collect metrics.
    
    Returns:
        dict: Dictionary with timestamp, query_time_s, cpu_time_s, and peak_rss_mb
    """
    _create_get_target_directory(sql)
    
    cpu_start = time.process_time()

    query_start = time.time()
    cursor.execute(sql)
    query_time = time.time() - query_start

    cpu_time_s = time.process_time() - cpu_start
    peak_rss_mb = get_peak_rss_mb()

    timestamp = int(time.time() * 1000)
    
    return {
        "timestamp": timestamp,
        "query_time_s": query_time,
        "cpu_time_s": cpu_time_s,
        "peak_rss_mb": peak_rss_mb,
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
