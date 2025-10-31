import logging
from pathlib import Path

from runner.container import create_perf_container, run_container
from runner.validation import verify_results

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
    use_local_binary: bool = False,
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
        use_local_binary: Use locally built binary instead of Docker (Core only)
    
    Returns:
        List of result file paths created
    """
    driver_label = f"{driver.upper()}"
    if driver != "core" and driver_type:
        driver_label += f" ({driver_type})"
    
    if use_local_binary:
        driver_label += " (local binary)"
    
    logger.info(f"Running {test_name} ({driver_label}): {iterations} iterations")
    
    if use_local_binary and driver == "core":
        # Run locally built Core binary
        from runner.local_runner import run_local_core_binary
        run_local_core_binary(
            test_name=test_name,
            sql_command=sql_command,
            parameters_json=parameters_json,
            results_dir=results_dir,
            iterations=iterations,
            warmup_iterations=warmup_iterations,
            setup_queries=setup_queries,
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
        )
        
        # Run container
        run_container(container)
    
    # Verify and return results
    result_files = verify_results(
        results_dir,
        test_name,
        driver,
        iterations,
        driver_type=driver_type,
    )
    
    return result_files


def run_comparison_test(
    test_name: str,
    sql_command: str,
    parameters_json: str,
    results_dir: Path,
    iterations: int,
    warmup_iterations: int,
    driver: str,
    setup_queries: list[str] = None,
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
    
    Returns:
        Dict with 'universal' and 'old' keys, each containing list of result file paths
    """
    logger.info(f"Running {test_name} comparison ({driver.upper()}): Universal vs Old")
    
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
    )
    
    return results

