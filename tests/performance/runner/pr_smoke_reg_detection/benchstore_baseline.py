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
from runner.result_format import is_arrow_baseline_run, merge_arrow_baseline_runs

logger = logging.getLogger(__name__)


def _query_run_info(sf_storage, benchmark_key, tags: list[str], limit: int):
    query = benchstore_pb2.RunInfoQuery(
        benchmark_key=benchmark_key,
        tags=tags,
        limit=limit,
    )
    return sf_storage.query_run_info(query)


def _query_arrow_baseline_runs(sf_storage, benchmark_key, driver_tag: str, num_runs: int):
    """Recent Arrow main runs: tagged RESULT_FORMAT=ARROW, then untagged history."""
    base_tags = ["BRANCH_NAME=main", f"DRIVER={driver_tag}"]
    tagged = _query_run_info(
        sf_storage, benchmark_key, [*base_tags, "RESULT_FORMAT=ARROW"], num_runs
    )
    tagged_runs = list(tagged.run_info_list) if tagged.run_info_list else []
    if len(tagged_runs) >= num_runs:
        return tagged_runs[:num_runs]

    unfiltered = _query_run_info(sf_storage, benchmark_key, base_tags, 50)
    unfiltered_runs = list(unfiltered.run_info_list) if unfiltered.run_info_list else []
    skipped_json = sum(1 for run in unfiltered_runs if not is_arrow_baseline_run(run))
    if skipped_json:
        logger.info(
            "Excluded %d JSON main-branch run(s) from Arrow baseline", skipped_json
        )
    return merge_arrow_baseline_runs(tagged_runs, unfiltered_runs, num_runs)


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

    Uses Arrow runs only (`RESULT_FORMAT=ARROW` first, then untagged pre-tag
    history). JSON nightlies are excluded so they cannot contaminate the PR
    smoke baseline.

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

    arrow_runs = _query_arrow_baseline_runs(
        sf_storage, benchmark_key, driver_tag, num_runs
    )

    if not arrow_runs:
        logger.warning("No Arrow main-branch runs found in Benchstore")
        return {}, None

    latest_run_key = arrow_runs[0].run_key

    logger.info(f"Fetched {len(arrow_runs)} Arrow baseline run(s):")
    for run_info in arrow_runs:
        logger.info(f"  run_key={run_info.run_key}")
        for tag in run_info.tags:
            if tag.startswith("BUILD_NUMBER=") or tag.startswith("BRANCH_NAME=") or tag.startswith("RESULT_FORMAT="):
                logger.info(f"    {tag}")

    # Collect per-test medians from each run
    per_test_values: dict[str, list[float]] = {name: [] for name in test_names}

    for run_info in arrow_runs:
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
