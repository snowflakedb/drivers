import sys

import requests

from auth import load_connection, resolve_token
from client import auth_headers
from config import TestConfig
from query_execution import execute_fetch_test
from results import write_csv_results, write_memory_timeline, write_run_metadata

_DRIVER = "sqlapi"
_DRIVER_VERSION = "sqlapi-v2"


def main():
    config = TestConfig()
    conn = load_connection(config.params_json)
    token, token_type = resolve_token(conn)
    headers = auth_headers(token, token_type)
    print(f"SQL API auth: {token_type}")
    print(f"SQL API host: {conn.get('host')}")

    session = requests.Session()
    try:
        server_version = _probe_server_version(session, conn["host"], headers, conn)
        results, memory_timeline = execute_fetch_test(
            session,
            config.sql_command,
            config.warmup_iterations,
            config.iterations,
            host=conn["host"],
            headers=headers,
            conn=conn,
            result_format=config.result_format,
        )
    except Exception as e:
        print(f"❌ Test execution failed: {e}")
        sys.exit(1)
    finally:
        session.close()

    write_run_metadata(server_version, _DRIVER, _DRIVER_VERSION)
    filename = write_csv_results(results, config.test_name, _DRIVER)
    timeline_filename = write_memory_timeline(memory_timeline, config.test_name, _DRIVER)
    print(f"\n✓ Complete → {filename}")
    if timeline_filename:
        print(f"✓ Memory timeline → {timeline_filename}")


def _probe_server_version(session, host, headers, conn) -> str:
    """Untimed CURRENT_VERSION() so BenchDash tags match other drivers."""
    from query_execution import _submit_until_ready
    from client import statement_body

    try:
        payload = _submit_until_ready(
            session,
            host,
            headers,
            statement_body(
                "SELECT CURRENT_VERSION() AS VERSION",
                database=conn.get("database"),
                schema=conn.get("schema"),
                warehouse=conn.get("warehouse"),
                role=conn.get("role"),
                result_format="json",
            ),
        )
        data = payload.get("data") or []
        if data and data[0]:
            return str(data[0][0])
    except Exception as err:
        print(f"Warning: Could not retrieve server version: {err}")
    return "UNKNOWN"


if __name__ == "__main__":
    main()
