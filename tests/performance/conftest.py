"""pytest configuration for performance tests"""
import os
import logging
from pathlib import Path
from datetime import datetime
import pytest

logger = logging.getLogger(__name__)

# Track test failures across the session
_test_failures = []

# Track current run directory (session-scoped)
_current_run_dir = None


def pytest_addoption(parser):
    """Add custom command line options"""
    parser.addoption(
        "--parameters-json",
        action="store",
        default=str(Path("parameters") / "parameters_perf_aws.json"),
        help="Path to parameters.json file",
    )
    parser.addoption(
        "--iterations",
        action="store",
        default=None,
        type=int,
        help="Number of test iterations (default: 2, or per-test marker)",
    )
    parser.addoption(
        "--warmup-iterations",
        action="store",
        default=None,
        type=int,
        help="Number of warmup iterations (default: 1, or per-test marker)",
    )
    parser.addoption(
        "--driver",
        action="store",
        default="core",
        help="Driver to test: core, python, odbc, jdbc",
    )
    parser.addoption(
        "--driver-type",
        action="store",
        default="universal",
        help="Driver type: universal, old, both (runs both sequentially). Not applicable for core (only has universal).",
    )
    parser.addoption(
        "--upload-to-benchstore",
        action="store_true",
        default=False,
        help="Upload metrics to Benchstore after test run",
    )
    parser.addoption(
        "--local-benchstore-upload",
        action="store_true",
        default=False,
        help="Use local authentication (externalbrowser) for Benchstore upload instead of config file credentials",
    )
    parser.addoption(
        "--use-local-binary",
        action="store_true",
        default=False,
        help="Use locally built binary instead of Docker container (Core only)",
    )


@pytest.fixture
def parameters_json_path(request):
    """Get parameters JSON path from command line or environment"""
    return request.config.getoption("--parameters-json") or os.getenv(
        "PARAMETERS_JSON", str(Path("..") / ".." / "parameters.json")
    )


@pytest.fixture
def parameters_json(parameters_json_path):
    """Read and return parameters JSON content"""
    with open(parameters_json_path, 'r') as f:
        return f.read()


@pytest.fixture
def iterations(request):
    """
    Get number of iterations with precedence:
    1. Command line args (--iterations=N) - highest priority
    2. Test-level marker (@pytest.mark.iterations(N))
    3. Environment variable (PERF_ITERATIONS)
    4. Default value (2) - lowest priority
    """
    # 1. Check command line (explicitly provided)
    cli_value = request.config.getoption("--iterations")
    if cli_value is not None:
        return cli_value
    
    # 2. Check test-level marker
    marker = request.node.get_closest_marker("iterations")
    if marker is not None:
        if marker.args:
            return marker.args[0]
        elif "value" in marker.kwargs:
            return marker.kwargs["value"]
    
    # 3. Check environment variable
    env_value = os.getenv("PERF_ITERATIONS")
    if env_value is not None:
        return int(env_value)
    
    # 4. Default
    return 2


@pytest.fixture
def warmup_iterations(request):
    """
    Get number of warmup iterations with precedence:
    1. Command line args (--warmup-iterations=N) - highest priority
    2. Test-level marker (@pytest.mark.warmup_iterations(N))
    3. Environment variable (PERF_WARMUP_ITERATIONS)
    4. Default value (1) - lowest priority
    """
    # 1. Check command line (explicitly provided)
    cli_value = request.config.getoption("--warmup-iterations")
    if cli_value is not None:
        return cli_value
    
    # 2. Check test-level marker
    marker = request.node.get_closest_marker("warmup_iterations")
    if marker is not None:
        if marker.args:
            return marker.args[0]
        elif "value" in marker.kwargs:
            return marker.kwargs["value"]
    
    # 3. Check environment variable
    env_value = os.getenv("PERF_WARMUP_ITERATIONS")
    if env_value is not None:
        return int(env_value)
    
    # 4. Default
    return 1


@pytest.fixture
def driver(request):
    """Get driver name from command line or environment"""
    return request.config.getoption("--driver") or os.getenv("PERF_DRIVER", "core")


@pytest.fixture
def driver_type(request):
    """Get driver type from command line or environment"""
    driver_type_value = request.config.getoption("--driver-type") or os.getenv("DRIVER_TYPE", "universal")
    driver_value = request.config.getoption("--driver") or os.getenv("PERF_DRIVER", "core")
    
    # Validate: Core driver only has universal implementation
    if driver_value == "core" and driver_type_value != "universal":
        raise pytest.UsageError(
            f"--driver-type is not supported for {driver_value} driver. "
            f"Core only has one implementation (universal). "
            f"Got: --driver={driver_value} --driver-type={driver_type_value}"
        )
    
    return driver_type_value


@pytest.fixture(scope="session")
def run_id():
    """Generate a unique run ID for this test session"""
    return datetime.now().strftime("%Y%m%d_%H%M%S")


@pytest.fixture(scope="session")
def session_results_dir(run_id):
    """
    Create and return run-specific results directory for this test session.
    
    Structure: results/run_YYYYMMDD_HHMMSS/
    """
    global _current_run_dir
    
    base_results = Path("results").absolute()
    base_results.mkdir(exist_ok=True)
    
    run_dir = base_results / f"run_{run_id}"
    run_dir.mkdir(exist_ok=True)
    
    _current_run_dir = run_dir
    
    logger.info(f"Results for this run will be saved to: {run_dir}")
    
    return run_dir


@pytest.fixture
def results_dir(session_results_dir):
    """Return the session-specific results directory"""
    return session_results_dir


@pytest.fixture
def use_local_binary(request):
    """Get use-local-binary flag from command line"""
    return request.config.getoption("--use-local-binary")


@pytest.fixture
def perf_test(parameters_json, results_dir, iterations, warmup_iterations, driver, driver_type, use_local_binary, request):
    """
    Returns a callable for running performance tests with pre-configured parameters.
    
    Usage in tests:
        def test_example(perf_test):
            perf_test(
                sql_command="SELECT 1",
                setup_queries=["ALTER SESSION SET QUERY_TAG = 'perf_test'"]  # optional
            )
    
    Note: ARROW format is automatically enabled. Any setup_queries provided will be
    appended after "alter session set query_result_format = 'ARROW'".
    
    The test_name is automatically derived from the test function name (strips "test_" prefix).
    You can also explicitly provide test_name if needed.
    
    For drivers with --driver-type=both, runs test twice (universal, then old)
    """
    from runner.runner import run_performance_test, run_comparison_test
    
    # Validate: local binary only works with Core
    if use_local_binary and driver != "core":
        raise pytest.UsageError(
            f"--use-local-binary is only supported for Core driver. Got: --driver={driver}"
        )
    
    def _run_test(sql_command: str, setup_queries: list[str] = None, test_name: str = None):
        # Auto-derive test name from function name if not provided
        if test_name is None:
            func_name = request.node.name
            if func_name.startswith("test_"):
                test_name = func_name[5:]  # Strip "test_" prefix
            else:
                test_name = func_name
        
        # Always use ARROW format, then append any additional setup queries
        arrow_query = "alter session set query_result_format = 'ARROW'"
        if setup_queries is None:
            final_setup_queries = [arrow_query]
        else:
            final_setup_queries = [arrow_query] + setup_queries
        
        # For drivers with "both" option, run comparison
        # Core only has universal implementation
        if driver_type == "both" and driver != "core":
            return run_comparison_test(
                test_name=test_name,
                sql_command=sql_command,
                setup_queries=final_setup_queries,
                parameters_json=parameters_json,
                results_dir=results_dir,
                iterations=iterations,
                warmup_iterations=warmup_iterations,
                driver=driver,
            )
        else:
            return run_performance_test(
                test_name=test_name,
                sql_command=sql_command,
                setup_queries=final_setup_queries,
                parameters_json=parameters_json,
                results_dir=results_dir,
                iterations=iterations,
                warmup_iterations=warmup_iterations,
                driver=driver,
                driver_type=driver_type if driver != "core" else None,
                use_local_binary=use_local_binary,
            )
    
    return _run_test


def pytest_runtest_setup(item):
    """Hook called before each test starts - add visual separation."""
    logger.info("")
    logger.info("=" * 80)
    logger.info(f">>> TEST: {item.name}")
    logger.info("=" * 80)
    logger.info("")


def pytest_runtest_teardown(item):
    """Hook called after each test ends - add visual separation."""
    logger.info("")
    logger.info("-" * 80)
    logger.info("")


def pytest_runtest_makereport(item, call):
    """Hook to capture test failures"""
    if call.when == "call" and call.excinfo is not None:
        _test_failures.append({
            'name': item.nodeid,
            'error': str(call.excinfo.value),
        })


def pytest_sessionfinish(session, exitstatus):
    """Hook to report all failures at the end of test session and optionally upload to Benchstore"""
    global _current_run_dir
    
    if _test_failures:
        logger.error("\n" + "=" * 80)
        logger.error(f"❌ TEST FAILURES SUMMARY ({len(_test_failures)} failed)")
        logger.error("=" * 80)
        for idx, failure in enumerate(_test_failures, 1):
            logger.error(f"\n{idx}. {failure['name']}")
            logger.error(f"   Error: {failure['error']}")
        logger.error("\n" + "=" * 80)
    else:
        logger.info("\n" + "=" * 80)
        logger.info("✓ TESTS COMPLETED")
        logger.info("=" * 80)
    
    if _current_run_dir:
        logger.info(f"\nResults saved to: {_current_run_dir}")
    
    upload_to_benchstore = session.config.getoption("--upload-to-benchstore")
    local_benchstore_upload = session.config.getoption("--local-benchstore-upload")
    parameters_json_path = session.config.getoption("--parameters-json")
    
    if upload_to_benchstore:
        logger.info("")
        logger.info("=" * 80)
        logger.info(">>> BENCHSTORE UPLOAD")
        logger.info("=" * 80)
        logger.info("")
        
        if not _current_run_dir:
            logger.error("❌ No run directory found - cannot upload results")
            return
        
        # Temporarily raise log level to WARNING for all handlers during benchstore upload
        # This suppresses INFO logs from benchstore/snowflake libraries while keeping ERROR/CRITICAL
        root_logger = logging.getLogger()
        saved_levels = {}
        for handler in root_logger.handlers:
            saved_levels[handler] = handler.level
            handler.setLevel(logging.WARNING)
        
        try:
            from runner.benchstore_upload import upload_metrics
            
            logger.info(f"Uploading results from: {_current_run_dir}")
            upload_metrics(
                results_dir=_current_run_dir,
                use_local_auth=local_benchstore_upload,
                parameters_json_path=parameters_json_path
            )
            
        except Exception as e:
            logger.error("\n" + "=" * 80)
            logger.error(f"❌ Benchstore upload failed: {e}")
            logger.error("=" * 80)
            # Fail the test run if upload fails
            raise
        finally:
            # Restore original handler log levels
            for handler, level in saved_levels.items():
                handler.setLevel(level)
    else:
        logger.info("\nSkipping Benchstore upload (use --upload-to-benchstore to enable)")
