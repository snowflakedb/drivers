"""Connection management and connector selection."""
import asyncio
import inspect
from importlib.metadata import version, PackageNotFoundError


def _await(value):
    """Resolve a coroutine if the cursor/connection is aio; otherwise return as-is."""
    if inspect.isawaitable(value):
        return asyncio.get_event_loop().run_until_complete(value)
    return value


def create_connection(driver_type, conn_params):
    """Create and return a connection."""
    connector = _get_connector()
    driver_version = _get_driver_version(driver_type)
    conn = connector.connect(**conn_params)
    return conn, driver_version


def create_aio_connection(conn_params):
    """Create a snowflake.connector.aio connection on a dedicated event loop."""
    from snowflake.connector.aio import connect

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    conn = loop.run_until_complete(connect(**conn_params))
    conn._perf_event_loop = loop
    return conn, _get_driver_version("universal")


def get_server_version(cursor):
    """Query and return the server version."""
    try:
        _await(cursor.execute("SELECT CURRENT_VERSION() AS VERSION"))
        server_version_result = _await(cursor.fetchone())
        if server_version_result:
            return server_version_result[0]
        else:
            print("Warning: Could not retrieve server version (empty result)")
            return "UNKNOWN"
    except Exception as err:
        print(f"Warning: Could not retrieve server version: {err}")
        return "UNKNOWN"


def execute_setup_queries(cursor, setup_queries):
    """Execute setup queries before test runs."""
    if not setup_queries:
        return
    
    print(f"\n=== Executing Setup Queries ({len(setup_queries)} queries) ===")
    for i, query in enumerate(setup_queries, 1):
        print(f"  Setup query {i}: {query}")
        try:
            _await(cursor.execute(query))
            try:
                _await(cursor.fetchall())
            except Exception:
                pass
        except Exception as e:
            print(f"\nERROR: Setup query {i} failed: {query}")
            print(f"   Error: {e}")
            raise
    
    print("Setup queries completed")


def close_connection(cursor, conn):
    _await(cursor.close())
    _await(conn.close())
    loop = getattr(conn, "_perf_event_loop", None)
    if loop is not None and not loop.is_closed():
        loop.close()


def _get_connector():
    """Get the snowflake connector module (whichever is installed in this image)."""
    from snowflake import connector
    return connector


def _get_driver_version(driver_type):
    """Get driver version from package metadata."""
    try:
        if driver_type == "old":
            return version("snowflake-connector-python")
        else:
            return version("snowflake-connector-python")
    except PackageNotFoundError as err:
        print(f"Warning: Could not determine driver version: {err}")
        return "UNKNOWN"
