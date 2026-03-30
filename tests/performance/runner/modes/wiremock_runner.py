import csv
import logging
import shutil
import statistics
import time
from pathlib import Path

from runner.docker_network import DockerNetworkManager
from runner.modes.common import execute_test, verify_test_results
from runner.test_types import PerfTestType
from runner.utils import perf_tests_root
from wiremock.wiremock_manager import WiremockManager
from wiremock.wiremock_monitor import WiremockMonitor, MonitorResult

logger = logging.getLogger(__name__)

MAPPINGS_BASE_DIR = perf_tests_root() / "mappings"


def run_wiremock_performance_test(
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
    Run a performance test with WireMock HTTP traffic recording.
    
    Workflow (if reuse_mappings_dir not provided):
    1. Start WireMock in record mode
    2. Execute test once to record HTTP traffic
    3. Create snapshot and transform mappings
    4. Stop WireMock
    5. Start WireMock in replay mode
    6. Execute test N times against recorded responses
    7. Stop WireMock
    
    Workflow (if reuse_mappings_dir provided):
    1. Start WireMock in replay mode with existing mappings
    2. Execute test N times against recorded responses
    3. Stop WireMock
    
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
    # Validate: local binary execution is not supported with WireMock
    if use_local_binary:
        raise ValueError(
            "Local binary execution (--use-local-binary) is not supported with WireMock tests. "
            "WireMock requires Docker container networking to properly intercept HTTP traffic."
        )
    
    # Determine mappings directory and mode
    mappings_dir, skip_recording = _get_mappings_dir(test_name, results_dir, run_id, reuse_mappings_dir)
    
    if skip_recording:
        _log_banner(f"WIREMOCK REPLAY MODE (REUSING MAPPINGS: {reuse_mappings_dir})")
    else:
        _log_banner("WIREMOCK RECORDING MODE")
    
    # Create Docker network for container communication
    network_manager = DockerNetworkManager()
    network = network_manager.create_network()
    
    try:
        # Recording phase (skip if reusing existing mappings)
        if not skip_recording:
            # Step 1: Start WireMock in record mode
            logger.info("")
            logger.info("Step 1: Starting WireMock in RECORD mode...")
            wiremock = WiremockManager(mappings_dir, network_mode=network)
            wm_monitor = None
            try:
                wiremock.start_recording()

                docker_container = wiremock.get_docker_container()
                if docker_container:
                    wm_monitor = WiremockMonitor(docker_container)
                    wm_monitor.start()

                # Step 3: Execute test once to record traffic
                logger.info("")
                logger.info("Step 3: Recording HTTP traffic...")
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
                    wiremock_url=wiremock.get_url(),
                    wiremock_container_name=wiremock.get_container_name(),
                    network_mode=network,
                    wiremock_manager=wiremock,
                )
                
                # Step 4: Create snapshot and transform
                logger.info("")
                logger.info("Step 4: Creating snapshot and transforming mappings...")
                wiremock.create_snapshot()
                
            finally:
                if wm_monitor:
                    monitor_result = wm_monitor.stop()
                    _write_wiremock_monitor_results(
                        monitor_result, results_dir, test_name, "record", driver_type,
                    )
                # Step 5: Stop recording WireMock
                logger.info("")
                logger.info("Step 5: Stopping WireMock (record mode)...")
                wiremock.stop()
            
            expected_row_count = _extract_row_count_from_recording(results_dir, test_name, driver, driver_type)
            if expected_row_count is None or expected_row_count == 0:
                raise RuntimeError(
                    f"Recording phase failed for '{test_name}': no valid result CSV was produced. "
                    f"The driver container likely crashed during query execution through "
                    f"the WireMock proxy. Check the container logs above for details."
                )
            logger.info(f"Extracted row count from recording: {expected_row_count} rows")
        else:
            logger.info("")
            logger.info("Skipping recording phase - reusing existing mappings")
            logger.info(f"Mappings directory: {mappings_dir}")
            # Use override if provided (e.g., in comparison mode where old driver reuses universal's mappings)
            expected_row_count = expected_row_count_override
            if expected_row_count:
                logger.info(f"Using expected row count from override: {expected_row_count} rows")
        
        # Start WireMock in replay mode
        replay_step = 2 if skip_recording else 6
        if not skip_recording:
            _log_banner("WIREMOCK REPLAY MODE")
        else:
            logger.info("")
        logger.info(f"Step {replay_step}: Starting WireMock in REPLAY mode...")
        wiremock = WiremockManager(mappings_dir, network_mode=network)
        wm_monitor = None
        
        try:
            wiremock.start_replay(driver_label=driver_type)

            docker_container = wiremock.get_docker_container()
            if docker_container:
                wm_monitor = WiremockMonitor(docker_container)
                wm_monitor.start()
            
            # Execute test N times with cached responses
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
                setup_queries=None,  # Skip setup queries in replay mode - they were only recorded once
                use_local_binary=use_local_binary,
                s3_files_dir=s3_files_dir,
                wiremock_url=wiremock.get_url(),
                wiremock_container_name=wiremock.get_container_name(),
                network_mode=network,
                is_replay=True,  # Flag to indicate replay mode
                expected_row_count=expected_row_count,  # Pass expected row count for validation
                wiremock_manager=wiremock,
            )
            
            # Collect metrics while WireMock is still running (triggers flush to disk)
            logger.info("")
            logger.info("Collecting response time metrics...")
            metrics = wiremock.get_request_metrics()
            _log_wiremock_metrics(metrics, warmup_iterations=warmup_iterations, iterations=iterations)
            
        finally:
            if wm_monitor:
                monitor_result = wm_monitor.stop()
                _write_wiremock_monitor_results(
                    monitor_result, results_dir, test_name, "replay", driver_type,
                )
            cleanup_step = 4 if skip_recording else 8
            logger.info("")
            logger.info(f"Step {cleanup_step}: Cleanup...")
            wiremock.stop()
            
            # Never delete mappings when reusing (they belong to another run)
            if skip_recording:
                logger.info(f"✓ Reused mappings from: {mappings_dir}")
            elif preserve_mappings:
                logger.info(f"✓ Mappings preserved at: {mappings_dir}")
            else:
                # Delete mappings by default
                if mappings_dir.exists():
                    logger.info(f"Removing mappings directory: {mappings_dir}")
                    shutil.rmtree(mappings_dir)
                    logger.info("✓ Mappings removed")
                else:
                    logger.debug(f"Mappings directory does not exist: {mappings_dir}")
    finally:
        # Clean up Docker network
        network_manager.remove_network()
    
    _log_banner("✓ WIREMOCK TEST COMPLETE")
    
    # Verify and return results
    return verify_test_results(
        results_dir,
        test_name,
        driver,
        iterations,
        driver_type=driver_type,
    )


def run_wiremock_comparison_test(
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
    Run WireMock test on both universal and old driver implementations.
    
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
            "Local binary execution (--use-local-binary) is not supported with WireMock tests. "
            "WireMock requires Docker container networking to properly intercept HTTP traffic."
        )
    
    logger.info(f"Running {test_name} WireMock comparison ({driver.upper()}): Universal vs Old")
    
    results = {}
    
    # Run Universal driver first (records mappings)
    _log_banner(">>> DRIVER: Universal (Recording)")
    results['universal'] = run_wiremock_performance_test(
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
        preserve_mappings=True,  # Must preserve for old driver to reuse
        reuse_mappings_dir=reuse_mappings_dir,
    )
    
    # Determine the mappings directory created by universal driver
    # If user provided reuse_mappings_dir, both drivers use it
    # Otherwise, old driver reuses the mappings just created by universal
    if reuse_mappings_dir:
        old_driver_mappings = reuse_mappings_dir
    else:
        actual_run_id = _extract_run_id(results_dir, run_id)
        old_driver_mappings = f"run_{actual_run_id}"
    
    # Extract expected row count from universal driver's recording for old driver validation
    # The old driver will look for its own recording file, but it doesn't have one
    # So we need to extract from universal's recording and pass it explicitly
    expected_row_count_for_old = _extract_row_count_from_recording(results_dir, test_name, driver, "universal")
    if expected_row_count_for_old:
        logger.info(f"Universal driver recorded {expected_row_count_for_old} rows - will validate old driver fetches the same")
    
    # Run Old driver second (reuses mappings from universal)
    _log_banner(f">>> DRIVER: Old (Reusing mappings from: {old_driver_mappings})")
    results['old'] = run_wiremock_performance_test(
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
        preserve_mappings=preserve_mappings,  # Use user's preference for final cleanup
        reuse_mappings_dir=old_driver_mappings,  # Reuse universal's mappings
        expected_row_count_override=expected_row_count_for_old,  # Pass universal's row count for validation
    )
    
    return results


def _extract_run_id(results_dir: Path, run_id: str = None) -> str:
    """Extract run_id from results_dir if not provided."""
    if run_id is not None:
        return run_id
    # results_dir format: .../results/run_YYYYMMDD_HHMMSS
    return results_dir.name.replace("run_", "")


def _get_mappings_dir(test_name: str, results_dir: Path, run_id: str = None, reuse_mappings_dir: str = None) -> tuple[Path, bool]:
    """
    Determine mappings directory and whether to skip recording.
    
    Returns:
        Tuple of (mappings_dir, skip_recording)
    """
    if reuse_mappings_dir:
        # User provided existing mappings directory to reuse
        mappings_dir = (MAPPINGS_BASE_DIR / reuse_mappings_dir / test_name).resolve()
        if not mappings_dir.exists():
            raise RuntimeError(
                f"Reuse mappings directory not found: {mappings_dir}\n"
                f"Available runs: {list(MAPPINGS_BASE_DIR.glob('run_*'))}"
            )
        return mappings_dir, True
    else:
        # Normal flow: record first, then replay
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


def _compute_percentiles(times: list) -> dict:
    """Compute avg/min/max/P50/P95/P99 from a list of numeric values."""
    if not times:
        return {"avg": 0, "min": 0, "max": 0, "p50": 0, "p95": 0, "p99": 0}
    s = sorted(times)
    n = len(s)
    return {
        "avg": statistics.mean(times),
        "min": min(times),
        "max": max(times),
        "p50": s[int(0.50 * n)],
        "p95": s[int(0.95 * n)],
        "p99": s[min(int(0.99 * n), n - 1)],
    }


def _log_time_section(label: str, p: dict):
    """Log a single metrics section with consistent formatting."""
    logger.info(f"  {label}:")
    logger.info(f"    Average: {p['avg']:>10.2f} ms")
    logger.info(f"    Min:     {p['min']:>10.2f} ms")
    logger.info(f"    P50:     {p['p50']:>10.2f} ms")
    logger.info(f"    P95:     {p['p95']:>10.2f} ms")
    logger.info(f"    P99:     {p['p99']:>10.2f} ms")
    logger.info(f"    Max:     {p['max']:>10.2f} ms")


def _filter_warmup(all_times: list, total_requests: int,
                   warmup_iterations: int, iterations: int) -> list:
    """Strip warmup-iteration samples from the front of a time series."""
    if not all_times or warmup_iterations <= 0 or iterations <= 0:
        return list(all_times)
    total_iterations = warmup_iterations + iterations
    requests_per_iteration = total_requests / total_iterations
    warmup_requests = int(requests_per_iteration * warmup_iterations)
    if warmup_requests < len(all_times):
        logger.info(f"Filtered out {warmup_requests} warmup requests "
                     f"({warmup_iterations} iterations)")
        return all_times[warmup_requests:]
    return list(all_times)


def _log_wiremock_metrics(metrics: dict, warmup_iterations: int = 0, iterations: int = 0):
    """
    Log WireMock response time metrics split into serve (stub matching) and
    send (socket write / TCP backpressure) phases.
    """
    logger.info("")
    logger.info("=" * 80)
    logger.info("WIREMOCK RESPONSE TIME METRICS")
    logger.info("=" * 80)

    total_requests = metrics.get("total_requests", 0)
    if total_requests == 0:
        logger.info("No requests recorded")
        logger.info("=" * 80)
        logger.info("")
        return

    serve_all = metrics.get("serve_times", [])
    send_all = metrics.get("send_times", [])

    serve_times = _filter_warmup(serve_all, total_requests, warmup_iterations, iterations)
    send_times = _filter_warmup(send_all, total_requests, warmup_iterations, iterations)

    logger.info(f"Total Requests: {len(serve_times):,}")
    logger.info("")

    _log_time_section("Serve (stub matching / processing)", _compute_percentiles(serve_times))
    logger.info("")
    _log_time_section("Send  (socket write to client)", _compute_percentiles(send_times))

    if serve_times and send_times:
        total_times = [s + w for s, w in zip(serve_times, send_times)]
        logger.info("")
        _log_time_section("Total (serve + send)", _compute_percentiles(total_times))

    logger.info("=" * 80)
    logger.info("")


def _get_proxy_url_for_container(wiremock_url: str, wiremock_container_name: str, network_mode: str = None) -> str:
    """
    Get Docker-accessible proxy URL for WireMock.
    
    Network modes:
    - "host" (Linux): All containers share host network → use localhost
    - None (macOS default bridge): Containers use host.docker.internal to reach host
    
    Args:
        wiremock_url: WireMock URL (host-accessible, e.g., http://127.0.0.1:12345)
        wiremock_container_name: Name of WireMock container (not used, kept for API compat)
        network_mode: Docker network mode ("host" on Linux, None on macOS)
    
    Returns:
        Proxy URL accessible from other containers
    """
    if network_mode == "host":
        # Linux host network: all containers share host network stack → use localhost
        return wiremock_url
    else:
        # macOS default bridge: use host.docker.internal to reach host-bound ports
        wiremock_port = wiremock_url.split(":")[-1]
        return f"http://host.docker.internal:{wiremock_port}"


def _extract_row_count_from_recording(results_dir: Path, test_name: str, driver: str, driver_type: str) -> int:
    """Extract row count from recording phase CSV file."""
    # Find the recording CSV file
    # Core driver doesn't include driver_type in filename, others do
    if driver == "core":
        pattern = f"{test_name}_record_{driver}_*.csv"
    else:
        pattern = f"{test_name}_record_{driver}_{driver_type}_*.csv"
    
    csv_files = list(results_dir.glob(pattern))
    
    if not csv_files:
        logger.warning(f"No recording CSV found matching pattern: {pattern}")
        return None
    
    csv_file = max(csv_files, key=lambda p: p.stat().st_mtime)
    
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
    wiremock_url: str,
    wiremock_container_name: str,
    network_mode: str,
    driver: str = "core",
    driver_type: str = None,
    setup_queries: list[str] = None,
    use_local_binary: bool = False,
    s3_files_dir: Path = None,
    is_replay: bool = False,
    expected_row_count: int = None,
    wiremock_manager: "WiremockManager" = None,
):
    """
    Run test with WireMock proxy configuration.
    
    Sets up proxy environment variables and exports the WireMock CA certificate
    so drivers trust the dynamically generated MITM certificates.
    """
    # Build environment variables for proxy and replay mode
    # With host network: use localhost; with bridge network: use container name
    proxy_url = _get_proxy_url_for_container(wiremock_url, wiremock_container_name, network_mode)
    env_vars = {
        "HTTPS_PROXY": proxy_url,
        "HTTP_PROXY": proxy_url,
        # Lowercase variants required by the old Snowflake ODBC driver (libcurl)
        "https_proxy": proxy_url,
        "http_proxy": proxy_url,
    }
    
    if is_replay:
        env_vars["WIREMOCK_REPLAY"] = "true"
        if expected_row_count is not None:
            logger.info(f"Setting EXPECTED_ROW_COUNT={expected_row_count} for replay validation")
            env_vars["EXPECTED_ROW_COUNT"] = str(expected_row_count)
    
    # Export the WireMock CA cert so the driver trusts the dynamically generated
    # MITM certificates. Each Dockerfile appends this to the appropriate CA bundle.
    if wiremock_manager:
        try:
            ca_cert_path = wiremock_manager.export_ca_cert(results_dir)
            env_vars["WIREMOCK_CA_CERT"] = "/results/" + ca_cert_path.name
            if driver == "odbc" and driver_type == "old":
                env_vars["WIREMOCK_PROXY_URL"] = proxy_url
        except Exception as e:
            logger.error(
                "Failed to export WireMock CA cert; skipping this test execution.",
                exc_info=True,
            )
            return
    
    # Execute test with common function
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
        test_type=PerfTestType.SELECT,  # WireMock tests are SELECT-based
        use_local_binary=use_local_binary,
        s3_files_dir=s3_files_dir,
        env_vars=env_vars,
        network_mode=network_mode,
    )


def _write_wiremock_monitor_results(
    result: MonitorResult,
    results_dir: Path,
    test_name: str,
    phase: str,
    driver_type: str | None,
):
    """Write WireMock monitoring CSV + log file and log a summary to console."""
    timestamp = int(time.time())
    driver_suffix = f"_{driver_type}" if driver_type else ""

    # --- CSV stats ---
    if result.samples:
        csv_name = f"wiremock_stats_{test_name}_{phase}{driver_suffix}_{timestamp}.csv"
        csv_path = results_dir / csv_name
        with open(csv_path, "w", newline="") as f:
            writer = csv.writer(f)
            writer.writerow(["timestamp_ms", "cpu_percent", "memory_usage_mb", "memory_limit_mb"])
            for s in result.samples:
                writer.writerow([s.timestamp_ms, s.cpu_percent, s.memory_usage_mb, s.memory_limit_mb])
        logger.info(f"Wrote WireMock stats: {csv_path.name} ({len(result.samples)} samples)")

    # --- Container logs ---
    if result.logs:
        log_name = f"wiremock_logs_{test_name}_{phase}{driver_suffix}_{timestamp}.log"
        log_path = results_dir / log_name
        log_path.write_text(result.logs, encoding="utf-8")
        logger.info(f"Wrote WireMock logs: {log_path.name}")

    # --- Console summary ---
    _log_wiremock_container_summary(result, phase, driver_type)


def _log_wiremock_container_summary(
    result: MonitorResult, phase: str, driver_type: str | None
):
    """Log peak CPU, peak memory, and average memory from WireMock container stats."""
    if not result.samples:
        return

    cpus = [s.cpu_percent for s in result.samples]
    mems = [s.memory_usage_mb for s in result.samples]
    label = f"{phase}" + (f" ({driver_type})" if driver_type else "")

    logger.info("")
    logger.info(f"WireMock container stats [{label}]:")
    logger.info(f"  Peak CPU:       {max(cpus):>8.1f} %")
    logger.info(f"  Peak Memory:    {max(mems):>8.1f} MB")
    logger.info(f"  Avg Memory:     {statistics.mean(mems):>8.1f} MB")
    logger.info(f"  Memory Limit:   {result.samples[0].memory_limit_mb:>8.1f} MB")
