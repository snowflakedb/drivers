"""SQL API SELECT execution: query_s = POST (+ 202 poll); fetch_s = partition GET + decode.

No result CSV is written inside the timer.
"""

from __future__ import annotations

import gzip
import os
import time

import requests

from client import (
    ARROW_FORMAT,
    assert_result_format,
    assert_row_count,
    is_arrow_format,
    json_row_count,
    metadata_num_rows,
    partition_count,
    partition_headers,
    partition_url,
    partitions_to_fetch,
    sqlapi_format,
    statement_body,
    statement_url,
    statements_url,
)
from common import get_peak_rss_mb, print_timing_stats, run_test_iterations, run_warmup
from resource_monitor import ResourceMonitor

_POLL_SLEEP_S = 0.1
_HTTP_TIMEOUT_S = 3600


def execute_fetch_test(
    session: requests.Session,
    sql_command: str,
    warmup_iterations: int,
    iterations: int,
    *,
    host: str,
    headers: dict[str, str],
    conn: dict,
    result_format: str,
):
    print("\n=== Executing SELECT Test (SQL API) ===")
    print(f"Query: {sql_command}")
    print(f"Format: {sqlapi_format(result_format)}")

    def execute(_session, sql):
        return _execute_query(
            _session,
            sql,
            host=host,
            headers=headers,
            conn=conn,
            result_format=result_format,
        )

    run_warmup(execute, session, sql_command, warmup_iterations)
    monitor = ResourceMonitor(interval_s=0.1)
    monitor.start()
    results = run_test_iterations(execute, session, sql_command, iterations)
    memory_timeline = monitor.stop()
    _validate_row_counts(results)
    query_times = [r["query_time_s"] for r in results]
    fetch_times = [r["fetch_time_s"] for r in results]
    print("\nSummary:")
    print_timing_stats("Query", query_times)
    print_timing_stats("Fetch", fetch_times)
    print(f"  Memory timeline: {len(memory_timeline)} samples collected")
    return results, memory_timeline


def _execute_query(session, sql, *, host, headers, conn, result_format):
    body = statement_body(
        sql,
        database=conn.get("database"),
        schema=conn.get("schema"),
        warehouse=conn.get("warehouse"),
        role=conn.get("role"),
        result_format=result_format,
    )
    query_start = time.time()
    payload = _submit_until_ready(session, host, headers, body)
    query_time = time.time() - query_start

    cpu_start = time.process_time()
    fetch_start = time.time()
    row_count = _fetch_partitions(session, host, headers, payload, result_format)
    fetch_time = time.time() - fetch_start
    expected = metadata_num_rows(payload)
    env_expected = os.getenv("EXPECTED_ROW_COUNT")
    if env_expected:
        expected = int(env_expected)
    assert_row_count(row_count, expected, "SQL API")

    return {
        "timestamp": int(time.time() * 1000),
        "query_time_s": query_time,
        "fetch_time_s": fetch_time,
        "row_count": row_count,
        "cpu_time_s": time.process_time() - cpu_start,
        "peak_rss_mb": get_peak_rss_mb(),
    }


def _submit_until_ready(session, host, headers, body) -> dict:
    resp = session.post(
        statements_url(host), headers=headers, json=body, timeout=_HTTP_TIMEOUT_S
    )
    deadline = time.time() + _HTTP_TIMEOUT_S
    while resp.status_code == 202:
        if time.time() > deadline:
            raise TimeoutError("SQL API statement stayed 202 past timeout")
        handle = resp.json().get("statementHandle")
        if not handle:
            raise RuntimeError(f"SQL API 202 without statementHandle: {resp.text[:500]}")
        time.sleep(_POLL_SLEEP_S)
        resp = session.get(
            statement_url(host, handle), headers=headers, timeout=_HTTP_TIMEOUT_S
        )
    if resp.status_code != 200:
        raise RuntimeError(
            f"SQL API POST failed ({resp.status_code}): {resp.text[:2000]}"
        )
    payload = resp.json()
    if "resultSetMetaData" not in payload and "statementHandle" not in payload:
        raise RuntimeError(f"SQL API POST returned no result metadata: {resp.text[:2000]}")
    requested = (body.get("resultSetMetaData") or {}).get("format")
    if requested:
        assert_result_format(payload, requested)
    return payload


def _fetch_partitions(session, host, headers, payload, result_format) -> int:
    fmt = sqlapi_format(result_format)
    n = partition_count(payload)
    rows = 0 if fmt == ARROW_FORMAT else json_row_count(payload)

    urls = [
        partition_url(host, payload.get("statementStatusUrl", ""), i)
        for i in partitions_to_fetch(n)
    ]
    if not urls:
        return rows

    part_headers = partition_headers(headers, fmt)
    for url in urls:
        rows += _get_partition_rows(session, url, part_headers, fmt)
    return rows


def _get_partition_rows(session, url, headers, fmt) -> int:
    resp = session.get(url, headers=headers, timeout=_HTTP_TIMEOUT_S)
    if resp.status_code != 200:
        raise RuntimeError(f"SQL API GET {url} failed ({resp.status_code}): {resp.text[:1000]}")
    content_type = (resp.headers.get("Content-Type") or "").split(";")[0].strip().lower()
    if is_arrow_format(fmt):
        if "json" in content_type:
            raise RuntimeError(
                f"SQL API GET expected Arrow, got Content-Type={content_type!r}: {resp.text[:500]}"
            )
        return _arrow_row_count(resp.content)
    return json_row_count(resp.json())


def _maybe_gunzip(content: bytes) -> bytes:
    data = content
    while len(data) >= 2 and data[0] == 0x1F and data[1] == 0x8B:
        data = gzip.decompress(data)
    return data


def _arrow_row_count(content: bytes) -> int:
    import pyarrow as pa

    data = _maybe_gunzip(content)
    if not data:
        raise RuntimeError("SQL API Arrow partition body is empty")
    buf = pa.BufferReader(data)
    try:
        return pa.ipc.open_stream(buf).read_all().num_rows
    except pa.ArrowInvalid as err:
        preview = data[:120]
        raise RuntimeError(
            f"SQL API Arrow decode failed ({err}); "
            f"len={len(data)} prefix={preview!r}"
        ) from err


def _validate_row_counts(results):
    if not results:
        return
    expected = int(os.getenv("EXPECTED_ROW_COUNT") or results[0]["row_count"])
    if expected == 0:
        raise RuntimeError("Row count baseline is 0")
    for i, result in enumerate(results):
        if result["row_count"] != expected:
            raise RuntimeError(
                f"Row count mismatch: iteration {i} returned {result['row_count']}, "
                f"expected {expected}"
            )
    print(f"✓ All {len(results)} iterations returned {expected} rows")
