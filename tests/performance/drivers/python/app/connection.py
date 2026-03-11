"""Connection management and connector selection."""
from importlib.metadata import version, PackageNotFoundError
from pathlib import Path
import ssl
import os


def create_connection(driver_type, conn_params):
    """Create and return a connection."""
    connector = _get_connector(driver_type)
    driver_version = _get_driver_version(driver_type)
    
    # Disable SSL verification for old Python driver when using WireMock proxy
    if os.getenv("HTTPS_PROXY") and driver_type == "old":
        _disable_ssl_verification_for_wiremock(connector)
    
    conn = connector.connect(**conn_params)
    
    return conn, driver_version


def get_server_version(cursor):
    """Query and return the server version."""
    try:
        cursor.execute("SELECT CURRENT_VERSION() AS VERSION")
        server_version_result = cursor.fetchone()
        if server_version_result:
            return server_version_result[0]
        else:
            print("⚠️  Warning: Could not retrieve server version (empty result)")
            return "UNKNOWN"
    except Exception as err:
        print(f"⚠️  Warning: Could not retrieve server version: {err}")
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


def _load_from_sources():
    """Load old driver from old_driver_src directory."""
    import sys
    
    legacy_path = Path(__file__).parent / "old_driver_src"
    if not legacy_path.exists():
        raise ImportError(f"Old driver directory not found: {legacy_path}")
    
    legacy_path_str = str(legacy_path.absolute())
    if legacy_path_str not in sys.path:
        sys.path.insert(0, legacy_path_str)
    
    import snowflake.connector as old_connector
    
    return old_connector


def _get_connector(driver_type):
    """Get the appropriate connector module based on driver type."""
    if driver_type == "old":
        return _load_from_sources()
    else:  # universal
        from snowflake import connector
        return connector


def _get_driver_version(driver_type):
    """Get driver version from package metadata."""
    try:
        if driver_type == "old":
            return version("snowflake-connector-python")
        else:  # universal
            return version("snowflake-connector-python-ud")
    except PackageNotFoundError as err:
        print(f"⚠️  Warning: Could not determine driver version: {err}")
        return "UNKNOWN"


def _disable_ssl_verification_for_wiremock(connector):
    """
    Disable SSL verification for old Python driver when using WireMock proxy.
    
    Args:
        connector: The connector module (loaded from old_driver_src for old driver)
    """
    # Import ssl_wrap_socket from the connector package
    if hasattr(connector, 'ssl_wrap_socket'):
        ssl_wrap = connector.ssl_wrap_socket
    else:
        import importlib
        module_name = connector.__name__ + '.ssl_wrap_socket'
        ssl_wrap = importlib.import_module(module_name)

    def no_verify_context(*args, **kwargs):
        ctx = ssl.create_default_context()
        ctx.check_hostname = False
        ctx.verify_mode = ssl.CERT_NONE
        return ctx
    
    # Replace the SSL context builder
    ssl_wrap._build_context_with_partial_chain = no_verify_context
