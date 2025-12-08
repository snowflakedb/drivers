#!/usr/bin/env python3
"""
Run official Snowflake connector tests against our PEP 249 driver
to validate compatibility
"""
import sys
import os

# Use our driver instead of official
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pep249_dbapi as snowflake_connector

# Mock the official connector module so tests import our driver
sys.modules['snowflake.connector'] = snowflake_connector
sys.modules['snowflake'] = type(sys)('snowflake')
sys.modules['snowflake'].connector = snowflake_connector

print("✅ Configured to use our PEP 249 driver instead of official driver")
print("Now run: pytest ~/repos/snowflake-connector-python/test/integ/<test_file>")
print("Example: pytest ~/repos/snowflake-connector-python/test/integ/test_connection.py -v")
