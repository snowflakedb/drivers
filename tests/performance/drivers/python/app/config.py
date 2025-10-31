"""Configuration parsing and environment variable handling."""

import json
import os
import sys
import tempfile
from pathlib import Path


class TestConfig:
    """Performance test configuration."""
    
    def __init__(self):
        self.driver_type = os.getenv("DRIVER_TYPE", "universal")
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
        
        # Parse private key contents (array of strings) and write to temporary file
        private_key_contents = conn_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS", [])
        private_key_file = None
        
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
        
        return connection_params
    
    def get_driver_label(self):
        """Get human-readable driver label."""
        return "PYTHON" if self.driver_type == "universal" else "PYTHON (Old)"

