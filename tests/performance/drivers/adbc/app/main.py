import sys

from config import TestConfig
from connection import connect
from query_execution import execute_fetch_test
from results import write_csv_results, write_memory_timeline, write_run_metadata

_DRIVER = "adbc"


def main():
    config = TestConfig()
    try:
        conn = connect(config.params_json)
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        sys.exit(1)

    cursor = conn.cursor()
    try:
        for query in config.get_setup_queries():
            cursor.execute(query)
        server_version = _probe_server_version(cursor)
        results, memory_timeline = execute_fetch_test(
            cursor,
            config.sql_command,
            config.warmup_iterations,
            config.iterations,
        )
    except Exception as e:
        print(f"❌ Test execution failed: {e}")
        sys.exit(1)
    finally:
        cursor.close()
        conn.close()

    write_run_metadata(server_version, _DRIVER, _driver_version())
    filename = write_csv_results(results, config.test_name, _DRIVER)
    timeline_filename = write_memory_timeline(memory_timeline, config.test_name, _DRIVER)
    print(f"\n✓ Complete → {filename}")
    if timeline_filename:
        print(f"✓ Memory timeline → {timeline_filename}")


def _probe_server_version(cursor) -> str:
    try:
        cursor.execute("SELECT CURRENT_VERSION() AS VERSION")
        row = cursor.fetchone()
        if row:
            return str(row[0])
    except Exception as err:
        print(f"Warning: Could not retrieve server version: {err}")
    return "UNKNOWN"


def _driver_version() -> str:
    try:
        import adbc_driver_snowflake

        return getattr(adbc_driver_snowflake, "__version__", "adbc-snowflake")
    except Exception:
        return "adbc-snowflake"


if __name__ == "__main__":
    main()
