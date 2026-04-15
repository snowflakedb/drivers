"""End-to-end performance test runner (direct Snowflake connection)."""
import logging
from pathlib import Path

from runner.modes.common import execute_test, extract_limit_from_sql, verify_test_results
from runner.test_types import PerfTestType

logger = logging.getLogger(__name__)


def run_performance_test(
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
) -> list[Path]:
    """
    Run a performance test with the specified configuration.
    
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
    
    Returns:
        List of result file paths created
    """
    driver_label = f"{driver.upper()}"
    if driver != "core" and driver_type:
        driver_label += f" ({driver_type})"
    
    if use_local_binary:
        driver_label += " (local binary)"
    
    logger.info(f"Running {test_name} ({driver_label}): {iterations} iterations [type={test_type}]")
    
    env_vars = {}
    expected = extract_limit_from_sql(sql_command)
    if expected:
        env_vars["EXPECTED_ROW_COUNT"] = str(expected)
    
    execute_test(
        test_name=test_name,
        sql_command=sql_command,
        parameters_json=parameters_json,
        results_dir=results_dir,
        iterations=iterations,
        warmup_iterations=warmup_iterations,
        driver=driver,
        driver_type=driver_type,
        setup_queries=setup_queries,
        test_type=test_type,
        use_local_binary=use_local_binary,
        s3_files_dir=s3_files_dir,
        env_vars=env_vars,
    )
    
    return verify_test_results(
        results_dir,
        test_name,
        driver,
        iterations,
        driver_type=driver_type,
    )


def run_comparison_test(
    test_name: str,
    sql_command: str,
    parameters_json: str,
    results_dir: Path,
    iterations: int,
    warmup_iterations: int,
    driver: str,
    setup_queries: list[str] = None,
    test_type: PerfTestType = PerfTestType.SELECT,
    s3_files_dir: Path = None,
) -> dict[str, list[Path]]:
    """
    Run the same test on both universal and old driver implementations.
    
    Args:
        test_name: Name of the test (used for result filenames)
        sql_command: SQL command to execute
        parameters_json: JSON string with connection parameters
        results_dir: Directory to store results
        iterations: Number of test iterations
        warmup_iterations: Number of warmup iterations
        driver: Driver to test (python, odbc, jdbc)
        setup_queries: Optional list of SQL queries to run before warmup/test iterations
        test_type: Type of test (PerfTestType.SELECT or PerfTestType.PUT_GET)
        s3_files_dir: Optional directory with S3-downloaded files to mount (for PUT/GET tests)
    
    Returns:
        Dict with 'universal' and 'old' keys, each containing list of result file paths
    """
    logger.info(f"Running {test_name} comparison ({driver.upper()}): Universal vs Old [type={test_type}]")
    
    results = {}
    
    # Run Universal driver first
    logger.info("")
    logger.info(">>> DRIVER: Universal")
    logger.info("")
    results['universal'] = run_performance_test(
        test_name=test_name,
        sql_command=sql_command,
        parameters_json=parameters_json,
        results_dir=results_dir,
        iterations=iterations,
        warmup_iterations=warmup_iterations,
        driver=driver,
        driver_type="universal",
        setup_queries=setup_queries,
        test_type=test_type,
        s3_files_dir=s3_files_dir,
    )
    
    # Run Old driver second
    logger.info("")
    logger.info(">>> DRIVER: Old")
    logger.info("")
    results['old'] = run_performance_test(
        test_name=test_name,
        sql_command=sql_command,
        parameters_json=parameters_json,
        results_dir=results_dir,
        iterations=iterations,
        warmup_iterations=warmup_iterations,
        driver=driver,
        driver_type="old",
        setup_queries=setup_queries,
        test_type=test_type,
        s3_files_dir=s3_files_dir,
    )
    
    return results

