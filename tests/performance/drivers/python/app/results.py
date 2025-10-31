"""Results output and CSV formatting."""

import csv
import json
import time
from pathlib import Path


def write_csv_results(results, test_name, driver_type):
    """Write test results to CSV file.
    
    Args:
        results: List of result dictionaries
        test_name: Name of the test
        driver_type: Driver type (universal or old)
    
    Returns:
        Path: Path to the created CSV file
    """
    timestamp = int(time.time())
    results_dir = Path("/results")
    results_dir.mkdir(exist_ok=True)
    
    filename = results_dir / f"{test_name}_python_{driver_type}_{timestamp}.csv"
    
    with open(filename, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=["query_time_s", "fetch_time_s"])
        writer.writeheader()
        for result in results:
            writer.writerow({
                "query_time_s": f"{result['query_time_s']:.6f}",
                "fetch_time_s": f"{result['fetch_time_s']:.6f}",
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
    
    timestamp = int(time.time())
    metadata = {
        "driver": "python",
        "driver_type": driver_type,
        "driver_version": driver_version,
        "server_version": server_version,
        "run_timestamp": timestamp,
    }
    
    with open(metadata_filename, 'w') as f:
        json.dump(metadata, f, indent=2)

