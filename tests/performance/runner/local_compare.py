"""Local performance comparison: compare current results with previous local runs."""
import csv
import logging
import re
import statistics
import sys
from pathlib import Path
from typing import Optional

_GREEN = "\033[32m"
_RED = "\033[31m"
_RESET = "\033[0m"
_USE_COLOR = sys.stdout.isatty()


def _color(text: str, color: str) -> str:
    return f"{color}{text}{_RESET}" if _USE_COLOR else text

logger = logging.getLogger(__name__)


def _detect_metric_column(csv_path: Path) -> Optional[str]:
    """Return 'fetch_s' for SELECT tests or 'query_s' for PUT/GET tests."""
    try:
        with open(csv_path, "r") as f:
            header = f.readline()
        if "fetch_s" in header:
            return "fetch_s"
        if "query_s" in header:
            return "query_s"
    except Exception as e:
        logger.debug(f"Failed to detect metric column from {csv_path}: {e}")
    return None


def _read_median(csv_path: Path, metric_col: str) -> Optional[float]:
    """Read median of metric_col from a result CSV. Returns None on failure."""
    try:
        with open(csv_path, "r") as f:
            reader = csv.DictReader(f)
            values = [float(row[metric_col]) for row in reader if row.get(metric_col)]
        return statistics.median(values) if values else None
    except Exception as e:
        logger.debug(f"Failed to read {metric_col} from {csv_path}: {e}")
        return None


def _is_main_result_file(path: Path) -> bool:
    """Return True if file is a main result CSV (not a record/memory/wiremock file)."""
    name = path.name
    if name.startswith("memory_timeline_"):
        return False
    if name.startswith("wiremock_"):
        return False
    # Exclude recording-phase files: pattern is <test>_record_<driver>_...
    # 'recorded' appears in test names (e.g. select_...recorded_http...) — that's fine.
    # '_record_' appears as a separator between test name and driver name in record files.
    if re.search(r"_record_[a-z]", name):
        return False
    return True


def _pick_main_file(files: list[Path]) -> Optional[Path]:
    return max(
        (f for f in files if _is_main_result_file(f)),
        key=lambda p: p.stat().st_mtime,
        default=None,
    )


def _find_result_file(
    run_dir: Path,
    test_name: str,
    driver: str,
    driver_type: Optional[str],
) -> Optional[Path]:
    """Find the main result CSV for a test in a given run directory."""
    if driver != "core" and driver_type:
        pattern = f"{test_name}_{driver}_{driver_type}_*.csv"
    else:
        pattern = f"{test_name}_{driver}_*.csv"
    candidates = [f for f in run_dir.glob(pattern) if _is_main_result_file(f)]
    if not candidates:
        return None
    return max(candidates, key=lambda p: p.stat().st_mtime)


def get_file_median(files: list[Path]) -> Optional[tuple[str, float]]:
    """
    Return (metric_col, median) from a list of result files, or None if not possible.
    Used to extract OLD driver median for UD vs OLD comparison.
    """
    f = _pick_main_file(files)
    if f is None:
        return None
    metric_col = _detect_metric_column(f)
    if not metric_col:
        return None
    median = _read_median(f, metric_col)
    if median is None:
        return None
    return metric_col, median

def compare_with_history(
    current_files: list[Path],
    results_dir: Path,
    test_name: str,
    driver: str,
    driver_type: Optional[str],
    old_median: Optional[float] = None,
) -> Optional[dict]:
    """
    Build comparison data for a test run against historical runs.

    Args:
        current_files: Result files from the current run
        results_dir:   Current run directory (.../results/run_YYYYMMDD_HHMMSS/)
        test_name:     Test name (without 'test_' prefix)
        driver:        Driver name (core, python, odbc, jdbc)
        driver_type:   Driver type (universal, old) or None for core
        old_median:    Optional OLD driver median from the same run (for UD vs OLD display)

    Returns:
        Dict with comparison data, or None if the current median cannot be determined.
        History fields are empty when results_dir has no previous runs.
    """
    if not current_files:
        return None

    current_file = _pick_main_file(current_files)
    if current_file is None:
        return None

    metric_col = _detect_metric_column(current_file)
    if not metric_col:
        return None

    current_median = _read_median(current_file, metric_col)
    if current_median is None:
        return None

    results_base = results_dir.parent
    current_run_name = results_dir.name

    history = []
    if results_base.exists():
        all_run_dirs = sorted(
            [
                d for d in results_base.iterdir()
                if d.is_dir() and d.name.startswith("run_") and d.name != current_run_name
            ],
            key=lambda d: d.name,
        )
        for run_dir in all_run_dirs:
            prev_file = _find_result_file(run_dir, test_name, driver, driver_type)
            if prev_file is None:
                continue
            median = _read_median(prev_file, metric_col)
            if median is not None:
                history.append((run_dir.name, median))

    last_median = history[-1][1] if history else None
    all_median = statistics.median(v for _, v in history) if history else None

    return {
        "test_name": test_name,
        "driver": driver,
        "driver_type": driver_type,
        "metric_col": metric_col,
        "current_median": current_median,
        "old_median": old_median,
        "history": history,
        "last_median": last_median,
        "all_median": all_median,
    }


def _pct_diff(current: float, reference: float) -> str:
    """Format percentage difference. Negative = faster (better)."""
    if reference == 0:
        return "N/A"
    pct = (current - reference) / reference * 100
    sign = "+" if pct > 0 else ""
    return f"{sign}{pct:.1f}%"


def _trend(current: float, reference: float) -> str:
    """Return a brief trend indicator."""
    if reference == 0:
        return ""
    pct = (current - reference) / reference * 100
    if abs(pct) < 2.0:
        return "~"
    return "slower" if pct > 0 else "faster"


def log_summary(comparisons: list[dict]) -> None:
    """Log all comparison results at session end, in test-run order."""
    if not comparisons:
        return

    logger.info("")
    logger.info("=" * 80)
    logger.info("SUMMARY")
    logger.info("=" * 80)

    for result in comparisons:
        test_name = result["test_name"]
        driver = result["driver"]
        driver_type = result["driver_type"]
        metric_col = result["metric_col"]
        current_median = result["current_median"]
        old_median = result.get("old_median")
        history = result["history"]
        last_median = result["last_median"]
        all_median = result["all_median"]

        label = f"{test_name}  ({driver}"
        if driver_type:
            label += f" {driver_type}"
        label += ")"

        logger.info("")
        logger.info(f"  {label}")

        # UD vs OLD (only when old_median is available)
        if old_median is not None:
            diff = _pct_diff(current_median, old_median)
            trend = _trend(current_median, old_median)
            if trend == "faster":
                colored = _color(f"{diff} ({trend})", _GREEN)
                suffix = "than OLD"
            elif trend == "slower":
                colored = _color(f"{diff} ({trend})", _RED)
                suffix = "than OLD"
            else:
                colored = f"{diff} ({trend})"
                suffix = "vs OLD"
            logger.info(
                f"    UD vs OLD:    {metric_col}  UD={current_median:.3f}s  OLD={old_median:.3f}s"
                f"  UD is {colored} {suffix}"
            )

        # History comparison
        if history:
            last_run_name = history[-1][0]
            last_diff = _pct_diff(current_median, last_median)
            last_trend = _trend(current_median, last_median)
            if last_trend == "faster":
                last_colored = _color(f"{last_diff}  {last_trend}", _GREEN)
            elif last_trend == "slower":
                last_colored = _color(f"{last_diff}  {last_trend}", _RED)
            else:
                last_colored = f"{last_diff}  {last_trend}"
            logger.info(
                f"    vs last run:  {metric_col}  {last_median:.3f}s -> {current_median:.3f}s"
                f"  {last_colored}"
                f"  ({last_run_name})"
            )
            if len(history) >= 2:
                all_diff = _pct_diff(current_median, all_median)
                all_trend = _trend(current_median, all_median)
                if all_trend == "faster":
                    all_colored = _color(f"{all_diff}  {all_trend}", _GREEN)
                elif all_trend == "slower":
                    all_colored = _color(f"{all_diff}  {all_trend}", _RED)
                else:
                    all_colored = f"{all_diff}  {all_trend}"
                logger.info(
                    f"    vs all prev:  {metric_col}  median {all_median:.3f}s -> {current_median:.3f}s"
                    f"  {all_colored}"
                    f"  [N={len(history)}]"
                )

    logger.info("")
    logger.info("=" * 80)
    logger.info("")
