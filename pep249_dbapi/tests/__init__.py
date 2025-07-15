# Tests for PEP 249 Database API 2.0 Implementation 
from pep249_dbapi import connect


def create_connection():
    """Create a test connection with parameters from parameters.json."""
    import os
    import json

    # Get parameters file path from environment variable
    assert "PARAMETER_PATH" in os.environ
    parameter_path = os.environ["PARAMETER_PATH"]

    # Read and parse parameters.json
    with open(parameter_path) as f:
        parameters = json.load(f)
        test_params = parameters["testconnection"]

    # Create connection with test parameters
    conn = connect(
        account=test_params.get("SNOWFLAKE_TEST_ACCOUNT"),
        user=test_params.get("SNOWFLAKE_TEST_USER"),
        password=test_params.get("SNOWFLAKE_TEST_PASSWORD"),
        database=test_params.get("SNOWFLAKE_TEST_DATABASE"),
        schema=test_params.get("SNOWFLAKE_TEST_SCHEMA"),
        warehouse=test_params.get("SNOWFLAKE_TEST_WAREHOUSE"),
        role=test_params.get("SNOWFLAKE_TEST_ROLE"),
        server_url=test_params.get("SNOWFLAKE_TEST_SERVER_URL")
    )
    return conn
