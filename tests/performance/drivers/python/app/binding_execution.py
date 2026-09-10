"""Parameter-binding execution and performance measurement."""

from __future__ import annotations

import json
import os
import time
from typing import Any

from common import get_peak_rss_mb, print_timing_stats, run_test_iterations, run_warmup
from resource_monitor import ResourceMonitor

def execute_binding_test(
    cursor,
    sql_command: str,
    warmup_iterations: int,
    iterations: int,
    binding_mode: str,
    binding_params_json: str | None,
):
    print("\n=== Executing Parameter Binding Test ===")
    print(f"Query: {sql_command}")
    print(f"Binding mode: {binding_mode}")

    binding_params = _parse_binding_params(binding_mode, binding_params_json)
    execute_fn = _make_execute_fn(cursor, sql_command, binding_mode, binding_params)
    run_warmup(execute_fn, cursor, sql_command, warmup_iterations)

    monitor = ResourceMonitor(interval_s=0.1)
    monitor.start()
    results = run_test_iterations(execute_fn, cursor, sql_command, iterations)
    memory_timeline = monitor.stop()

    _validate_results(results, binding_mode)
    _print_statistics(results)
    print(f"  Memory timeline: {len(memory_timeline)} samples collected")
    return results, memory_timeline


def _parse_binding_params(binding_mode: str, binding_params_json: str | None) -> Any:
    if not binding_params_json:
        raise RuntimeError("BINDING_PARAMS_JSON is required for parameter binding tests")
    raw = json.loads(binding_params_json)
    if binding_mode == "execute":
        return tuple(_coerce_scalar_param(value) for value in raw)
    if binding_mode == "executemany":
        return [tuple(row) for row in raw]
    raise RuntimeError(f"Unsupported binding mode '{binding_mode}'")


def _coerce_scalar_param(value: Any) -> Any:
    if isinstance(value, list) and value and all(
        isinstance(byte, int) and 0 <= byte <= 255 for byte in value
    ):
        return bytes(value)
    return value


def _make_execute_fn(cursor, sql_command: str, binding_mode: str, binding_params: Any):
    if binding_mode == "execute":
        return lambda _cursor, _sql: _execute_scalar(cursor, sql_command, binding_params)
    if binding_mode == "executemany":
        return lambda _cursor, _sql: _execute_executemany(cursor, sql_command, binding_params)
    raise RuntimeError(f"Unsupported binding mode '{binding_mode}'")


def _execute_scalar(cursor, sql_command: str, binding_params: tuple[Any, ...]):
    query_start = time.time()
    _execute_bound(cursor, sql_command, binding_params)
    query_time = time.time() - query_start

    fetch_start = time.time()
    rows = cursor.fetchall()
    fetch_time = time.time() - fetch_start

    return _result_dict(query_time, fetch_time, len(rows))


def _execute_executemany(cursor, sql_command: str, binding_params: list[tuple[Any, ...]]):
    if not _via_wiremock():
        _truncate_bind_table(cursor)

    query_start = time.time()
    _executemany_bound(cursor, sql_command, binding_params)
    query_time = time.time() - query_start

    row_count = (
        len(binding_params) if _via_wiremock() else _inserted_row_count(cursor)
    )
    return _result_dict(query_time, 0.0, row_count)


def _via_wiremock() -> bool:
    return bool(
        os.getenv("HTTPS_PROXY") or os.getenv("HTTP_PROXY") or os.getenv("WIREMOCK_REPLAY")
    )


def _binding_kwargs() -> dict[str, bool]:
    if os.getenv("DRIVER_TYPE", "universal") == "universal":
        return {"_force_qmark_paramstyle": True}
    return {}


def _execute_bound(cursor, sql_command: str, binding_params: tuple[Any, ...]):
    cursor.execute(sql_command, binding_params, **_binding_kwargs())


def _executemany_bound(cursor, sql_command: str, binding_params: list[tuple[Any, ...]]):
    cursor.executemany(sql_command, binding_params, **_binding_kwargs())


BIND_PERF_TABLE = "bind_perf_wide_t"


def _truncate_bind_table(cursor):
    try:
        cursor.execute(f"TRUNCATE TABLE IF EXISTS {BIND_PERF_TABLE}")
    except Exception:
        cursor.execute(f"DELETE FROM {BIND_PERF_TABLE}")


def _inserted_row_count(cursor) -> int:
    cursor.execute(f"SELECT COUNT(*) FROM {BIND_PERF_TABLE}")
    row = cursor.fetchone()
    if row is None:
        raise RuntimeError(f"SELECT COUNT(*) FROM {BIND_PERF_TABLE} returned no rows")
    return int(row[0])


def _result_dict(query_time: float, fetch_time: float, row_count: int) -> dict[str, Any]:
    return {
        "timestamp": int(time.time() * 1000),
        "query_time_s": query_time,
        "fetch_time_s": fetch_time,
        "row_count": row_count,
        "cpu_time_s": 0.0,
        "peak_rss_mb": get_peak_rss_mb(),
    }


def _validate_results(results: list[dict[str, Any]], binding_mode: str):
    if not results:
        return

    expected_from_env = os.getenv("EXPECTED_ROW_COUNT")
    if expected_from_env:
        expected_count = int(expected_from_env)
    elif binding_mode == "execute":
        expected_count = 1
    else:
        expected_count = results[0]["row_count"]

    for index, result in enumerate(results):
        actual = result["row_count"]
        if actual != expected_count:
            raise RuntimeError(
                f"Row count mismatch: iteration {index} returned {actual} rows, "
                f"expected {expected_count} rows"
            )

    print(f"✓ All {len(results)} iterations returned {expected_count} rows")


def _print_statistics(results: list[dict[str, Any]]):
    query_times = [result["query_time_s"] for result in results]
    fetch_times = [result["fetch_time_s"] for result in results]
    print("\nSummary:")
    print_timing_stats("Query", query_times)
    if any(fetch_times):
        print_timing_stats("Fetch", fetch_times)
