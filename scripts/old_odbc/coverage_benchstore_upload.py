#!/usr/bin/env python
"""
Upload old ODBC coverage statistics to Benchstore.

This script parses lcov summary.txt file and uploads the coverage metrics
(line, function, branch coverage percentages) to Benchstore for tracking.
"""

import argparse
import os
import re
import logging
import time
from pathlib import Path
from typing import Dict, Optional

import yaml
from benchstore.storage.sf_storage import SnowflakeConnectionParams, SFStorage
from benchstore.proto import benchstore_pb2
from benchstore.client import benchmark_manager
from benchstore.client.quickstore import Quickstore
from google.protobuf.timestamp_pb2 import Timestamp

logger = logging.getLogger(__name__)

PROJECT_NAME = "SnowDrivers"
BENCHMARK_NAME = "OLD_Coverage"


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
    """
    return SnowflakeConnectionParams(
        authenticator="snowflake",
        user=snowhouse_config['config']['sf']['user'],
        password=snowhouse_config['config']['sf']['password']
    )


def get_local_connection_params() -> SnowflakeConnectionParams:
    """
    Get local Snowflake connection parameters using browser authentication.
    """
    return SnowflakeConnectionParams(
        authenticator="externalbrowser",
        user=os.getenv("USER"),
    )


def login_to_benchstore(use_local_auth: bool = False) -> SFStorage:
    """
    Authenticate and establish connection to Benchstore.
    """
    if use_local_auth:
        snowflake_connection_params = get_local_connection_params()
    else:
        snowhouse_config = get_snowhouse_config()
        snowflake_connection_params = get_snowflake_connection_params(snowhouse_config)
    
    try:
        sf_storage = SFStorage(snowflake_connection_params=snowflake_connection_params)
        return sf_storage
        
    except Exception as e:
        logger.error("=" * 60)
        logger.error("❌ Authentication failed!")
        logger.error(f"Error: {e}")
        logger.error("=" * 60)
        raise


def parse_coverage_summary(summary_path: Path) -> Dict[str, float]:
    """
    Parse lcov summary.txt file to extract coverage statistics.
    
    Expected format:
        lines......: 30.8% (4923 of 16003 lines)
        functions..: 10.1% (569 of 5620 functions)
        branches...: no data found
    
    Args:
        summary_path: Path to summary.txt file
        
    Returns:
        Dict with coverage percentages for lines, functions, and branches
    """
    if not summary_path.exists():
        raise Exception(f"Coverage summary file not found: {summary_path}")
    
    metrics = {
        'line_coverage_pct': 0.0,
        'lines_found': 0.0,
        'lines_hit': 0.0,
        'function_coverage_pct': 0.0,
        'functions_found': 0.0,
        'functions_hit': 0.0,
        'branch_coverage_pct': 0.0,
        'branches_found': 0.0,
        'branches_hit': 0.0,
    }
    
    # Pattern to match: "lines......: 30.8% (4923 of 16003 lines)"
    coverage_pattern = re.compile(r'(\w+)\.+:\s*([\d.]+)%\s*\((\d+)\s+of\s+(\d+)')
    
    with open(summary_path, 'r') as f:
        for line in f:
            match = coverage_pattern.search(line)
            if match:
                metric_type = match.group(1).lower()
                percentage = float(match.group(2))
                hit = int(match.group(3))
                found = int(match.group(4))
                
                if metric_type == 'lines':
                    metrics['line_coverage_pct'] = percentage
                    metrics['lines_hit'] = float(hit)
                    metrics['lines_found'] = float(found)
                elif metric_type == 'functions':
                    metrics['function_coverage_pct'] = percentage
                    metrics['functions_hit'] = float(hit)
                    metrics['functions_found'] = float(found)
                elif metric_type == 'branches':
                    metrics['branch_coverage_pct'] = percentage
                    metrics['branches_hit'] = float(hit)
                    metrics['branches_found'] = float(found)
    
    return metrics


def upload_coverage_metrics(
    summary_path: Path,
    use_local_auth: bool = False
):
    """
    Upload coverage metrics to Benchstore.
    
    Args:
        summary_path: Path to lcov summary.txt file
        use_local_auth: If True, use local browser authentication
    """
    logger.info(f"Parsing coverage data from: {summary_path}")
    metrics = parse_coverage_summary(summary_path)
    
    logger.info("Coverage Statistics:")
    logger.info(f"  Line coverage:     {metrics['line_coverage_pct']:.2f}% ({int(metrics['lines_hit'])}/{int(metrics['lines_found'])})")
    logger.info(f"  Function coverage: {metrics['function_coverage_pct']:.2f}% ({int(metrics['functions_hit'])}/{int(metrics['functions_found'])})")
    logger.info(f"  Branch coverage:   {metrics['branch_coverage_pct']:.2f}% ({int(metrics['branches_hit'])}/{int(metrics['branches_found'])})")
    
    # Get environment tags
    is_local = os.getenv('BUILD_NUMBER') is None
    build_number = "LOCAL" if is_local else os.getenv('BUILD_NUMBER')
    branch_name = "LOCAL" if is_local else os.getenv('BRANCH_NAME', 'unknown')
    
    # Login to Benchstore
    sf_storage = login_to_benchstore(use_local_auth=use_local_auth)
    
    # Get connection params
    if use_local_auth:
        snowflake_connection_params = get_local_connection_params()
    else:
        snowhouse_config = get_snowhouse_config()
        snowflake_connection_params = get_snowflake_connection_params(snowhouse_config)
    
    # Get or create benchmark
    try:
        _ = benchmark_manager.find_or_create_benchmark(
            PROJECT_NAME,
            BENCHMARK_NAME,
            sf_storage
        )
    except Exception as e:
        logger.error(f"Failed to find/create benchmark: {e}")
        raise
    
    # Build tags
    tags = [
        f"BUILD_NUMBER={build_number}",
        f"BRANCH_NAME={branch_name}",
        f"TYPE=old_odbc_coverage",
    ]
    
    # Create Quickstore input
    quickstore_input = benchstore_pb2.QuickstoreInput(
        benchmark_name_lookup=benchstore_pb2.BenchmarkNameLookup(
            project_name=PROJECT_NAME,
            benchmark_name=BENCHMARK_NAME,
        ),
        tags=tags,
        default_comparable_tags=tags,
    )
    
    # Upload metrics
    try:
        with Quickstore(quickstore_input, snowflake_connection_params=snowflake_connection_params) as quickstore:
            timestamp = Timestamp()
            timestamp.FromSeconds(int(time.time()))
            
            quickstore.add_sample_point_from_input(
                benchstore_pb2.AddSamplePointInput(
                    timestamp=timestamp,
                    metrics=metrics,
                )
            )
        
        logger.info("✓ Coverage metrics uploaded to Benchstore successfully")
        
    except Exception as e:
        logger.error(f"Failed to upload coverage metrics: {e}")
        raise


def main():
    """
    Main entry point for standalone script execution.
    """
    parser = argparse.ArgumentParser(description="Upload old ODBC coverage statistics to Benchstore")
    parser.add_argument(
        "--summary",
        type=Path,
        required=True,
        help="Path to lcov summary.txt file"
    )
    parser.add_argument(
        "--local-benchstore-upload",
        action="store_true",
        default=False,
        help="Use local authentication (externalbrowser) for Benchstore upload"
    )
    args = parser.parse_args()
    
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)8s] %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    
    try:
        upload_coverage_metrics(
            summary_path=args.summary,
            use_local_auth=args.local_benchstore_upload
        )
    except Exception as e:
        logger.error(f"\n❌ Error: {e}")
        raise


if __name__ == "__main__":
    main()

