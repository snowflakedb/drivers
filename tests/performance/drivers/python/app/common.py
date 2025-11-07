"""Common utilities for performance testing."""

import statistics
from typing import List, Dict, Callable, Any


def run_warmup(execute_fn: Callable, cursor, sql: str, warmup_iterations: int) -> None:
    """Generic warmup execution for any test type."""
    if warmup_iterations == 0:
        return
    
    for _ in range(warmup_iterations):
        execute_fn(cursor, sql)


def run_test_iterations(
    execute_fn: Callable, cursor, sql: str, iterations: int
) -> List[Dict[str, Any]]:
    """Generic test iteration execution for any test type."""
    results = []
    
    for _ in range(iterations):
        result = execute_fn(cursor, sql)
        results.append(result)
    
    return results


def print_timing_stats(label: str, values: List[float]) -> None:
    """Print timing statistics with consistent formatting."""
    stats = _calculate_statistics(values)
    print(f"  {label}: median={stats['median']:.3f}s  "
          f"min={stats['min']:.3f}s  max={stats['max']:.3f}s")


def _calculate_statistics(values: List[float]) -> Dict[str, float]:
    """Calculate median, min, max for a list of values."""
    if not values:
        return {"median": 0.0, "min": 0.0, "max": 0.0}
    
    return {
        "median": statistics.median(values),
        "min": min(values),
        "max": max(values),
    }
