"""Results output for single-implementation perf drivers (sqlapi, adbc)."""

import csv
import json
import os
import sys
import time
from pathlib import Path


def write_csv_results(results, test_name, driver):
    timestamp = int(time.time())
    subdir = "_record" if test_name.endswith("_record") else test_name
    results_dir = Path("/results") / "universal" / subdir
    results_dir.mkdir(parents=True, exist_ok=True)
    filename = results_dir / f"{test_name}_{driver}_{timestamp}.csv"

    with open(filename, "w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "timestamp_ms",
                "query_s",
                "fetch_s",
                "row_count",
                "cpu_time_s",
                "peak_rss_mb",
            ],
        )
        writer.writeheader()
        for result in results:
            writer.writerow(
                {
                    "timestamp_ms": result["timestamp"],
                    "query_s": f"{result['query_time_s']:.6f}",
                    "fetch_s": f"{result['fetch_time_s']:.6f}",
                    "row_count": result.get("row_count", 0),
                    "cpu_time_s": f"{result['cpu_time_s']:.6f}",
                    "peak_rss_mb": f"{result['peak_rss_mb']:.1f}",
                }
            )
    return filename


def write_memory_timeline(memory_timeline, test_name, driver):
    if not memory_timeline:
        return None
    timestamp = int(time.time())
    subdir = "_record" if test_name.endswith("_record") else test_name
    results_dir = Path("/results") / "universal" / subdir
    results_dir.mkdir(parents=True, exist_ok=True)
    filename = results_dir / f"memory_timeline_{test_name}_{driver}_{timestamp}.csv"
    with open(filename, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["timestamp_ms", "rss_bytes", "vm_bytes"])
        writer.writeheader()
        for sample in memory_timeline:
            writer.writerow(
                {
                    "timestamp_ms": sample.timestamp_ms,
                    "rss_bytes": sample.rss_bytes,
                    "vm_bytes": sample.vm_bytes,
                }
            )
    return filename


def write_run_metadata(server_version, driver, driver_version):
    metadata_filename = Path("/results") / f"run_metadata_{driver}.json"
    if metadata_filename.exists():
        return
    metadata = {
        "driver": driver,
        "driver_type": "universal",
        "driver_version": driver_version,
        "runtime_language_version": f"{sys.version_info.major}.{sys.version_info.minor}",
        "server_version": server_version,
        "architecture": _get_architecture(),
        "os": os.environ.get("OS_INFO", "Linux"),
        "run_timestamp": int(time.time()),
        "build_rust_version": "NA",
    }
    metadata_filename.write_text(json.dumps(metadata, indent=2))


def _get_architecture():
    import platform

    machine = platform.machine().lower()
    if machine in ("amd64", "x64", "x86_64"):
        return "x86_64"
    if machine in ("aarch64", "armv8"):
        return "arm64"
    return machine
