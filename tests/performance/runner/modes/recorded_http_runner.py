import csv
import logging
import shutil
import statistics
from pathlib import Path

from runner.docker_network import DockerNetworkManager
from runner.modes.common import execute_test, verify_test_results
from runner.test_types import TestType
from runner.utils import perf_tests_root

logger = logging.getLogger(__name__)

MAPPINGS_BASE_DIR = perf_tests_root() / "mappings"


def run_recorded_http_performance_test(
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
    s3_files_dir: Path = None,
    run_id: str = None,
    preserve_mappings: bool = False,
    reuse_mappings_dir: str = None,
    expected_row_count_override: int = None,
) -> list[Path]:
    """
    Run a performance test with recorded HTTP traffic.

    Workflow (if reuse_mappings_dir not provided):
    1. Start proxy in record mode to capture HTTP traffic
    2. Execute test once to record traffic
    3. Stop recorder and transform mappings
    4. Start proxy in replay mode
    5. Execute test N times against recorded responses
    6. Stop replay server

    Workflow (if reuse_mappings_dir provided):
    1. Start proxy in replay mode with existing mappings
    2. Execute test N times against recorded responses
    3. Stop replay server

    Args:
        test_name: Name of the test (used for result filenames)
        sql_command: SQL command to execute
        parameters_json: JSON string with connection parameters
        results_dir: Directory to store results
        iterations: Number of test iterations (for replay phase)
        warmup_iterations: Number of warmup iterations (for replay phase)
        driver: Driver to use (core, python, odbc, jdbc)
        driver_type: Driver type: 'universal' or 'old' (only 'universal' for core)
        setup_queries: Optional list of SQL queries to run before warmup/test iterations
        use_local_binary: Use locally built binary instead of Docker (Core only)
        s3_files_dir: Optional directory with S3-downloaded files to mount (for PUT/GET tests)
        run_id: Optional run ID for organizing results
        preserve_mappings: Keep mappings after test (default: delete)
        reuse_mappings_dir: Optional path to existing mappings directory (e.g., "run_20251230_155413")
                           If provided, skips recording phase and uses existing mappings
        expected_row_count_override: Optional row count for validation (used in comparison mode)
                                     When old driver reuses universal's mappings, this is set to universal's row count

    Returns:
        List of result file paths created
    """
    if use_local_binary:
        raise ValueError(
            "Local binary execution (--use-local-binary) is not supported with recorded HTTP tests. "
            "These tests require Docker container networking to intercept HTTP traffic."
        )

    mappings_dir, skip_recording = _get_mappings_dir(test_name, results_dir, run_id, reuse_mappings_dir)

    if skip_recording:
        _log_banner(f"REPLAY MODE (REUSING MAPPINGS: {reuse_mappings_dir})")
    else:
        _log_banner("RECORDING MODE")

    network_manager = DockerNetworkManager()
    network = network_manager.create_network()

    try:
        from replay_server.manager import ProxyServerManager

        # ── Recording phase (skip if reusing existing mappings) ──────
        if not skip_recording:
            logger.info("")
            logger.info("Step 1: Starting recorder...")
            recorder = ProxyServerManager(mappings_dir, network_mode=network)
            try:
                recorder.start_recording()

                logger.info("")
                logger.info("Step 2: Recording HTTP traffic...")
                _run_test_with_proxy(
                    test_name=f"{test_name}_record",
                    sql_command=sql_command,
                    parameters_json=parameters_json,
                    results_dir=results_dir,
                    iterations=1,
                    warmup_iterations=0,
                    driver=driver,
                    driver_type=driver_type,
                    setup_queries=setup_queries,
                    use_local_binary=use_local_binary,
                    s3_files_dir=s3_files_dir,
                    proxy_url=recorder.get_url(),
                    network_mode=network,
                    proxy_manager=recorder,
                )

                logger.info("")
                logger.info("Step 3: Creating snapshot and transforming mappings...")

            finally:
                logger.info("")
                logger.info("Step 4: Stopping recorder...")
                recorder.stop()

            recorder.create_snapshot()

            expected_row_count = _extract_row_count_from_recording(results_dir, test_name, driver, driver_type)
            if expected_row_count:
                logger.info(f"Extracted row count from recording: {expected_row_count} rows")
        else:
            logger.info("")
            logger.info("Skipping recording phase - reusing existing mappings")
            logger.info(f"Mappings directory: {mappings_dir}")
            expected_row_count = expected_row_count_override
            if expected_row_count:
                logger.info(f"Using expected row count from override: {expected_row_count} rows")

        # ── Replay phase ─────────────────────────────────────────────
        replay_step = 2 if skip_recording else 5
        if not skip_recording:
            _log_banner("REPLAY MODE")
        else:
            logger.info("")
        logger.info(f"Step {replay_step}: Starting replay server...")

        replay_mgr = ProxyServerManager(mappings_dir, network_mode=network)

        try:
            replay_mgr.start_replay(driver_label=driver_type)

            logger.info("")
            logger.info(f"Step {replay_step + 1}: Running {iterations} iterations with recorded responses...")
            _run_test_with_proxy(
                test_name=test_name,
                sql_command=sql_command,
                parameters_json=parameters_json,
                results_dir=results_dir,
                iterations=iterations,
                warmup_iterations=warmup_iterations,
                driver=driver,
                driver_type=driver_type,
                setup_queries=None,
                use_local_binary=use_local_binary,
                s3_files_dir=s3_files_dir,
                proxy_url=replay_mgr.get_url(),
                network_mode=network,
                is_replay=True,
                expected_row_count=expected_row_count,
                proxy_manager=replay_mgr,
            )

            logger.info("")
            logger.info("Collecting response time metrics...")
            metrics = replay_mgr.get_request_metrics()
            _log_replay_metrics(metrics, warmup_iterations=warmup_iterations, iterations=iterations)
            _check_unmatched_requests(metrics)

        finally:
            cleanup_step = 4 if skip_recording else 7
            logger.info("")
            logger.info(f"Step {cleanup_step}: Cleanup...")
            replay_mgr.stop()

            if skip_recording:
                logger.info(f"✓ Reused mappings from: {mappings_dir}")
            elif preserve_mappings:
                logger.info(f"✓ Mappings preserved at: {mappings_dir}")
            else:
                if mappings_dir.exists():
                    logger.info(f"Removing mappings directory: {mappings_dir}")
                    shutil.rmtree(mappings_dir)
                    logger.info("✓ Mappings removed")
                else:
                    logger.debug(f"Mappings directory does not exist: {mappings_dir}")
    finally:
        network_manager.remove_network()

    _log_banner("✓ RECORDED HTTP TEST COMPLETE")

    return verify_test_results(
        results_dir,
        test_name,
        driver,
        iterations,
        driver_type=driver_type,
    )


def run_recorded_http_comparison_test(
    test_name: str,
    sql_command: str,
    parameters_json: str,
    results_dir: Path,
    iterations: int,
    warmup_iterations: int,
    driver: str,
    setup_queries: list[str] = None,
    use_local_binary: bool = False,
    s3_files_dir: Path = None,
    run_id: str = None,
    preserve_mappings: bool = False,
    reuse_mappings_dir: str = None,
) -> dict[str, list[Path]]:
    """
    Run recorded HTTP test on both universal and old driver implementations.

    Args:
        test_name: Name of the test (used for result filenames)
        sql_command: SQL command to execute
        parameters_json: JSON string with connection parameters
        results_dir: Directory to store results
        iterations: Number of test iterations (for replay phase)
        warmup_iterations: Number of warmup iterations (for replay phase)
        driver: Driver to test (python, odbc, jdbc)
        setup_queries: Optional list of SQL queries to run before warmup/test iterations
        use_local_binary: Use locally built binary instead of Docker (Core only)
        s3_files_dir: Optional directory with S3-downloaded files to mount (for PUT/GET tests)
        run_id: Optional run ID for organizing results
        preserve_mappings: Keep mappings after test (default: delete)
        reuse_mappings_dir: Optional path to existing mappings directory

    Returns:
        Dict with 'universal' and 'old' keys, each containing list of result file paths
    """
    if use_local_binary:
        raise ValueError(
            "Local binary execution (--use-local-binary) is not supported with recorded HTTP tests. "
            "These tests require Docker container networking to intercept HTTP traffic."
        )

    logger.info(f"Running {test_name} comparison ({driver.upper()}): Universal vs Old")

    results = {}

    _log_banner(">>> DRIVER: Universal (Recording)")
    results['universal'] = run_recorded_http_performance_test(
        test_name=test_name,
        sql_command=sql_command,
        parameters_json=parameters_json,
        results_dir=results_dir,
        iterations=iterations,
        warmup_iterations=warmup_iterations,
        driver=driver,
        driver_type="universal",
        setup_queries=setup_queries,
        use_local_binary=use_local_binary,
        s3_files_dir=s3_files_dir,
        run_id=run_id,
        preserve_mappings=True,
        reuse_mappings_dir=reuse_mappings_dir,
    )

    if reuse_mappings_dir:
        old_driver_mappings = reuse_mappings_dir
    else:
        actual_run_id = _extract_run_id(results_dir, run_id)
        old_driver_mappings = f"run_{actual_run_id}"

    expected_row_count_for_old = _extract_row_count_from_recording(results_dir, test_name, driver, "universal")
    if expected_row_count_for_old:
        logger.info(f"Universal driver recorded {expected_row_count_for_old} rows - will validate old driver fetches the same")

    _log_banner(f">>> DRIVER: Old (Reusing mappings from: {old_driver_mappings})")
    results['old'] = run_recorded_http_performance_test(
        test_name=test_name,
        sql_command=sql_command,
        parameters_json=parameters_json,
        results_dir=results_dir,
        iterations=iterations,
        warmup_iterations=warmup_iterations,
        driver=driver,
        driver_type="old",
        setup_queries=setup_queries,
        use_local_binary=use_local_binary,
        s3_files_dir=s3_files_dir,
        run_id=run_id,
        preserve_mappings=preserve_mappings,
        reuse_mappings_dir=old_driver_mappings,
        expected_row_count_override=expected_row_count_for_old,
    )

    return results


def _extract_run_id(results_dir: Path, run_id: str = None) -> str:
    """Extract run_id from results_dir if not provided."""
    if run_id is not None:
        return run_id
    return results_dir.name.replace("run_", "")


def _get_mappings_dir(test_name: str, results_dir: Path, run_id: str = None, reuse_mappings_dir: str = None) -> tuple[Path, bool]:
    """
    Determine mappings directory and whether to skip recording.

    Returns:
        Tuple of (mappings_dir, skip_recording)
    """
    if reuse_mappings_dir:
        mappings_dir = (MAPPINGS_BASE_DIR / reuse_mappings_dir / test_name).resolve()
        if not mappings_dir.exists():
            raise RuntimeError(
                f"Reuse mappings directory not found: {mappings_dir}\n"
                f"Available runs: {list(MAPPINGS_BASE_DIR.glob('run_*'))}"
            )
        return mappings_dir, True
    else:
        actual_run_id = _extract_run_id(results_dir, run_id)
        mappings_dir = (MAPPINGS_BASE_DIR / f"run_{actual_run_id}" / test_name).resolve()
        return mappings_dir, False


def _log_banner(message: str, separator: str = "=" * 80):
    """Log a banner message with separator lines."""
    logger.info("")
    logger.info(separator)
    logger.info(message)
    logger.info(separator)
    logger.info("")


def _log_replay_metrics(metrics: dict, warmup_iterations: int = 0, iterations: int = 0):
    """Log replay server response time metrics in a formatted display."""
    logger.info("")
    logger.info("=" * 80)
    logger.info("REPLAY SERVER RESPONSE TIME METRICS")
    logger.info("=" * 80)

    total_requests = metrics.get("total_requests", 0)

    if total_requests == 0:
        logger.info("No requests recorded")
        logger.info("=" * 80)
        logger.info("")
        return

    all_times = metrics.get("response_times", [])
    times = all_times

    if all_times and warmup_iterations > 0 and iterations > 0:
        total_iterations = warmup_iterations + iterations
        requests_per_iteration = total_requests / total_iterations
        warmup_requests = int(requests_per_iteration * warmup_iterations)

        if warmup_requests < len(all_times):
            times = all_times[warmup_requests:]
            logger.info(f"Filtered out {warmup_requests} warmup requests ({warmup_iterations} iterations)")

    if times:
        sorted_times = sorted(times)
        n = len(sorted_times)
        avg_time = statistics.mean(times)
        min_time = sorted_times[0]
        max_time = sorted_times[-1]
        p50_time = sorted_times[int(0.50 * n)]
        p95_time = sorted_times[int(0.95 * n)]
        p99_time = sorted_times[min(int(0.99 * n), n - 1)]
    else:
        avg_time = min_time = max_time = p50_time = p95_time = p99_time = 0

    logger.info(f"Total Requests:        {len(times):,}")
    logger.info(f"Average Response Time: {avg_time:.2f} ms")
    logger.info(f"Min Response Time:     {min_time:.2f} ms")
    logger.info(f"Max Response Time:     {max_time:.2f} ms")
    logger.info(f"P50 Response Time:     {p50_time:.2f} ms")
    logger.info(f"P95 Response Time:     {p95_time:.2f} ms")
    logger.info(f"P99 Response Time:     {p99_time:.2f} ms")

    logger.info("=" * 80)
    logger.info("")


def _check_unmatched_requests(metrics: dict):
    """Fail the test if the replay proxy returned 404 for any request."""
    unmatched = metrics.get("unmatched_requests", 0)
    if unmatched > 0:
        details = metrics.get("unmatched_details", [])
        detail_lines = "\n  ".join(details) if details else "(details unavailable)"
        raise RuntimeError(
            f"Replay server had {unmatched} unmatched request(s) (returned 404):\n  {detail_lines}"
        )


def _get_proxy_url_for_container(proxy_url: str, network_mode: str = None) -> str:
    """
    Get Docker-accessible proxy URL.

    Network modes:
    - "host" (Linux): All containers share host network -> use localhost
    - None (macOS default bridge): Containers use host.docker.internal to reach host
    """
    if network_mode == "host":
        return proxy_url
    else:
        port = proxy_url.split(":")[-1]
        return f"http://host.docker.internal:{port}"


def _extract_row_count_from_recording(results_dir: Path, test_name: str, driver: str, driver_type: str) -> int:
    """Extract row count from recording phase CSV file."""
    if driver == "core":
        pattern = f"{test_name}_record_{driver}_*.csv"
    else:
        pattern = f"{test_name}_record_{driver}_{driver_type}_*.csv"

    csv_files = list(results_dir.glob(pattern))

    if not csv_files:
        logger.warning(f"No recording CSV found matching pattern: {pattern}")
        return None

    csv_file = csv_files[0]

    try:
        with open(csv_file, 'r') as f:
            reader = csv.DictReader(f)
            rows = list(reader)
            if rows and 'row_count' in rows[0]:
                return int(rows[0]['row_count'])
            else:
                logger.warning(f"No row_count column found in {csv_file}")
                return None
    except Exception as e:
        logger.warning(f"Failed to extract row count from {csv_file}: {e}")
        return None


def _run_test_with_proxy(
    test_name: str,
    sql_command: str,
    parameters_json: str,
    results_dir: Path,
    iterations: int,
    warmup_iterations: int,
    proxy_url: str,
    network_mode: str,
    driver: str = "core",
    driver_type: str = None,
    setup_queries: list[str] = None,
    use_local_binary: bool = False,
    s3_files_dir: Path = None,
    is_replay: bool = False,
    expected_row_count: int = None,
    proxy_manager=None,
):
    """
    Run test with proxy configuration.

    Sets up proxy environment variables and exports the CA certificate
    so drivers trust the dynamically generated MITM certificates.
    """
    container_proxy_url = _get_proxy_url_for_container(proxy_url, network_mode)
    env_vars = {
        "HTTPS_PROXY": container_proxy_url,
        "HTTP_PROXY": container_proxy_url,
        "https_proxy": container_proxy_url,
        "http_proxy": container_proxy_url,
    }

    if is_replay:
        env_vars["REPLAY_MODE"] = "true"
        if expected_row_count is not None:
            logger.info(f"Setting EXPECTED_ROW_COUNT={expected_row_count} for replay validation")
            env_vars["EXPECTED_ROW_COUNT"] = str(expected_row_count)

    if proxy_manager:
        try:
            ca_cert_path = proxy_manager.export_ca_cert(results_dir)
            env_vars["PROXY_CA_CERT"] = "/results/" + ca_cert_path.name
            if driver == "odbc" and driver_type == "old":
                env_vars["PROXY_URL"] = container_proxy_url
        except Exception:
            logger.error(
                "Failed to export CA cert; skipping this test execution.",
                exc_info=True,
            )
            return

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
        test_type=TestType.SELECT,
        use_local_binary=use_local_binary,
        s3_files_dir=s3_files_dir,
        env_vars=env_vars,
        network_mode=network_mode,
    )
