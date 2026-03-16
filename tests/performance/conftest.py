"""pytest configuration for performance tests"""
import json
import logging
import os
from datetime import datetime
from pathlib import Path
from runner.test_types import TestType
from runner.utils import perf_tests_root, collect_node_info, log_node_info
import pytest

logger = logging.getLogger(__name__)

# Track test failures across the session
_test_failures = []

# Track current run directory (session-scoped)
_current_run_dir = None


def pytest_configure():
    """Configure pytest and suppress verbose library logs"""
    # Suppress testcontainers INFO logs (image pulling, container started, etc.)
    logging.getLogger("testcontainers").setLevel(logging.WARNING)


def pytest_addoption(parser):
    """Add custom command line options"""
    parser.addoption(
        "--cloud",
        action="store",
        default=None,
        help="Cloud provider: aws, azure, or gcp. If specified, will use parameters/parameters_perf_{cloud}.json",
    )
    parser.addoption(
        "--parameters-json",
        action="store",
        default=None,
        help="Path to parameters.json file. If not specified and --cloud is provided, uses parameters/parameters_perf_{cloud}.json. Default: parameters/parameters_perf_aws.json",
    )
    parser.addoption(
        "--iterations",
        action="store",
        default=None,
        type=int,
        help="Number of test iterations (default: 5, or per-test marker)",
    )
    parser.addoption(
        "--warmup-iterations",
        action="store",
        default=None,
        type=int,
        help="Number of warmup iterations (default: 0, or per-test marker)",
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
    parser.addoption(
        "--preserve-mappings",
        action="store_true",
        default=False,
        help="Preserve WireMock mapping directories after tests (useful for debugging). Default: delete after completion",
    )
    parser.addoption(
        "--reuse-mappings",
        action="store",
        default=None,
        help="Reuse existing WireMock mappings directory (e.g., 'run_20251230_155413'). Skips recording phase.",
    )


def _resolve_parameters_path(config) -> str:
    """
    Resolve parameters JSON path from config and environment.
    
    Priority:
    1. --parameters-json flag (explicit path)
    2. --cloud flag (uses parameters/parameters_perf_{cloud}.json)
    3. CLOUD environment variable (uses parameters/parameters_perf_{cloud}.json)
    4. Default: parameters/parameters_perf_aws.json
    
    Args:
        config: pytest config object
        
    Returns:
        Path to parameters JSON file
    """
    # Check for explicit parameters file path
    params_path = config.getoption("--parameters-json")
    if params_path:
        return params_path
    
    # Check for cloud parameter (CLI or environment)
    cloud = config.getoption("--cloud") or os.getenv("CLOUD")
    if cloud:
        cloud = cloud.lower()
        return str(Path("parameters") / f"parameters_perf_{cloud}.json")
    
    # Default to AWS
    return str(Path("parameters") / "parameters_perf_aws.json")


@pytest.fixture
def parameters_json_path(request):
    """Get parameters JSON path from command line, cloud option, or environment."""
    return _resolve_parameters_path(request.config)


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
    4. Default value (5) - lowest priority
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
    return 5


@pytest.fixture
def warmup_iterations(request):
    """
    Get number of warmup iterations with precedence:
    1. Command line args (--warmup-iterations=N) - highest priority
    2. Test-level marker (@pytest.mark.warmup_iterations(N))
    3. Environment variable (PERF_WARMUP_ITERATIONS)
    4. Default value (0) - lowest priority
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
    return 0


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
    
    log_node_info(collect_node_info())
    
    return run_dir


@pytest.fixture
def reuse_mappings_dir(request):
    """Get reuse mappings directory from command line"""
    return request.config.getoption("--reuse-mappings")


@pytest.fixture
def results_dir(session_results_dir):
    """Return the session-specific results directory"""
    return session_results_dir


@pytest.fixture
def use_local_binary(request):
    """Get use-local-binary flag from command line"""
    return request.config.getoption("--use-local-binary")


@pytest.fixture
def preserve_mappings(request):
    """Get preserve-mappings flag from command line"""
    return request.config.getoption("--preserve-mappings")


def _derive_test_name(func_name: str) -> str:
    """Derive test name from function name by stripping 'test_' prefix."""
    if func_name.startswith("test_"):
        return func_name[5:]
    return func_name


def _normalize_driver_type(driver: str, driver_type: str) -> str:
    """Normalize driver type (Core only has universal implementation)."""
    return None if driver == "core" else driver_type


def _should_run_comparison(driver: str, driver_type: str) -> bool:
    """Check if test should run as comparison (both driver types)."""
    return driver_type == "both" and driver != "core"


def _validate_wiremock_old_driver(driver: str, driver_type: str):
    """Validate that old driver is not used alone with WireMock tests."""
    if driver_type == "old" and driver != "core":
        raise pytest.UsageError(
            f"WireMock tests cannot run with --driver-type=old only.\n"
            f"The old {driver} driver requires mappings from the universal driver.\n"
            f"Use --driver-type=universal or --driver-type=both instead."
        )


def _prepare_setup_queries(test_type: TestType, parameters_json: str, setup_queries: list[str] = None) -> list[str]:
    """
    Prepare setup queries based on test type.
    
    Args:
        test_type: Type of test (SELECT, PUT_GET, or SELECT_RECORDED_HTTP)
        parameters_json: JSON string with connection parameters
        setup_queries: Optional user-provided setup queries
    
    Returns:
        List of setup queries with test-type-specific prefixes
    """
    match test_type:
        case TestType.SELECT | TestType.SELECT_RECORDED_HTTP:
            # SELECT tests: always use ARROW format
            arrow_query = "alter session set query_result_format = 'ARROW'"
            return [arrow_query] + (setup_queries or [])
        
        case TestType.PUT_GET:
            # PUT/GET tests: USE DATABASE is required for TEMPORARY STAGE
            params = json.loads(parameters_json)
            testconn = params.get("testconnection", {})
            database = testconn.get("SNOWFLAKE_TEST_DATABASE") or testconn.get("database", "")
            
            use_db_query = f"USE DATABASE {database}"
            return [use_db_query] + (setup_queries or [])


@pytest.fixture
def perf_test(parameters_json, results_dir, run_id, iterations, warmup_iterations, driver, driver_type, use_local_binary, preserve_mappings, reuse_mappings_dir, request):
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
    from runner.modes.e2e_runner import run_performance_test, run_comparison_test
    
    # Validate: local binary only works with Core
    if use_local_binary and driver != "core":
        raise pytest.UsageError(
            f"--use-local-binary is only supported for Core driver. Got: --driver={driver}"
        )
    
    def _run_test(
        sql_command: str, 
        setup_queries: list[str] = None,
        test_name: str = None,
        test_type: TestType = TestType.SELECT,
        s3_download_url: str = None,  # S3 URL for PUT/GET tests
        s3_download_dir: str = None  # Local directory for downloaded files
    ):
        # Prepare test parameters
        if test_name is None:
            test_name = _derive_test_name(request.node.name)
        
        final_setup_queries = _prepare_setup_queries(test_type, parameters_json, setup_queries)
        s3_files_dir = _download_s3_files_if_needed(s3_download_url, s3_download_dir)
        is_comparison = _should_run_comparison(driver, driver_type)
        
        # Route to appropriate runner based on test type
        if test_type == TestType.SELECT_RECORDED_HTTP:
            return _run_wiremock_test(
                test_name=test_name,
                sql_command=sql_command,
                setup_queries=final_setup_queries,
                s3_files_dir=s3_files_dir,
                is_comparison=is_comparison,
            )
        else:
            return _run_e2e_test(
                test_name=test_name,
                sql_command=sql_command,
                setup_queries=final_setup_queries,
                test_type=test_type,
                s3_files_dir=s3_files_dir,
                is_comparison=is_comparison,
            )
    
    def _run_wiremock_test(
        test_name: str,
        sql_command: str,
        setup_queries: list[str],
        s3_files_dir,
        is_comparison: bool,
    ):
        """Run WireMock test (recorded HTTP traffic)."""
        _validate_wiremock_old_driver(driver, driver_type)
        
        if is_comparison:
            from runner.modes.wiremock_runner import run_wiremock_comparison_test
            return run_wiremock_comparison_test(
                test_name=test_name,
                sql_command=sql_command,
                setup_queries=setup_queries,
                parameters_json=parameters_json,
                results_dir=results_dir,
                iterations=iterations,
                warmup_iterations=warmup_iterations,
                driver=driver,
                use_local_binary=use_local_binary,
                s3_files_dir=s3_files_dir,
                run_id=run_id,
                preserve_mappings=preserve_mappings,
                reuse_mappings_dir=reuse_mappings_dir,
            )
        else:
            from runner.modes.wiremock_runner import run_wiremock_performance_test
            return run_wiremock_performance_test(
                test_name=test_name,
                sql_command=sql_command,
                setup_queries=setup_queries,
                parameters_json=parameters_json,
                results_dir=results_dir,
                iterations=iterations,
                warmup_iterations=warmup_iterations,
                driver=driver,
                driver_type=_normalize_driver_type(driver, driver_type),
                use_local_binary=use_local_binary,
                s3_files_dir=s3_files_dir,
                run_id=run_id,
                preserve_mappings=preserve_mappings,
                reuse_mappings_dir=reuse_mappings_dir,
            )
    
    def _run_e2e_test(
        test_name: str,
        sql_command: str,
        setup_queries: list[str],
        test_type: TestType,
        s3_files_dir,
        is_comparison: bool,
    ):
        """Run E2E test (real Snowflake connection)."""
        if is_comparison:
            return run_comparison_test(
                test_name=test_name,
                sql_command=sql_command,
                setup_queries=setup_queries,
                test_type=test_type,
                parameters_json=parameters_json,
                results_dir=results_dir,
                iterations=iterations,
                warmup_iterations=warmup_iterations,
                driver=driver,
                s3_files_dir=s3_files_dir,
            )
        else:
            return run_performance_test(
                test_name=test_name,
                sql_command=sql_command,
                setup_queries=setup_queries,
                test_type=test_type,
                parameters_json=parameters_json,
                results_dir=results_dir,
                iterations=iterations,
                warmup_iterations=warmup_iterations,
                driver=driver,
                driver_type=_normalize_driver_type(driver, driver_type),
                use_local_binary=use_local_binary,
                s3_files_dir=s3_files_dir,
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
    parameters_json_path = _resolve_parameters_path(session.config)
    
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


def _download_s3_files_if_needed(s3_download_url: str = None, s3_download_dir: str = None):
    """
    Download S3 files if needed (for PUT/GET tests).
    
    Args:
        s3_download_url: S3 URL to download files from
        s3_download_dir: Local directory to download files to (optional)
    
    Returns:
        Path to downloaded files directory, or None if no download needed
    """
    if not s3_download_url:
        return None
    
    from runner.s3_utils import download_s3_files
    
    # Default download dir: tests/performance/put_get_files/{dataset_name_from_s3_url}
    if s3_download_dir is None:
        s3_files_base = perf_tests_root() / "put_get_files"
        # Extract dataset name from S3 URL
        # e.g., "s3://bucket/path/12Mx100/" -> "12Mx100"
        dataset_name = s3_download_url.rstrip('/').split('/')[-1]
        s3_download_dir = str(s3_files_base / dataset_name)
    
    s3_files_dir = Path(s3_download_dir)
    
    try:
        download_s3_files(s3_download_url, s3_files_dir)
    except Exception as e:
        pytest.fail(f"S3 download failed: {e}")
    
    return s3_files_dir
