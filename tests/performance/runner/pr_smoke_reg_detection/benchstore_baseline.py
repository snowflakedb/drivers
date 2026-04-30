"""Query Benchstore for the latest main branch performance baselines."""

import logging
import statistics
from typing import Optional

from benchstore.proto import benchstore_pb2
from benchstore.client import benchmark_manager

from runner.benchstore_upload import (
    PROJECT_NAME,
    BENCHMARK_NAME,
    login_to_benchstore,
)

logger = logging.getLogger(__name__)


def _build_metric_key_to_label(sf_storage, benchmark_info) -> dict[int, str]:
    """Resolve metric_key (int) -> label (str) from BenchmarkInfo."""
    query = benchstore_pb2.BenchmarkInfoQuery(benchmark_key=benchmark_info.benchmark_key)
    response = sf_storage.query_benchmark_info(query)

    key_to_label = {}
    for info in response.benchmark_info_list:
        for vi in info.metric_info_list:
            key_to_label[vi.value_key] = vi.label
    return key_to_label


def get_main_baseline(
    test_names: list[str],
    driver: str = "python",
    driver_type: str = "universal",
    use_local_auth: bool = False,
    num_runs: int = 3,
) -> tuple[dict[str, float], Optional[int]]:
    """
    Query Benchstore for the latest main branch median fetch_s values.

    Fetches the last `num_runs` runs and computes the median of their medians
    to reduce sensitivity to single-run outliers.

    Args:
        test_names: Test names to look up (without 'test_' prefix).
        driver: Driver name (python, odbc, core).
        driver_type: universal or old.
        use_local_auth: Use browser auth instead of config file.
        num_runs: Number of recent main runs to average over (default 3).

    Returns:
        (baselines, latest_run_key) where baselines maps test_name -> median fetch_s,
        and latest_run_key is the Benchstore run key of the most recent baseline.
    """
    sf_storage = login_to_benchstore(use_local_auth=use_local_auth)

    benchmark_info = benchmark_manager.find_or_create_benchmark(
        PROJECT_NAME, BENCHMARK_NAME, sf_storage
    )
    benchmark_key = benchmark_info.benchmark_key

    key_to_label = _build_metric_key_to_label(sf_storage, benchmark_info)

    driver_tag = f"{driver}_old" if driver_type == "old" else driver

    query = benchstore_pb2.RunInfoQuery(
        benchmark_key=benchmark_key,
        tags=[
            "BRANCH_NAME=main",
            f"DRIVER={driver_tag}",
        ],
        limit=num_runs,
    )
    response = sf_storage.query_run_info(query)

    if not response.run_info_list:
        logger.warning("No main branch runs found in Benchstore")
        return {}, None

    latest_run_key = response.run_info_list[0].run_key

    logger.info(f"Fetched {len(response.run_info_list)} baseline run(s):")
    for run_info in response.run_info_list:
        logger.info(f"  run_key={run_info.run_key}")
        for tag in run_info.tags:
            if tag.startswith("BUILD_NUMBER=") or tag.startswith("BRANCH_NAME="):
                logger.info(f"    {tag}")

    # Collect per-test medians from each run
    per_test_values: dict[str, list[float]] = {name: [] for name in test_names}

    for run_info in response.run_info_list:
        label_to_agg = {}
        for agg in run_info.aggregate.metric_aggregate_list:
            label = key_to_label.get(agg.metric_key)
            if label:
                label_to_agg[label] = agg

        for test_name in test_names:
            fetch_label = f"{test_name}_fetch_s"
            agg = label_to_agg.get(fetch_label)
            if agg and agg.median > 0:
                per_test_values[test_name].append(agg.median)

    baselines: dict[str, float] = {}
    for test_name in test_names:
        values = per_test_values[test_name]
        if values:
            baseline = statistics.median(values)
            baselines[test_name] = baseline
            logger.info(
                f"  {test_name}: baseline={baseline:.4f}s "
                f"(from {len(values)} run(s): {', '.join(f'{v:.4f}' for v in values)})"
            )
        else:
            logger.warning(f"  {test_name}: no baseline found in Benchstore")

    return baselines, latest_run_key
