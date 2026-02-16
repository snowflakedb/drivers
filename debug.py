#!/usr/bin/env python3
"""
Debug script to test ALTER SESSION cache updates.

This script replicates the test_get_parameter_after_alter_session test
to debug the session parameters cache behavior.
"""

import json
import os
import sys

# Add the python wrapper to the path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'python', 'src'))

import snowflake.connector

def main():
    print("=== Starting debug script for ALTER SESSION cache test ===\n")

    # Load connection parameters from parameters.json
    params_file = os.path.join(os.path.dirname(__file__), 'parameters.json')

    if not os.path.exists(params_file):
        print(f"ERROR: Could not find {params_file}")
        sys.exit(1)

    with open(params_file) as f:
        params = json.load(f)

    test_conn = params.get('testconnection', {})
    account = test_conn.get('SNOWFLAKE_TEST_ACCOUNT')
    host = test_conn.get('SNOWFLAKE_TEST_HOST')
    user = test_conn.get('SNOWFLAKE_TEST_USER')
    password = test_conn.get('SNOWFLAKE_TEST_PASSWORD')
    warehouse = test_conn.get('SNOWFLAKE_TEST_WAREHOUSE')
    database = test_conn.get('SNOWFLAKE_TEST_DATABASE')
    schema = test_conn.get('SNOWFLAKE_TEST_SCHEMA')
    role = test_conn.get('SNOWFLAKE_TEST_ROLE')

    if not all([account, host, user, password]):
        print("ERROR: Missing required connection parameters in parameters.json")
        sys.exit(1)

    print(f"Connecting to account: {account}")
    print(f"Host: {host}")
    print(f"User: {user}")
    print(f"Warehouse: {warehouse}")
    print(f"Database: {database}")
    print(f"Schema: {schema}")
    print(f"Role: {role}\n")

    # Create connection
    conn = snowflake.connector.connect(
        account=account,
        host=host,
        user=user,
        password=password,
        warehouse=warehouse,
        database=database,
        schema=schema,
        role=role,
    )

    print("✓ Connection established\n")

    # Create cursor and execute ALTER SESSION
    cursor = conn.cursor()

    # First, check the initial TIMEZONE value (should come from server)
    print("--- Testing with TIMEZONE (parameter returned by server) ---")
    print("Getting initial TIMEZONE from session parameter cache...")
    initial_timezone = conn._get_session_parameter('TIMEZONE')
    print(f"Initial TIMEZONE value: {initial_timezone!r}\n")

    # Change TIMEZONE to a different value
    print("Executing: ALTER SESSION SET TIMEZONE = 'UTC'")
    cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC'")
    print("✓ ALTER SESSION executed\n")

    # Get parameter value from cache
    print("Getting TIMEZONE from session parameter cache...")
    value = conn._get_session_parameter('TIMEZONE')
    print(f"Value from cache after ALTER SESSION: {value!r}\n")

    # Test result
    if value == 'UTC':
        print("✅ SUCCESS: Cache was updated correctly for TIMEZONE!")
    else:
        print(f"❌ FAILURE: Expected 'UTC', got {value!r}")
        sys.exit(1)

    # Additional test: Update the parameter again to a different timezone
    print("\n--- Testing TIMEZONE parameter update again ---")
    print("Executing: ALTER SESSION SET TIMEZONE = 'Europe/London'")
    cursor.execute("ALTER SESSION SET TIMEZONE = 'Europe/London'")
    print("✓ ALTER SESSION executed\n")

    print("Getting TIMEZONE from session parameter cache...")
    value = conn._get_session_parameter('TIMEZONE')
    print(f"Value from cache: {value!r}\n")

    if value == 'Europe/London':
        print("✅ SUCCESS: Cache was updated correctly on second TIMEZONE change!")
    else:
        print(f"❌ FAILURE: Expected 'Europe/London', got {value!r}")
        sys.exit(1)

    # Now test QUERY_TAG (not returned by server)
    print("\n--- Testing with QUERY_TAG (NOT returned by server) ---")
    print("Executing: ALTER SESSION SET QUERY_TAG = 'test_tag_123'")
    cursor.execute("ALTER SESSION SET QUERY_TAG = 'test_tag_123'")
    print("✓ ALTER SESSION executed\n")

    print("Getting QUERY_TAG from session parameter cache...")
    value = conn._get_session_parameter('QUERY_TAG')
    print(f"Value from cache: {value!r}\n")

    if value == 'test_tag_123':
        print("✅ SUCCESS: Cache was updated correctly for QUERY_TAG!")
    else:
        print(f"❌ FAILURE: Expected 'test_tag_123', got {value!r}")
        sys.exit(1)

    # Clean up
    cursor.close()
    conn.close()

    print("\n=== All tests passed! ===")

if __name__ == '__main__':
    main()
