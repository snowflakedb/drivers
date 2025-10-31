import sys

from config import TestConfig
from connection import create_connection, get_server_version, execute_setup_queries
from execution import run_warmup, run_test_iterations, print_statistics
from results import write_csv_results, write_run_metadata


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
    
    print("\n=== Executing Test Query ===")

    run_warmup(cursor, config.sql_command, config.warmup_iterations)
    results = run_test_iterations(cursor, config.sql_command, config.iterations)
    
    cursor.close()
    conn.close()

    filename = write_csv_results(results, config.test_name, config.driver_type)
    write_run_metadata(config.driver_type, driver_version, server_version)
    
    print_statistics(results)
    print(f"\n✓ Complete → {filename}")


if __name__ == "__main__":
    main()
