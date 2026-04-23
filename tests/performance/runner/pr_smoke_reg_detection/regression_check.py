"""
PR performance regression detection.

Compares PR test results against Benchstore main branch baselines.
When regression exceeds threshold, re-runs only regressed tests to confirm.
Every run is uploaded to a separate Benchstore benchmark with diff_pct aggregates
and a REGRESSION_DETECTED tag for full observability.
"""

import csv
import json
import logging
import os
import statistics
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from runner.pr_smoke_reg_detection.benchstore_baseline import get_main_baseline
from runner.local_compare import is_main_result_file

logger = logging.getLogger(__name__)

PR_REGRESSION_BENCHMARK = "Universal_Driver_PR_Regression"


@dataclass
class RegressionResult:
    test_name: str
    main_median: float
    pr_median: float
    diff_pct: float
    is_regressed: bool
    confirmation_runs: list[float] = field(default_factory=list)
    confirmed: bool = False


def read_pr_medians(results_dir: Path, driver: str, driver_type: str) -> dict[str, float]:
    """
    Read median fetch_s (or query_s for PUT/GET) from PR result CSVs.

    Returns:
        dict mapping test_name -> median value
    """
    driver_type_subdir = driver_type if driver != "core" else "universal"
    search_dir = results_dir / driver_type_subdir

    if not search_dir.exists():
        logger.warning(f"Results directory not found: {search_dir}")
        return {}

    medians = {}

    for test_dir in sorted(search_dir.iterdir()):
        if not test_dir.is_dir():
            continue

        test_name = test_dir.name
        csv_files = [
            f for f in test_dir.glob("*.csv") if is_main_result_file(f)
        ]
        if not csv_files:
            continue

        result_csv = max(csv_files, key=lambda p: p.stat().st_mtime)

        try:
            with open(result_csv, "r") as f:
                reader = csv.DictReader(f)
                rows = list(reader)

            if not rows:
                continue

            metric_col = "fetch_s" if "fetch_s" in rows[0] else "query_s"
            values = [float(r[metric_col]) for r in rows if r.get(metric_col)]
            if values:
                medians[test_name] = statistics.median(values)
        except Exception as e:
            logger.warning(f"Failed to read results for {test_name}: {e}")

    return medians


def check_regression(
    pr_results: dict[str, float],
    baselines: dict[str, float],
    threshold_pct: float = 5.0,
) -> list[RegressionResult]:
    """Compare PR medians vs baselines. Return list of RegressionResult."""
    results = []
    for test_name, pr_median in pr_results.items():
        main_median = baselines.get(test_name)
        if main_median is None or main_median == 0:
            logger.warning(f"No baseline for {test_name}, skipping regression check")
            continue

        diff_pct = (pr_median - main_median) / main_median * 100
        is_regressed = diff_pct > threshold_pct

        results.append(
            RegressionResult(
                test_name=test_name,
                main_median=main_median,
                pr_median=pr_median,
                diff_pct=diff_pct,
                is_regressed=is_regressed,
            )
        )
    return results


def _rerun_test(
    test_name: str,
    test_params: dict,
    results_dir: Path,
    run_id: str,
    driver: str,
    driver_type: str,
    iterations: int,
    warmup_iterations: int,
) -> Optional[float]:
    """
    Re-run a single test using WireMock replay with existing mappings.

    Returns the median fetch_s from the re-run, or None on failure.
    """
    from runner.modes.wiremock_runner import run_wiremock_performance_test

    sql_command = test_params["sql_command"]
    parameters_json = test_params["parameters_json"]
    setup_queries = test_params.get("setup_queries", [])

    rerun_dir = results_dir / f"_rerun_{test_name}_{int(time.time())}"
    rerun_dir.mkdir(parents=True, exist_ok=True)

    try:
        result_files = run_wiremock_performance_test(
            test_name=test_name,
            sql_command=sql_command,
            parameters_json=parameters_json,
            results_dir=rerun_dir,
            iterations=iterations,
            warmup_iterations=warmup_iterations,
            driver=driver,
            driver_type=driver_type if driver != "core" else None,
            setup_queries=setup_queries,
            run_id=run_id,
            preserve_mappings=True,
            reuse_mappings_dir=run_id,
        )
    except Exception as e:
        logger.error(f"Re-run failed for {test_name}: {e}")
        return None

    main_files = [f for f in result_files if is_main_result_file(f)]
    if not main_files:
        logger.error(f"No result files from re-run of {test_name}")
        return None

    result_csv = max(main_files, key=lambda p: p.stat().st_mtime)
    try:
        with open(result_csv, "r") as f:
            reader = csv.DictReader(f)
            rows = list(reader)
        if not rows:
            return None
        metric_col = "fetch_s" if "fetch_s" in rows[0] else "query_s"
        values = [float(r[metric_col]) for r in rows if r.get(metric_col)]
        return statistics.median(values) if values else None
    except Exception as e:
        logger.error(f"Failed to read re-run results for {test_name}: {e}")
        return None


def _log_report(
    results: list[RegressionResult],
    threshold_pct: float,
    baseline_run_key: Optional[int],
    passed: bool,
):
    """Log a formatted regression report table."""
    logger.info("")
    logger.info("=" * 100)
    if passed:
        logger.info("REGRESSION CHECK PASSED")
    else:
        logger.error("REGRESSION CHECK FAILED")
    logger.info("=" * 100)
    logger.info("")

    header = f"{'Test':<50} | {'Main':>10} | {'PR':>10} | {'Diff':>8} | {'Status'}"
    logger.info(header)
    logger.info("-" * len(header))

    for r in results:
        status = "OK"
        if r.is_regressed:
            if r.confirmed:
                runs_str = ", ".join(f"{v:.3f}s" for v in r.confirmation_runs)
                status = f"REGRESSED (confirmed, reruns: [{runs_str}])"
            else:
                status = "noise (not confirmed on re-run)"

        logger.info(
            f"{r.test_name:<50} | {r.main_median:>9.3f}s | {r.pr_median:>9.3f}s | "
            f"{r.diff_pct:>+7.1f}% | {status}"
        )

    logger.info("")
    logger.info(f"Threshold: {threshold_pct}%")
    if baseline_run_key is not None:
        logger.info(f"Baseline: Benchstore run_key={baseline_run_key}")
    logger.info("=" * 100)
    logger.info("")


def _upload_to_benchstore(
    results: list[RegressionResult],
    results_dir: Path,
    driver: str,
    driver_type: str,
    baseline_run_key: Optional[int],
    threshold_pct: float,
    use_local_auth: bool,
    regression_detected: bool,
):
    """Upload PR regression check data to the separate PR regression benchmark.

    Every run is uploaded for full observability. A REGRESSION_DETECTED tag
    marks whether a confirmed regression was found, and per-test diff_pct
    aggregates are always included so trends are visible even for passing runs.
    """
    from benchstore.proto import benchstore_pb2
    from benchstore.client.quickstore import Quickstore
    from google.protobuf.timestamp_pb2 import Timestamp

    from benchstore.client import benchmark_manager

    from runner.benchstore_upload import (
        PROJECT_NAME,
        login_to_benchstore,
        get_snowhouse_config,
        get_snowflake_connection_params,
        get_local_connection_params,
        read_csv_results,
        _sanitize_tag,
    )
    from runner.container import get_resource_limits
    from runner.utils import collect_node_info

    sf_storage = login_to_benchstore(use_local_auth=use_local_auth)
    benchmark_manager.find_or_create_benchmark(
        PROJECT_NAME, PR_REGRESSION_BENCHMARK, sf_storage
    )
    if use_local_auth:
        connection_params = get_local_connection_params()
    else:
        snowhouse_config = get_snowhouse_config()
        connection_params = get_snowflake_connection_params(snowhouse_config)

    is_local = os.getenv("BUILD_NUMBER") is None
    build_number = "LOCAL" if is_local else os.getenv("BUILD_NUMBER")
    branch_name = "LOCAL" if is_local else os.getenv("BRANCH_NAME", "unknown")
    jenkins_node = os.getenv("JENKINS_NODE_LABEL", "UNKNOWN")
    pr_number = os.getenv("CHANGE_ID", "LOCAL")

    resource_limits = get_resource_limits()
    node_info = collect_node_info()

    driver_tag_value = f"{driver}_old" if driver_type == "old" else driver

    tags = [
        f"BUILD_NUMBER={build_number}",
        f"BRANCH_NAME={branch_name}",
        f"DRIVER={driver_tag_value}",
        f"JENKINS_NODE={jenkins_node}",
        f"DOCKER_MEMORY={resource_limits['memory']}",
        f"DOCKER_CPU={resource_limits['cpu']}",
        f"PR_NUMBER={pr_number}",
        f"REGRESSION_THRESHOLD={threshold_pct}",
        f"BASELINE_RUN_KEY={baseline_run_key or 'UNKNOWN'}",
        f"REGRESSION_DETECTED={'true' if regression_detected else 'false'}",
        f"NODE_CPU_MODEL={node_info.get('node_cpu_model', 'UNKNOWN')}",
        f"NODE_CPU_CORES={node_info.get('node_cpu_cores', 'UNKNOWN')}",
        f"NODE_MEMORY_GB={node_info.get('node_memory_gb', 'UNKNOWN')}",
    ]
    if "node_instance_type" in node_info:
        tags.append(f"NODE_INSTANCE_TYPE={node_info['node_instance_type']}")

    tags = [_sanitize_tag(t) for t in tags]

    tested_names = {r.test_name for r in results}

    comparable_tags = [t for t in tags if t.startswith("DRIVER=") or t.startswith("JENKINS_NODE=")]

    quickstore_input = benchstore_pb2.QuickstoreInput(
        benchmark_name_lookup=benchstore_pb2.BenchmarkNameLookup(
            project_name=PROJECT_NAME,
            benchmark_name=PR_REGRESSION_BENCHMARK,
        ),
        tags=tags,
        default_comparable_tags=comparable_tags,
    )

    driver_type_subdir = driver_type if driver != "core" else "universal"
    search_dir = results_dir / driver_type_subdir

    try:
        with Quickstore(quickstore_input, snowflake_connection_params=connection_params) as quickstore:
            uploaded = 0
            for test_dir in sorted(search_dir.iterdir()):
                if not test_dir.is_dir():
                    continue
                if test_dir.name not in tested_names:
                    continue

                csv_files = [
                    f for f in test_dir.glob("*.csv") if is_main_result_file(f)
                ]
                if not csv_files:
                    continue
                result_csv = max(csv_files, key=lambda p: p.stat().st_mtime)

                try:
                    rows = read_csv_results(result_csv)
                except Exception:
                    continue

                for row in rows:
                    metrics = {f"{test_dir.name}_query_s": row["query_s"]}
                    if "fetch_s" in row:
                        metrics[f"{test_dir.name}_fetch_s"] = row["fetch_s"]
                    if "cpu_time_s" in row:
                        metrics[f"{test_dir.name}_cpu_time_s"] = row["cpu_time_s"]
                    if "peak_rss_mb" in row:
                        metrics[f"{test_dir.name}_peak_rss_mb"] = row["peak_rss_mb"]

                    ts = Timestamp()
                    ts.FromMilliseconds(row["timestamp"])
                    quickstore.add_sample_point_from_input(
                        benchstore_pb2.AddSamplePointInput(timestamp=ts, metrics=metrics)
                    )
                    uploaded += 1

            for r in results:
                quickstore.add_run_aggregate(
                    benchstore_pb2.AddRunAggregateInput(
                        custom_aggregate_label=f"{r.test_name}_diff_pct",
                        custom_aggregate_value=r.diff_pct,
                    )
                )

            logger.info(
                f"Uploaded {uploaded} sample points + diff_pct aggregates "
                f"for {len(results)} tests to {PR_REGRESSION_BENCHMARK} "
                f"(regression_detected={regression_detected})"
            )

    except Exception as e:
        logger.error(f"Failed to upload regression data to Benchstore: {e}")


def _write_summary_file(
    results: list[RegressionResult],
    results_dir: Path,
    driver: str,
    passed: bool,
    threshold_pct: float,
    baseline_run_key: Optional[int],
):
    """Write a compact JSON summary to results/../regression_summary_{driver}.json.

    The Jenkinsfile reads this file to include per-test details in the
    pipeline-level summary that is easy to find without scrolling through logs.
    """
    summary = {
        "driver": driver,
        "passed": passed,
        "threshold_pct": threshold_pct,
        "baseline_run_key": baseline_run_key,
        "tests": [
            {
                "name": r.test_name,
                "main_s": round(r.main_median, 4),
                "pr_s": round(r.pr_median, 4),
                "diff_pct": round(r.diff_pct, 1),
                "status": (
                    "REGRESSED" if r.confirmed
                    else "noise" if r.is_regressed
                    else "OK"
                ),
                "confirmation_runs": [round(v, 4) for v in r.confirmation_runs],
            }
            for r in results
        ],
    }
    out_path = results_dir.parent / f"regression_summary_{driver}.json"
    try:
        # Merge with existing file so fast + slow runs both appear in one summary.
        if out_path.exists():
            existing = json.loads(out_path.read_text())
            summary["tests"] = existing.get("tests", []) + summary["tests"]
            summary["passed"] = existing.get("passed", True) and summary["passed"]
        out_path.write_text(json.dumps(summary, indent=2))
        logger.info(f"Wrote regression summary: {out_path}")
    except Exception as e:
        logger.warning(f"Could not write regression summary file: {e}")


def run_regression_check(
    results_dir: Path,
    driver: str,
    driver_type: str,
    threshold_pct: float,
    use_local_auth: bool = False,
    test_params_registry: Optional[dict] = None,
    run_id: Optional[str] = None,
    iterations: int = 10,
    warmup_iterations: int = 2,
    max_retries: int = 2,
) -> bool:
    """
    Main entry point: compare PR results against Benchstore baseline.

    Flow:
        1. Read PR results from results_dir CSVs
        2. Query Benchstore for latest main baselines
        3. Compare PR medians vs main medians
        4. If no regression > threshold: PASS
        5. If regression detected: re-run regressed tests (up to max_retries)
        6. Confirm regression if it appears in >= 2 of (1 + max_retries) total runs
        7. Upload all results to PR_Regression benchmark (with REGRESSION_DETECTED tag
           and per-test diff_pct aggregates)
        8. Return True if passed, False if regression confirmed

    Args:
        results_dir: Path to run results directory
        driver: Driver name
        driver_type: Driver type (universal/old)
        threshold_pct: Regression threshold percentage
        use_local_auth: Use browser auth for Benchstore
        test_params_registry: Dict mapping test_name -> {sql_command, parameters_json, ...}
        run_id: Run ID for reusing WireMock mappings
        iterations: Iterations for re-runs
        warmup_iterations: Warmup iterations for re-runs
        max_retries: Max re-runs for regressed tests (default 2, total runs = 1 + max_retries)

    Returns:
        True if check passed, False if confirmed regression found
    """
    logger.info("")
    logger.info("=" * 100)
    logger.info(">>> REGRESSION CHECK: Comparing PR results against Benchstore main baseline")
    logger.info("=" * 100)
    logger.info("")

    # 1. Read PR results
    pr_medians = read_pr_medians(results_dir, driver, driver_type)
    if not pr_medians:
        logger.warning("No PR results found - skipping regression check")
        return True

    logger.info(f"PR results for {len(pr_medians)} tests:")
    for name, val in sorted(pr_medians.items()):
        logger.info(f"  {name}: {val:.4f}s")

    # 2. Query Benchstore for baselines
    logger.info("")
    logger.info("Querying Benchstore for main branch baselines...")
    baselines, baseline_run_key = get_main_baseline(
        test_names=list(pr_medians.keys()),
        driver=driver,
        driver_type=driver_type,
        use_local_auth=use_local_auth,
    )

    if not baselines:
        logger.warning("No baselines found in Benchstore - skipping regression check")
        return True

    # 3. Compare
    logger.info("")
    logger.info("Comparing PR results against baselines...")
    results = check_regression(pr_medians, baselines, threshold_pct)

    regressed = [r for r in results if r.is_regressed]

    if not regressed:
        _log_report(results, threshold_pct, baseline_run_key, passed=True)
        _write_summary_file(results, results_dir, driver, passed=True, threshold_pct=threshold_pct, baseline_run_key=baseline_run_key)
        logger.info("Uploading regression check data to Benchstore...")
        try:
            _upload_to_benchstore(
                results=results,
                results_dir=results_dir,
                driver=driver,
                driver_type=driver_type,
                baseline_run_key=baseline_run_key,
                threshold_pct=threshold_pct,
                use_local_auth=use_local_auth,
                regression_detected=False,
            )
        except Exception as e:
            logger.error(f"Benchstore upload failed (non-fatal): {e}")
        return True

    logger.info("")
    logger.info(
        f"Potential regression detected in {len(regressed)} test(s): "
        + ", ".join(r.test_name for r in regressed)
    )

    # 4. Re-run regressed tests to confirm
    if test_params_registry is None or run_id is None:
        logger.warning(
            "Cannot re-run tests (missing test_params_registry or run_id). "
            "Treating initial regression as confirmed."
        )
        for r in regressed:
            r.confirmed = True
    else:
        for r in regressed:
            r.confirmation_runs.append(r.pr_median)

            for retry_num in range(1, max_retries + 1):
                logger.info("")
                logger.info(
                    f"Re-run {retry_num}/{max_retries} for {r.test_name}..."
                )

                params = test_params_registry.get(r.test_name)
                if params is None:
                    logger.error(
                        f"No test params found for {r.test_name} - cannot re-run"
                    )
                    break

                rerun_median = _rerun_test(
                    test_name=r.test_name,
                    test_params=params,
                    results_dir=results_dir,
                    run_id=run_id,
                    driver=driver,
                    driver_type=driver_type,
                    iterations=iterations,
                    warmup_iterations=warmup_iterations,
                )

                if rerun_median is not None:
                    r.confirmation_runs.append(rerun_median)
                    logger.info(f"  Re-run median: {rerun_median:.4f}s")
                else:
                    logger.warning(f"  Re-run {retry_num} produced no result")

            # Confirm if regression appears in >= 2 of total runs.
            # If reruns failed to produce results (len < expected), treat
            # conservatively as confirmed since we can't disprove the regression.
            expected_runs = 1 + max_retries
            if len(r.confirmation_runs) < expected_runs:
                r.confirmed = True
                logger.warning(
                    f"  {r.test_name}: only {len(r.confirmation_runs)}/{expected_runs} "
                    f"runs produced results — treating as CONFIRMED (cannot disprove)"
                )
            else:
                regressed_count = sum(
                    1
                    for m in r.confirmation_runs
                    if r.main_median > 0
                    and (m - r.main_median) / r.main_median * 100 > threshold_pct
                )
                r.confirmed = regressed_count >= 2
                logger.info(
                    f"  {r.test_name}: regressed in {regressed_count}/{len(r.confirmation_runs)} runs "
                    f"-> {'CONFIRMED' if r.confirmed else 'NOT CONFIRMED (noise)'}"
                )

    # 5. Report
    any_confirmed = any(r.confirmed for r in results)
    _log_report(results, threshold_pct, baseline_run_key, passed=not any_confirmed)
    _write_summary_file(results, results_dir, driver, passed=not any_confirmed, threshold_pct=threshold_pct, baseline_run_key=baseline_run_key)

    # 6. Upload every run for observability (tagged with regression status)
    logger.info("Uploading regression check data to Benchstore...")
    try:
        _upload_to_benchstore(
            results=results,
            results_dir=results_dir,
            driver=driver,
            driver_type=driver_type,
            baseline_run_key=baseline_run_key,
            threshold_pct=threshold_pct,
            use_local_auth=use_local_auth,
            regression_detected=any_confirmed,
        )
    except Exception as e:
        logger.error(f"Benchstore upload failed (non-fatal): {e}")

    return not any_confirmed
