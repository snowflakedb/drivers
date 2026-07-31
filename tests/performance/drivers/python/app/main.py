import sys
import os

from config import TestConfig
from connection import create_connection, get_server_version, execute_setup_queries
from put_execution import execute_put_get_test
from query_execution import execute_fetch_test
from results import write_csv_results, write_memory_timeline, write_run_metadata
from test_types import TestType

TEST_EXECUTORS = {
    TestType.SELECT: execute_fetch_test,
    TestType.PUT_GET: execute_put_get_test,
}


def execute_test(test_type: TestType, cursor, sql_command: str, warmup_iterations: int, iterations: int, fetch_mode: str = "fetchmany"):
    """Execute test using registered executor for the given test type."""
    executor = TEST_EXECUTORS.get(test_type)
    if not executor:
        raise ValueError(f"Unknown test type: {test_type}. Supported: {list(TEST_EXECUTORS.keys())}")

    if test_type == TestType.SELECT:
        return executor(cursor, sql_command, warmup_iterations, iterations, fetch_mode)
    return executor(cursor, sql_command, warmup_iterations, iterations)


def _run_cold_start(config):
    """Run cold-start test: each iteration is a fresh subprocess that imports, connects, and runs SELECT 1."""
    import json
    import subprocess
    import time
    from pathlib import Path

    conn_params = config.parse_connection_params()
    child_env = os.environ.copy()
    child_env["CONNECTION_PARAMS_JSON"] = json.dumps(conn_params)
    child_env["DRIVER_TYPE"] = config.driver_type

    child_script = str(Path(__file__).with_name("cold_start_execution.py"))
    subdir = "_record" if config.test_name.endswith("_record") else config.test_name
    results_dir = Path("/results") / config.driver_type / subdir
    results_dir.mkdir(parents=True, exist_ok=True)

    timestamp = int(time.time())
    filename = results_dir / f"{config.test_name}_python_{config.driver_type}_{timestamp}.csv"

    rows = []

    print(f"\n=== Cold-Start Test ({config.iterations} iterations) ===")
    for i in range(config.iterations):
        label = f"iter {i + 1}/{config.iterations}"
        proc = subprocess.run(
            [sys.executable, child_script],
            env=child_env,
            capture_output=True,
            text=True,
            timeout=120,
        )
        if proc.returncode != 0:
            print(f"  [{label}] FAILED (exit {proc.returncode})")
            print(proc.stderr)
            sys.exit(1)
        line = proc.stdout.strip()
        print(f"  [{label}] {line}")
        rows.append(line)

    with open(filename, "w", newline="") as f:
        f.write("timestamp_ms,e2e_s,load_s,connect_s,select1_s,cpu_time_s,peak_rss_mb\n")
        for row in rows:
            f.write(row + "\n")

    # Write run metadata (driver version requires a quick import)
    from connection import _get_driver_version
    driver_version = _get_driver_version(config.driver_type)
    write_run_metadata(config.driver_type, driver_version, "N/A")

    print(f"\n✓ Complete → {filename}")


def main():
    config = TestConfig()

    if config.test_type == TestType.COLD_START:
        _run_cold_start(config)
        return

    conn_params = config.parse_connection_params()
    setup_queries = config.get_setup_queries()
    
    try:
        conn, driver_version = create_connection(config.driver_type, conn_params)
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        sys.exit(1)
    
    cursor = conn.cursor()
    
    try:
        execute_setup_queries(cursor, setup_queries)
    except Exception as e:
        print(f"❌ Setup query failed: {e}")
        cursor.close()
        conn.close()
        sys.exit(1)
    
    try:
        results, memory_timeline = execute_test(
            config.test_type,
            cursor,
            config.sql_command,
            config.warmup_iterations,
            config.iterations,
            config.fetch_mode,
        )
    except Exception as e:
        print(f"❌ Test execution failed: {e}")
        cursor.close()
        conn.close()
        sys.exit(1)
    
    # In replay mode, skip server version query and use N/A
    if os.getenv("WIREMOCK_REPLAY") == "true":
        server_version = "N/A"
    else:
        server_version = get_server_version(cursor)
    write_run_metadata(config.driver_type, driver_version, server_version or "UNKNOWN")
        
    cursor.close()
    conn.close()

    filename = write_csv_results(results, config.test_name, config.driver_type, config.test_type)
    timeline_filename = write_memory_timeline(memory_timeline, config.test_name, config.driver_type)
    
    print(f"\n✓ Complete → {filename}")
    if timeline_filename:
        print(f"✓ Memory timeline → {timeline_filename}")


if __name__ == "__main__":
    main()
