"""Common test execution utilities for performance tests."""
import logging
import os
from pathlib import Path

from runner.container import create_perf_container, run_container
from runner.test_types import PerfTestType
from runner.validation import verify_results

logger = logging.getLogger(__name__)


def execute_test(
    test_name: str,
    sql_command: str,
    parameters_json: str,
    results_dir: Path,
    iterations: int,
    warmup_iterations: int,
    driver: str = "core",
    driver_type: str = None,
    setup_queries: list[str] = None,
    test_type: PerfTestType = PerfTestType.SELECT,
    use_local_binary: bool = False,
    s3_files_dir: Path = None,
    env_vars: dict[str, str] = None,
    network_mode: str = "host",
) -> None:
    """
    Execute a test using either local binary or Docker container.
    
    Args:
        test_name: Name of the test (used for result filenames)
        sql_command: SQL command to execute
        parameters_json: JSON string with connection parameters
        results_dir: Directory to store results
        iterations: Number of test iterations
        warmup_iterations: Number of warmup iterations
        driver: Driver to use (core, python, odbc, jdbc)
        driver_type: Driver type: 'universal' or 'old' (only 'universal' for core)
        setup_queries: Optional list of SQL queries to run before warmup/test iterations
        test_type: Type of test (PerfTestType.SELECT or PerfTestType.PUT_GET)
        use_local_binary: Use locally built binary instead of Docker (Core only)
        s3_files_dir: Optional directory with S3-downloaded files to mount (for PUT/GET tests)
        env_vars: Optional environment variables to pass to the test
        network_mode: Docker network mode ("host" for direct host network, reduces jitter)
    """
    if use_local_binary and driver == "core":
        # Run locally built Core binary
        from runner.modes.local_runner import run_local_core_binary
        
        # Set environment variables if provided
        if env_vars:
            for key, value in env_vars.items():
                os.environ[key] = value
        
        run_local_core_binary(
            test_name=test_name,
            sql_command=sql_command,
            parameters_json=parameters_json,
            results_dir=results_dir,
            iterations=iterations,
            warmup_iterations=warmup_iterations,
            setup_queries=setup_queries,
            test_type=test_type,
            s3_files_dir=s3_files_dir,
        )
    else:
        # Create container
        container = create_perf_container(
            driver=driver,
            parameters_json=parameters_json,
            sql_command=sql_command,
            test_name=test_name,
            iterations=iterations,
            warmup_iterations=warmup_iterations,
            results_dir=results_dir,
            driver_type=driver_type,
            setup_queries=setup_queries,
            test_type=test_type,
            s3_files_dir=s3_files_dir,
            network_mode=network_mode,
        )
        
        # Add environment variables if provided
        if env_vars:
            for key, value in env_vars.items():
                container = container.with_env(key, value)
        
        # Run container
        run_container(container)


def verify_test_results(
    results_dir: Path,
    test_name: str,
    driver: str,
    iterations: int,
    driver_type: str = None,
) -> list[Path]:
    """
    Verify test results and return result file paths.
    
    Args:
        results_dir: Directory containing results
        test_name: Name of the test
        driver: Driver that was used
        iterations: Expected number of iterations
        driver_type: Driver type ('universal' or 'old')
    
    Returns:
        List of result file paths
    """
    return verify_results(
        results_dir,
        test_name,
        driver,
        iterations,
        driver_type=driver_type,
    )
