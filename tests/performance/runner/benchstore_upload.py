#!/usr/bin/env python
"""
Upload performance test results to Benchstore (CBS).

This module provides functions for uploading performance metrics to Benchstore.
It can be used both as a standalone script and called from pytest hooks.
"""

import argparse
import os
import csv
import json
import logging
import re
from collections import defaultdict
from pathlib import Path
from typing import Optional, Dict, List

import yaml
from benchstore.storage.sf_storage import SnowflakeConnectionParams, SFStorage
from benchstore.proto import benchstore_pb2
from benchstore.client import benchmark_manager
from benchstore.client.quickstore import Quickstore
from google.protobuf.timestamp_pb2 import Timestamp

from runner.container import get_resource_limits
from runner.utils import perf_tests_root, collect_node_info

logger = logging.getLogger(__name__)

# Benchstore only allows A-Za-z0-9 - _ = . : in tag strings.
_TAG_SANITIZE_RE = re.compile(r"[^A-Za-z0-9\-_=.:]")


def _sanitize_tag(tag: str) -> str:
    """Replace characters not allowed by Benchstore with underscores."""
    return _TAG_SANITIZE_RE.sub("_", tag)


PROJECT_NAME = "SnowDrivers"
BENCHMARK_NAME = "Universal_Driver"
PERFORMANCE_TESTS_DIR = perf_tests_root()


def get_snowhouse_config() -> dict:
    """
    Load Snowhouse configuration from YAML file.
    
    Looks for config file in the following order:
    1. Path specified in SF_PERF_CONFIG environment variable
    2. ~/sf_perf_config.yml
    
    Returns:
        dict: Configuration dictionary
        
    Raises:
        Exception: If config file not found
    """
    config_file = os.getenv('SF_PERF_CONFIG')
    home = os.path.expanduser("~")
    
    if config_file is None or not os.path.exists(config_file):
        config_file = os.path.join(home, "sf_perf_config.yml")
    
    if os.path.exists(config_file):
        with open(config_file) as f:
            return yaml.load(f, Loader=yaml.FullLoader)
    
    raise Exception("Snowhouse config file not found. Please set SF_PERF_CONFIG or create ~/sf_perf_config.yml")


def get_snowflake_connection_params(snowhouse_config: dict) -> SnowflakeConnectionParams:
    """
    Extract Snowflake connection parameters from config file.
    
    Args:
        snowhouse_config: Configuration dictionary from snowhouse config file
        
    Returns:
        SnowflakeConnectionParams: Connection parameters for Snowflake (using config credentials)
    """
    return SnowflakeConnectionParams(
        authenticator="snowflake",
        user=snowhouse_config['config']['sf']['user'],
        password=snowhouse_config['config']['sf']['password']
    )


def get_local_connection_params() -> SnowflakeConnectionParams:
    """
    Get local Snowflake connection parameters using browser authentication.
    
    Returns:
        SnowflakeConnectionParams: Connection parameters for local execution (using externalbrowser)
    """
    return SnowflakeConnectionParams(
        authenticator="externalbrowser",
        user=os.getenv("USER"),
    )


def login_to_benchstore(use_local_auth: bool = False) -> SFStorage:
    """
    Authenticate and establish connection to Benchstore.
    
    This function:
    1. Loads configuration from snowhouse config file (if not using local auth)
    2. Extracts Snowflake connection parameters
    3. Creates and returns an authenticated SFStorage instance
    
    Args:
        use_local_auth: If True, use local browser authentication. 
                       If False, use credentials from config file.
    
    Returns:
        SFStorage: Authenticated storage instance
        
    Raises:
        Exception: If configuration is missing or authentication fails
    """

    if use_local_auth:
        snowflake_connection_params = get_local_connection_params()
    else:
        snowhouse_config = get_snowhouse_config()
        snowflake_connection_params = get_snowflake_connection_params(snowhouse_config)
    
    # Create storage instance (authentication is performed here)
    try:
        sf_storage = SFStorage(snowflake_connection_params=snowflake_connection_params)
        return sf_storage
        
    except Exception as e:
        logger.error("=" * 60)
        logger.error("❌ Authentication failed!")
        logger.error(f"Error: {e}")
        logger.error("=" * 60)
        raise


def parse_cloud_provider_from_parameters(parameters_json_path: str) -> Optional[str]:
    """
    Extract cloud provider from parameters filename.
    
    Expected format: parameters_perf_{provider}.json
    Example: parameters_perf_aws.json -> AWS
    
    Args:
        parameters_json_path: Path to parameters JSON file
        
    Returns:
        Cloud provider name (uppercase) or None if not found
    """
    if not parameters_json_path:
        return None
    
    filename = Path(parameters_json_path).name
    match = re.search(r'parameters_perf_(\w+)\.json', filename)
    if match:
        provider = match.group(1).upper()
        return provider
    
    logger.warning(f"Could not extract cloud provider from filename: {filename}")
    return None


def extract_region_from_parameters(parameters_json_path: str) -> str:
    """
    Extract region from Snowflake host in parameters JSON.
    
    Examples:
        sfctest0.us-west-2.aws.snowflakecomputing.com -> us-west-2
        sfctest0.east-us-2.azure.snowflakecomputing.com -> east-us-2
        sfctest0.snowflakecomputing.com -> us-west-2 (default)
    
    Args:
        parameters_json_path: Path to parameters JSON file
        
    Returns:
        Region identifier or UNKNOWN
    """
    if not parameters_json_path or not Path(parameters_json_path).exists():
        return "UNKNOWN"
    
    try:
        with open(parameters_json_path, 'r') as f:
            params = json.load(f)
        
        host = params.get('testconnection', {}).get('SNOWFLAKE_TEST_HOST', '')
        if not host:
            return "UNKNOWN"
        
        parts = host.split('.')
        if len(parts) >= 4:
            # Format: account.region.cloud.snowflakecomputing.com
            return parts[1]
        
        # Default format: account.snowflakecomputing.com (us-west-2 implicit)
        return "us-west-2"
    except Exception as e:
        logger.warning(f"Could not extract region from parameters: {e}")
        return "UNKNOWN"


def read_run_metadata(results_dir: Path) -> Dict[str, Dict]:
    """
    Read all run metadata files from results directory.
    
    Expected filenames: run_metadata_{driver}_{driver_type}.json
    Examples: 
      - run_metadata_python_universal.json
      - run_metadata_core.json
    
    Args:
        results_dir: Directory containing result files
        
    Returns:
        Dict mapping (driver, driver_type) tuples to metadata dicts
        
    Raises:
        Exception: If no metadata files found or if any file cannot be read
    """
    metadata_by_driver = {}
    
    # Find all run metadata files
    metadata_files = list(results_dir.glob("run_metadata_*.json"))
    
    if not metadata_files:
        raise Exception(f"No run metadata files found in results directory: {results_dir}")
    
    for metadata_file in metadata_files:
        try:
            with open(metadata_file, 'r') as f:
                metadata = json.load(f)
            
            driver = metadata.get('driver')
            driver_type = metadata.get('driver_type')
            
            if not driver or not driver_type:
                raise Exception(f"Metadata file {metadata_file.name} missing required fields (driver or driver_type)")
            
            key = (driver, driver_type)
            metadata_by_driver[key] = metadata
                
        except Exception as e:
            raise Exception(f"Failed to read metadata from {metadata_file.name}: {e}")
    
    return metadata_by_driver


def parse_csv_filename(filename: str) -> Optional[Dict[str, str]]:
    """
    Parse performance test CSV filename.
    
    Expected formats: 
      - {test_name}_{driver}_{driver_type}_{timestamp}.csv (python, odbc, jdbc)
      - {test_name}_{driver}_{timestamp}.csv (core only has universal)
    Examples:
      - fetch_string_1000000_rows_odbc_universal_1761569440.csv
      - fetch_string_1000000_rows_core_1761569440.csv
    
    Args:
        filename: CSV filename
        
    Returns:
        Dict with test_name, driver, driver_type, timestamp or None if parsing fails
    """
    # First try pattern with driver_type (python, odbc, jdbc)
    pattern_with_type = r'(.+)_(python|odbc|jdbc)_(universal|old)_(\d+)\.csv'
    match = re.match(pattern_with_type, filename)
    
    if match:
        return {
            'test_name': match.group(1),
            'driver': match.group(2),
            'driver_type': match.group(3),
            'timestamp': match.group(4),
        }
    
    # Try pattern without driver_type (core only has universal)
    pattern_core = r'(.+)_(core)_(\d+)\.csv'
    match = re.match(pattern_core, filename)
    
    if match:
        return {
            'test_name': match.group(1),
            'driver': match.group(2),
            'driver_type': 'universal',  # core only has universal
            'timestamp': match.group(3),
        }
    
    logger.warning(f"Could not parse filename: {filename}")
    return None


def read_csv_results(csv_path: Path) -> List[Dict]:
    """
    Read performance test results from CSV file.
    
    Args:
        csv_path: Path to CSV file
        
    Returns:
        List of dicts, each containing timestamp and metrics for one iteration
    """
    results = []
    
    with open(csv_path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            result = {
                'timestamp': int(row['timestamp_ms']),
                'query_s': float(row['query_s'])
            }
            
            if 'fetch_s' in row:
                result['fetch_s'] = float(row['fetch_s'])
            if 'cpu_time_s' in row:
                result['cpu_time_s'] = float(row['cpu_time_s'])
            if 'peak_rss_mb' in row:
                result['peak_rss_mb'] = float(row['peak_rss_mb'])
            
            results.append(result)
    
    return results


def parse_memory_timeline_filename(filename: str) -> Optional[Dict[str, str]]:
    """
    Parse memory timeline CSV filename.
    
    Expected formats:
      - memory_timeline_{test_name}_{driver}_{driver_type}_{timestamp}.csv
      - memory_timeline_{test_name}_{driver}_{timestamp}.csv (core)
    Examples:
      - memory_timeline_select_string_1M_arrow_recorded_http_python_universal_1773658001.csv
      - memory_timeline_select_string_1M_arrow_recorded_http_core_1773658001.csv
    
    Args:
        filename: CSV filename
        
    Returns:
        Dict with test_name, driver, driver_type, timestamp or None if not a timeline file
    """
    pattern_with_type = r'memory_timeline_(.+)_(python|odbc|jdbc)_(universal|old)_(\d+)\.csv'
    match = re.match(pattern_with_type, filename)
    if match:
        return {
            'test_name': match.group(1),
            'driver': match.group(2),
            'driver_type': match.group(3),
            'timestamp': match.group(4),
        }
    
    pattern_core = r'memory_timeline_(.+)_(core)_(\d+)\.csv'
    match = re.match(pattern_core, filename)
    if match:
        return {
            'test_name': match.group(1),
            'driver': match.group(2),
            'driver_type': 'universal',
            'timestamp': match.group(3),
        }
    
    return None


def read_memory_timeline(csv_path: Path) -> List[Dict]:
    """
    Read memory timeline samples from CSV file.
    
    Args:
        csv_path: Path to memory timeline CSV file
        
    Returns:
        List of dicts with timestamp_ms, rss_bytes, vm_bytes
    """
    samples = []
    with open(csv_path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            if row['timestamp_ms'] and row['rss_bytes'] and row['vm_bytes']:
                samples.append({
                    'timestamp_ms': int(row['timestamp_ms']),
                    'rss_bytes': int(row['rss_bytes']),
                    'vm_bytes': int(row['vm_bytes']),
                })
    return samples


def bytes_to_mb(size_in_bytes: int) -> float:
    return round(size_in_bytes / (1024 * 1024), 2)


def upload_metrics(results_dir: Optional[Path] = None, use_local_auth: bool = False, parameters_json_path: Optional[str] = None):
    """
    Core upload logic for Benchstore.
    
    This function:
    1. Finds CSV result files in results_dir
    2. Parses each file to extract test metadata and results
    3. Gathers environment tags (SERVER_VERSION, DRIVER_VERSION, CLOUD_PROVIDER, etc.)
    4. Creates/finds benchmark in Benchstore
    5. Uploads metrics
    
    Args:
        results_dir: Path to results directory to upload. If None, uses default results/
                     For pytest runs, this should be the run-specific directory.
        use_local_auth: If True, use local browser authentication. 
                       If False, use credentials from config file.
        parameters_json_path: Path to parameters JSON file (for extracting cloud provider and server version)
    
    Returns:
        None
        
    Raises:
        Exception: If validation or authentication fails
    """
    if results_dir is None:
        results_dir = PERFORMANCE_TESTS_DIR / "results"
        # Try to find latest run directory
        if results_dir.exists():
            run_dirs = sorted(results_dir.glob("run_*"), reverse=True)
            if run_dirs:
                results_dir = run_dirs[0]
    
    if not results_dir.exists():
        raise Exception(f"Results directory does not exist: {results_dir}")
    
    csv_files = (
        list(results_dir.glob("universal/**/*.csv")) +
        list(results_dir.glob("old/**/*.csv"))
    )
    if not csv_files:
        raise Exception(f"No CSV files found in: {results_dir}")
    
    # Get environment tags
    is_local = os.getenv('BUILD_NUMBER') is None
    build_number = "LOCAL" if is_local else os.getenv('BUILD_NUMBER')
    branch_name = "LOCAL" if is_local else os.getenv('BRANCH_NAME', 'unknown')
    
    # Get cloud provider from parameters filename
    cloud_provider = parse_cloud_provider_from_parameters(parameters_json_path)
    if cloud_provider is None:
        cloud_provider = "UNKNOWN"
        logger.warning("Cloud provider not detected, using: UNKNOWN")
    
    # Extract region from parameters
    region = extract_region_from_parameters(parameters_json_path)
    
    run_metadata = read_run_metadata(results_dir)
    
    # Login to Benchstore
    sf_storage = login_to_benchstore(use_local_auth=use_local_auth)
    
    # Get or create benchmark
    try:
        # Get connection params for benchmark manager
        if use_local_auth:
            snowflake_connection_params = get_local_connection_params()
        else:
            snowhouse_config = get_snowhouse_config()
            snowflake_connection_params = get_snowflake_connection_params(snowhouse_config)
        
        _ = benchmark_manager.find_or_create_benchmark(
            PROJECT_NAME,
            BENCHMARK_NAME,
            sf_storage
        )
    except Exception as e:
        logger.error(f"Failed to find/create benchmark: {e}")
        raise
    
    # Group CSV files by their tag set (driver + driver_type)
    csv_groups = defaultdict(list)
    timeline_groups = defaultdict(list)
    
    for csv_file in csv_files:
        # Check if this is a memory timeline file first
        timeline_info = parse_memory_timeline_filename(csv_file.name)
        if timeline_info is not None:
            test_name = timeline_info['test_name']
            if test_name.endswith('_record'):
                logger.info(f"Skipping recording phase timeline: {csv_file.name}")
                continue
            group_key = (timeline_info['driver'], timeline_info['driver_type'])
            timeline_groups[group_key].append((csv_file, timeline_info))
            continue
        
        # Parse as regular results CSV
        file_info = parse_csv_filename(csv_file.name)
        if file_info is None:
            logger.warning(f"Skipping {csv_file.name} - could not parse filename")
            continue
        
        test_name = file_info['test_name']
        if test_name.endswith('_record'):
            logger.info(f"Skipping recording phase result: {csv_file.name}")
            continue
        
        driver = file_info['driver']
        driver_type = file_info['driver_type']
        group_key = (driver, driver_type)
        csv_groups[group_key].append((csv_file, file_info))
    
    total_uploaded = 0
    
    # Get resource limits from container configuration
    resource_limits = get_resource_limits()
    docker_memory = resource_limits['memory']
    docker_cpu = resource_limits['cpu']
    
    jenkins_node = os.getenv('JENKINS_NODE_LABEL', 'UNKNOWN')
    trigger_type = os.getenv('TRIGGER_TYPE', 'LOCAL' if is_local else 'unknown')
    
    # Collect host hardware info for node equivalence tracking
    node_info = collect_node_info()
    
    for (driver, driver_type), csv_file_list in csv_groups.items():
        # Get versions from run metadata
        metadata_key = (driver, driver_type)
        if metadata_key not in run_metadata:
            raise Exception(f"No metadata found for {driver} ({driver_type})")
        
        metadata = run_metadata[metadata_key]
        client_version = metadata.get('driver_version', 'UNKNOWN')
        server_version = metadata.get('server_version', 'UNKNOWN')
        architecture = metadata.get('architecture', 'UNKNOWN')
        os_info = metadata.get('os', 'UNKNOWN')
        
        # Extract versions from metadata
        # BUILD_RUST_VERSION: Rust compiler version that built the code
        # RUNTIME_LANGUAGE_VERSION: Runtime language version (NA for compiled Rust and ODBC)
        build_rust_version = metadata.get('build_rust_version', 'NA')
        runtime_language_version = metadata.get('runtime_language_version', 'NA')
        
        driver_tag_value = f"{driver}_old" if driver_type == "old" else driver
        
        tags = [
            f"BUILD_NUMBER={build_number}",
            f"BRANCH_NAME={branch_name}",
            f"ARCHITECTURE={architecture}",
            f"OS={os_info}",
            f"DRIVER={driver_tag_value}",
            f"SERVER_VERSION={server_version}",
            f"DRIVER_VERSION={client_version}",
            f"BUILD_RUST_VERSION={build_rust_version}",
            f"RUNTIME_LANGUAGE_VERSION={runtime_language_version}",
            f"CLOUD_PROVIDER={cloud_provider}",
            f"REGION={region}",
            f"TRIGGER_TYPE={trigger_type}",
            f"JENKINS_NODE={jenkins_node}",
            f"DOCKER_MEMORY={docker_memory}",
            f"DOCKER_CPU={docker_cpu}",
            f"NODE_CPU_MODEL={node_info.get('node_cpu_model', 'UNKNOWN')}",
            f"NODE_CPU_CORES={node_info.get('node_cpu_cores', 'UNKNOWN')}",
            f"NODE_CPU_THREADS={node_info.get('node_cpu_threads', 'UNKNOWN')}",
            f"NODE_MEMORY_GB={node_info.get('node_memory_gb', 'UNKNOWN')}",
            f"NODE_CPU_MAX_MHZ={node_info.get('node_cpu_max_mhz', 'UNKNOWN')}",
            f"NODE_L3_CACHE={node_info.get('node_l3_cache', 'UNKNOWN')}",
        ]
        
        if "node_instance_type" in node_info:
            tags.append(f"NODE_INSTANCE_TYPE={node_info['node_instance_type']}")
        
        tags = [_sanitize_tag(t) for t in tags]
        default_comparable_tags = list(tags)
        
        quickstore_input = benchstore_pb2.QuickstoreInput(
            benchmark_name_lookup=benchstore_pb2.BenchmarkNameLookup(
                project_name=PROJECT_NAME,
                benchmark_name=BENCHMARK_NAME,
            ),
            tags=tags,
            default_comparable_tags=default_comparable_tags,
        )
        
        # Open Quickstore once for all CSV files in this group
        try:
            with Quickstore(quickstore_input, snowflake_connection_params=snowflake_connection_params) as quickstore:
                results_by_test = {}
                
                # Upload all CSV files in this group
                for csv_file, file_info in csv_file_list:
                    test_name = file_info['test_name']
                    
                    # Read CSV results
                    try:
                        results = read_csv_results(csv_file)
                    except Exception as e:
                        logger.error(f"Failed to read {csv_file.name}: {e}")
                        continue
                    
                    if not results:
                        logger.warning(f"No results in {csv_file.name}")
                        continue
                    
                    results_by_test[test_name] = results
                    
                    # Upload all iterations from this CSV
                    for idx, result in enumerate(results, 1):
                        metrics = {
                            f"{test_name}_query_s": result['query_s'],
                        }
                        
                        # fetch_s is only present in SELECT tests, not PUT/GET tests
                        if 'fetch_s' in result:
                            metrics[f"{test_name}_fetch_s"] = result['fetch_s']
                        if 'cpu_time_s' in result:
                            metrics[f"{test_name}_cpu_time_s"] = result['cpu_time_s']
                        if 'peak_rss_mb' in result:
                            metrics[f"{test_name}_peak_rss_mb"] = result['peak_rss_mb']
                        
                        timestamp = Timestamp()
                        timestamp.FromMilliseconds(result['timestamp'])
                        
                        quickstore.add_sample_point_from_input(
                            benchstore_pb2.AddSamplePointInput(
                                timestamp=timestamp,
                                metrics=metrics,
                            )
                        )
                        
                        total_uploaded += 1
                
                # Upload memory timeline data for this driver group (gauge metrics)
                # Per BenchStore docs: memory is a gauge - upload periodic samples and
                # let Quickstore auto-compute aggregates (max, min, mean, median, percentiles)
                group_key = (driver, driver_type)
                if group_key in timeline_groups:
                    for timeline_file, timeline_info in timeline_groups[group_key]:
                        test_name = timeline_info['test_name']
                        
                        try:
                            samples = read_memory_timeline(timeline_file)
                        except Exception as e:
                            logger.error(f"Failed to read timeline {timeline_file.name}: {e}")
                            continue
                        
                        if not samples:
                            logger.warning(f"No samples in {timeline_file.name}")
                            continue
                        
                        rss_metric = f"{test_name}_rss_memory_mb"
                        vm_metric = f"{test_name}_vm_memory_mb"
                        
                        for sample in samples:
                            timestamp = Timestamp()
                            timestamp.FromMilliseconds(sample['timestamp_ms'])
                            
                            quickstore.add_sample_point_from_input(
                                benchstore_pb2.AddSamplePointInput(
                                    timestamp=timestamp,
                                    metrics={
                                        rss_metric: bytes_to_mb(sample['rss_bytes']),
                                        vm_metric: bytes_to_mb(sample['vm_bytes']),
                                    },
                                )
                            )
                        
                        # Upload max RSS delta as a run aggregate for quick regression detection
                        rss_values = [s['rss_bytes'] for s in samples]
                        rss_delta_mb = bytes_to_mb(max(rss_values) - min(rss_values))
                        quickstore.add_run_aggregate(
                            benchstore_pb2.AddRunAggregateInput(
                                custom_aggregate_label=f"{test_name}_rss_memory_delta_mb",
                                custom_aggregate_value=rss_delta_mb,
                            )
                        )
                        
                        # Upload memory growth for leak detection.
                        # Compares the idle (minimum) RSS of the first iteration to the
                        # last iteration. A growing value indicates memory not being
                        # freed between iterations (potential leak).
                        if test_name in results_by_test:
                            iteration_results = results_by_test[test_name]
                            if len(iteration_results) >= 2:
                                iter_end_times_ms = [r['timestamp'] for r in iteration_results]
                                first_iter_rss = [s['rss_bytes'] for s in samples if s['timestamp_ms'] <= iter_end_times_ms[0]]
                                last_iter_rss = [s['rss_bytes'] for s in samples if s['timestamp_ms'] > iter_end_times_ms[-2]]
                                
                                if first_iter_rss and last_iter_rss:
                                    growth_mb = max(0.0, bytes_to_mb(min(last_iter_rss) - min(first_iter_rss)))
                                    quickstore.add_run_aggregate(
                                        benchstore_pb2.AddRunAggregateInput(
                                            custom_aggregate_label=f"{test_name}_rss_memory_growth_mb",
                                            custom_aggregate_value=growth_mb,
                                        )
                                    )
                                    logger.info(f"  Memory growth for {test_name}: {growth_mb} MB")
                        
                        timeline_uploaded = len(samples)
                        total_uploaded += timeline_uploaded
                        logger.info(f"  Uploaded {timeline_uploaded} memory timeline samples for {test_name}")
            
            logger.critical(f"✓ Uploaded {driver} ({driver_type}) [{architecture}/{os_info}] driver data to Benchstore")
                
        except Exception as e:
            logger.error(f"Failed to upload {driver} ({driver_type}): {e}")
            continue
    
    # Warn about timeline files with no matching results group
    orphaned = set(timeline_groups.keys()) - set(csv_groups.keys())
    for key in orphaned:
        logger.warning(f"Memory timeline files for {key} have no matching results CSVs - skipped")


def main():
    """
    Main entry point for standalone script execution.
    
    Configures logging and calls upload_metrics().
    """
    parser = argparse.ArgumentParser(description="Upload performance test results to Benchstore")
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=None,
        help="Path to results directory (defaults to results/ or latest run_* directory)"
    )
    parser.add_argument(
        "--local-benchstore-upload",
        action="store_true",
        default=False,
        help="Use local authentication (externalbrowser) for Benchstore upload instead of config file credentials"
    )
    parser.add_argument(
        "--parameters-json",
        type=str,
        default=None,
        help="Path to parameters JSON file (for extracting cloud provider and server version)"
    )
    args = parser.parse_args()
    
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)8s] %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    
    try:
        # If no results dir specified, try to find the latest run directory
        results_dir = args.results_dir
        if results_dir is None:
            base_results = PERFORMANCE_TESTS_DIR / "results"
            if base_results.exists():
                # Find latest run_* directory
                run_dirs = sorted(base_results.glob("run_*"), reverse=True)
                if run_dirs:
                    results_dir = run_dirs[0]
        
        upload_metrics(
            results_dir=results_dir,
            use_local_auth=args.local_benchstore_upload,
            parameters_json_path=args.parameters_json
        )
        
    except Exception as e:
        logger.error(f"\n❌ Error: {e}")
        raise


if __name__ == "__main__":
    main()


