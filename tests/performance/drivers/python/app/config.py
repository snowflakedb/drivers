"""Configuration parsing and environment variable handling."""

import json
import os
import sys
import tempfile
from pathlib import Path
from test_types import TestType


class TestConfig:
    """Performance test configuration."""
    
    def __init__(self):
        self.driver_type = os.getenv("DRIVER_TYPE", "universal")
        
        test_type_str = os.getenv("TEST_TYPE", "select")
        try:
            self.test_type = TestType(test_type_str)
        except ValueError:
            print(f"ERROR: Invalid test type '{test_type_str}'. Supported types: select, put_get")
            sys.exit(1)
        
        self.sql_command = os.getenv("SQL_COMMAND")
        self.test_name = os.getenv("TEST_NAME")
        self.iterations = int(os.getenv("PERF_ITERATIONS", "1"))
        self.warmup_iterations = int(os.getenv("PERF_WARMUP_ITERATIONS", "0"))
        self.params_json = os.getenv("PARAMETERS_JSON")
        self.setup_queries_json = os.getenv("SETUP_QUERIES")
        
        if not all([self.sql_command, self.test_name, self.params_json]):
            print("ERROR: Missing required environment variables")
            sys.exit(1)
    
    def get_setup_queries(self):
        """Parse and return setup queries if provided."""
        if self.setup_queries_json:
            return json.loads(self.setup_queries_json)
        return []
    
    def parse_connection_params(self):
        """Parse connection parameters from JSON."""
        params = json.loads(self.params_json)
        conn_params = params.get("testconnection", {})

        # First check if private key file path is provided
        private_key_file = conn_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_FILE")

        # If a file path is provided, ensure it exists to avoid confusing auth errors later
        if private_key_file and not Path(private_key_file).is_file():
            print(f"ERROR: Private key file '{private_key_file}' does not exist")
            sys.exit(1)

        # If no file path, parse private key contents (array of strings) and write to temporary file
        if not private_key_file:
            private_key_contents = conn_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS", [])
            if private_key_contents:
                # Write private key to a temporary file for authentication (OS-agnostic)
                temp_dir = Path(tempfile.gettempdir())
                key_file_path = temp_dir / "perf_test_private_key.p8"
                with open(key_file_path, 'w') as f:
                    f.write("\n".join(private_key_contents))
                    f.write("\n")
                private_key_file = str(key_file_path)

        connection_params = {
            "account": conn_params.get("SNOWFLAKE_TEST_ACCOUNT") or conn_params.get("account"),
            "host": conn_params.get("SNOWFLAKE_TEST_HOST") or conn_params.get("host"),
            "user": conn_params.get("SNOWFLAKE_TEST_USER") or conn_params.get("user"),
            "database": conn_params.get("SNOWFLAKE_TEST_DATABASE") or conn_params.get("database"),
            "schema": conn_params.get("SNOWFLAKE_TEST_SCHEMA") or conn_params.get("schema"),
            "warehouse": conn_params.get("SNOWFLAKE_TEST_WAREHOUSE") or conn_params.get("warehouse"),
            "role": conn_params.get("SNOWFLAKE_TEST_ROLE") or conn_params.get("role"),
        }

        # Add JWT authentication parameters if private key is provided
        if private_key_file:
            connection_params["authenticator"] = "SNOWFLAKE_JWT"
            connection_params["private_key_file"] = private_key_file

        # Process TLS/certificate verification parameters (for WireMock testing)
        _process_tls_parameters(conn_params, connection_params, self.driver_type)

        return connection_params
    
    def get_driver_label(self):
        """Get human-readable driver label."""
        return "PYTHON" if self.driver_type == "universal" else "PYTHON (Old)"


def _process_tls_parameters(conn_params, connection_params, driver_type):
    """
    Process TLS/certificate verification parameters for WireMock testing.
    
    This function handles verification parameters differently for old vs universal drivers:
    
    For OLD driver:
        - Sets insecure_mode=True to disable OCSP checks (partial SSL bypass)
        - The old driver also requires SSL patching in connection.py via _disable_ssl_verification_for_wiremock() for full SSL bypass.
    
    For UNIVERSAL driver:
        - Passes verify_certificates and verify_hostname directly to sf_core
    
    Args:
        conn_params: Raw connection parameters from JSON
        connection_params: Processed connection parameters dict to update
        driver_type: Type of driver being used ("universal" or "old")
    """
    if driver_type == "universal":
        _process_tls_for_universal(conn_params, connection_params)
    else:
        _process_tls_for_old(conn_params, connection_params)


def _process_tls_for_universal(conn_params, connection_params):
    """
    Process TLS parameters for universal driver.
    
    Passes verify_certificates and verify_hostname directly to sf_core.
    """
    if "verify_certificates" in conn_params:
        connection_params["verify_certificates"] = conn_params["verify_certificates"]
    
    if "verify_hostname" in conn_params:
        connection_params["verify_hostname"] = conn_params["verify_hostname"]


def _process_tls_for_old(conn_params, connection_params):
    """
    Process TLS parameters for old driver.
    
    Converts verify_certificates to insecure_mode (inverted logic).
    The old driver also requires SSL patching in connection.py for full SSL bypass.
    """
    if "verify_certificates" in conn_params:
        verify_value = conn_params["verify_certificates"]
        
        # Convert string to boolean for insecure_mode (inverted)
        if isinstance(verify_value, str):
            should_disable_verify = (verify_value.lower() == "false")
        else:
            should_disable_verify = not verify_value
        
        connection_params["insecure_mode"] = should_disable_verify

