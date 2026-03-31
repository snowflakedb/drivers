"""Results output and CSV formatting."""

import csv
import json
import os
import sys
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
    subdir = "_record" if test_name.endswith("_record") else test_name
    results_dir = Path("/results") / driver_type / subdir
    results_dir.mkdir(parents=True, exist_ok=True)
    
    filename = results_dir / f"{test_name}_python_{driver_type}_{timestamp}.csv"
    
    with open(filename, 'w', newline='') as f:
        if test_type == TestType.PUT_GET:
            writer = csv.DictWriter(f, fieldnames=[
                "timestamp_ms", "query_s", "cpu_time_s", "peak_rss_mb",
            ])
            writer.writeheader()
            for result in results:
                writer.writerow({
                    "timestamp_ms": result['timestamp'],
                    "query_s": f"{result['query_time_s']:.6f}",
                    "cpu_time_s": f"{result['cpu_time_s']:.6f}",
                    "peak_rss_mb": f"{result['peak_rss_mb']:.1f}",
                })
        else:
            writer = csv.DictWriter(f, fieldnames=[
                "timestamp_ms", "query_s", "fetch_s", "row_count", "cpu_time_s", "peak_rss_mb",
            ])
            writer.writeheader()
            for result in results:
                writer.writerow({
                    "timestamp_ms": result['timestamp'],
                    "query_s": f"{result['query_time_s']:.6f}",
                    "fetch_s": f"{result['fetch_time_s']:.6f}",
                    "row_count": result.get('row_count', 0),
                    "cpu_time_s": f"{result['cpu_time_s']:.6f}",
                    "peak_rss_mb": f"{result['peak_rss_mb']:.1f}",
                })
    
    return filename


def write_memory_timeline(memory_timeline, test_name, driver_type):
    """Write memory timeline samples to a separate CSV file.
    
    Args:
        memory_timeline: List of MemorySample objects from ResourceMonitor
        test_name: Name of the test
        driver_type: Driver type (universal or old)
    
    Returns:
        Path: Path to the created CSV file, or None if no samples
    """
    if not memory_timeline:
        return None

    timestamp = int(time.time())
    subdir = "_record" if test_name.endswith("_record") else test_name
    results_dir = Path("/results") / driver_type / subdir
    results_dir.mkdir(parents=True, exist_ok=True)

    filename = results_dir / f"memory_timeline_{test_name}_python_{driver_type}_{timestamp}.csv"

    with open(filename, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=["timestamp_ms", "rss_bytes", "vm_bytes"])
        writer.writeheader()
        for sample in memory_timeline:
            writer.writerow({
                "timestamp_ms": sample.timestamp_ms,
                "rss_bytes": sample.rss_bytes,
                "vm_bytes": sample.vm_bytes,
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
    runtime_language_version = _get_python_version()  # Python version at runtime
    
    timestamp = int(time.time())
    metadata = {
        "driver": "python",
        "driver_type": driver_type,
        "driver_version": driver_version,
        "runtime_language_version": runtime_language_version,  # Python runtime version
        "server_version": server_version,
        "architecture": architecture,
        "os": os_info,
        "run_timestamp": timestamp,
    }
    
    # Only add build_rust_version for universal driver (old driver is pure Python, no Rust)
    # This is the Rust compiler version that built libsf_core.so (from sf-core-builder)
    if driver_type == "universal":
        metadata["build_rust_version"] = _get_build_rust_version()
    
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


def _get_python_version():
    """Get Python version (e.g., '3.13')."""
    return f"{sys.version_info.major}.{sys.version_info.minor}"


def _get_build_rust_version():
    """Get Rust compiler version that built libsf_core.so (from sf-core-builder)."""
    return os.environ.get('BUILD_RUST_VERSION', 'NA')
