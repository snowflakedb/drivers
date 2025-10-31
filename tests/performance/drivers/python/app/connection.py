"""Connection management and connector selection."""

import time
from importlib.metadata import version, PackageNotFoundError


def get_connector(driver_type):
    """Get the appropriate connector module based on driver type."""
    if driver_type == "old":
        import snowflake.connector
        return snowflake.connector
    else:  # universal
        import pep249_dbapi
        return pep249_dbapi


def get_driver_version(driver_type):
    """Get driver version from package metadata."""
    try:
        if driver_type == "old":
            return version("snowflake-connector-python")
        else:  # universal
            return version("snowflake-pep249-dbapi")
    except PackageNotFoundError:
        return "UNKNOWN"


def create_connection(driver_type, conn_params):
    """Create and return a connection."""
    connector = get_connector(driver_type)
    driver_version = get_driver_version(driver_type)
    
    conn = connector.connect(**conn_params)
    
    return conn, driver_version


def get_server_version(cursor):
    """Query and return the server version."""
    try:
        cursor.execute("SELECT CURRENT_VERSION() AS VERSION")
        server_version_result = cursor.fetchone()
        server_version = server_version_result[0] if server_version_result else "UNKNOWN"
        return server_version
    except Exception:
        return "UNKNOWN"


def execute_setup_queries(cursor, setup_queries):
    """Execute setup queries before test runs."""
    if not setup_queries:
        return
    
    print(f"\n=== Executing Setup Queries ({len(setup_queries)} queries) ===")
    for i, query in enumerate(setup_queries, 1):
        print(f"  Setup query {i}: {query}")
        try:
            cursor.execute(query)
            # Consume any results to ensure the query completes
            try:
                cursor.fetchall()
            except Exception:
                pass  # Some queries don't return results
        except Exception as e:
            print(f"\n❌ ERROR: Setup query {i} failed: {query}")
            print(f"   Error: {e}")
            raise
    
    print("✓ Setup queries completed")

