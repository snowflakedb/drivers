"""Results output and CSV formatting."""

import csv
import json
import time
from pathlib import Path
from test_types import TestType


def write_csv_results(results, test_name, driver_type, test_type: TestType = TestType.SELECT):
    """Write test results to CSV file.
    
    Args:
        results: List of result dictionaries
        test_name: Name of the test
        driver_type: Driver type (universal or old)
        test_type: Type of test (TestType.SELECT or TestType.PUT_GET)
    
    Returns:
        Path: Path to the created CSV file
    """
    timestamp = int(time.time())
    results_dir = Path("/results")
    results_dir.mkdir(exist_ok=True)
    
    filename = results_dir / f"{test_name}_python_{driver_type}_{timestamp}.csv"
    
    with open(filename, 'w', newline='') as f:
        if test_type == TestType.PUT_GET:
            # PUT/GET tests: timestamp and query_s
            writer = csv.DictWriter(f, fieldnames=["timestamp", "query_s"])
            writer.writeheader()
            for result in results:
                writer.writerow({
                    "timestamp": result['timestamp'],
                    "query_s": f"{result['query_time_s']:.6f}",
                })
        else:
            # SELECT tests: timestamp, query_s and fetch_s
            writer = csv.DictWriter(f, fieldnames=["timestamp", "query_s", "fetch_s"])
            writer.writeheader()
            for result in results:
                writer.writerow({
                    "timestamp": result['timestamp'],
                    "query_s": f"{result['query_time_s']:.6f}",
                    "fetch_s": f"{result['fetch_time_s']:.6f}",
                })
    
    return filename


def write_run_metadata(driver_type, driver_version, server_version):
    """Write run metadata JSON file (once per run).
    
    Args:
        driver_type: Driver type (universal or old)
        driver_version: Version string of the driver
        server_version: Snowflake server version
    """
    results_dir = Path("/results")
    metadata_filename = results_dir / f"run_metadata_python_{driver_type}.json"
    
    # Only write if doesn't exist (shared across all tests in run)
    if metadata_filename.exists():
        return
    
    # Detect architecture and OS inside container
    architecture = _get_architecture()
    os_info = _get_os_version()
    
    timestamp = int(time.time())
    metadata = {
        "driver": "python",
        "driver_type": driver_type,
        "driver_version": driver_version,
        "server_version": server_version,
        "architecture": architecture,
        "os": os_info,
        "run_timestamp": timestamp,
    }
    
    with open(metadata_filename, 'w') as f:
        json.dump(metadata, f, indent=2)


def _get_architecture():
    """Detect architecture inside the container."""
    import platform
    
    machine = platform.machine().lower()
    
    if machine in ('amd64', 'x64', 'x86_64'):
        return 'x86_64'
    elif machine in ('aarch64', 'armv8'):
        return 'arm64'
    else:
        return machine


def _get_os_version():
    """Get OS version from environment variable (exported at container startup)."""
    import os
    return os.environ.get('OS_INFO', 'Linux')

