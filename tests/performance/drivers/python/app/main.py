import sys

from config import TestConfig
from connection import create_connection, get_server_version, execute_setup_queries
from put_execution import execute_put_get_test
from query_execution import execute_fetch_test
from results import write_csv_results, write_run_metadata
from test_types import TestType

TEST_EXECUTORS = {
    TestType.SELECT: execute_fetch_test,
    TestType.PUT_GET: execute_put_get_test,
}


def execute_test(test_type: TestType, cursor, sql_command: str, warmup_iterations: int, iterations: int):
    """Execute test using registered executor for the given test type."""
    executor = TEST_EXECUTORS.get(test_type)
    if not executor:
        raise ValueError(f"Unknown test type: {test_type}. Supported: {list(TEST_EXECUTORS.keys())}")
    
    return executor(cursor, sql_command, warmup_iterations, iterations)


def main():
    config = TestConfig()
    conn_params = config.parse_connection_params()
    setup_queries = config.get_setup_queries()
    
    try:
        conn, driver_version = create_connection(config.driver_type, conn_params)
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        sys.exit(1)
    
    cursor = conn.cursor()
    
    server_version = get_server_version(cursor)
    
    try:
        execute_setup_queries(cursor, setup_queries)
    except Exception:
        cursor.close()
        conn.close()
        sys.exit(1)
    
    results = execute_test(
        config.test_type, 
        cursor, 
        config.sql_command, 
        config.warmup_iterations, 
        config.iterations
    )
    
    cursor.close()
    conn.close()

    filename = write_csv_results(results, config.test_name, config.driver_type, config.test_type)
    write_run_metadata(config.driver_type, driver_version, server_version)
    
    print(f"\n✓ Complete → {filename}")


if __name__ == "__main__":
    main()
