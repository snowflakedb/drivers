import os
import json
from typing import Dict, Any


def get_test_parameters() -> Dict[str, Any]:
    """Get test connection parameters from environment or parameters file.
    
    Returns:
        Dictionary containing test connection parameters from parameters.json
        or environment variables as fallback.
    """
    # First try environment variable
    parameter_path = os.environ.get("PARAMETER_PATH")
    if parameter_path and os.path.exists(parameter_path):
        with open(parameter_path) as f:
            parameters = json.load(f)
            return parameters.get("testconnection", {})
    
    # Fallback to default test parameters (for local testing)
    env_vars = [
        "SNOWFLAKE_TEST_ACCOUNT",
        "SNOWFLAKE_TEST_USER",
        "SNOWFLAKE_TEST_PASSWORD",
        "SNOWFLAKE_TEST_DATABASE",
        "SNOWFLAKE_TEST_SCHEMA",
        "SNOWFLAKE_TEST_WAREHOUSE",
        "SNOWFLAKE_TEST_ROLE",
        "SNOWFLAKE_TEST_SERVER_URL",
        "SNOWFLAKE_TEST_HOST",
        "SNOWFLAKE_TEST_PORT",
        "SNOWFLAKE_TEST_PROTOCOL",
        "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS",
        "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD",
    ]
    return {k: os.environ.get(k) for k in env_vars}

