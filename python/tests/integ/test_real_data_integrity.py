#!/usr/bin/env python
"""Real-world data integrity tests adapted from official Snowflake connector.

Tests database capabilities and data type handling by:
1. Creating tables with specific data types
2. Inserting random data
3. Reading back and verifying data integrity
"""

import random
import time
import decimal
import pytest
from datetime import datetime, date, time as dt_time

import pep249_dbapi


@pytest.fixture(scope="module")
def connection():
    """Create a connection using parameters.json"""
    import json
    import os
    
    parameter_path = os.environ.get("PARAMETER_PATH")
    if not parameter_path:
        pytest.skip("PARAMETER_PATH not set")
    
    with open(parameter_path) as f:
        params = json.load(f)["testconnection"]
    
    conn = pep249_dbapi.connect(
        account=params["SNOWFLAKE_TEST_ACCOUNT"],
        user=params["SNOWFLAKE_TEST_USER"],
        password=params["SNOWFLAKE_TEST_PASSWORD"],
        database=params.get("SNOWFLAKE_TEST_DATABASE"),
        schema=params.get("SNOWFLAKE_TEST_SCHEMA"),
        warehouse=params.get("SNOWFLAKE_TEST_WAREHOUSE"),
    )
    
    yield conn
    conn.close()


def check_data_integrity(connection, column_defs, table_name, generator, rows=10):
    """Generic data integrity checker.
    
    Args:
        connection: Database connection
        column_defs: List of column definitions like ["col1 INT", "col2 VARCHAR"]
        table_name: Unique table name suffix
        generator: Function(row, col) that generates test data
        rows: Number of rows to test
    """
    table = f"test_integrity_{table_name}_{random.randint(1000, 9999)}"
    cursor = connection.cursor()
    
    try:
        # Create table
        create_sql = f"CREATE OR REPLACE TABLE {table} ({', '.join(column_defs)})"
        cursor.execute(create_sql)
        
        # Generate and insert data
        insert_sql = f"INSERT INTO {table} VALUES ({', '.join(['%s'] * len(column_defs))})"
        data = [[generator(i, j) for j in range(len(column_defs))] for i in range(rows)]
        
        for row in data:
            cursor.execute(insert_sql, row)
        
        # Read back and verify
        cursor.execute(f"SELECT * FROM {table} ORDER BY 1")
        results = cursor.fetchall()
        
        assert len(results) == rows, f"Expected {rows} rows, got {len(results)}"
        
        # Sort both for comparison
        sorted_data = sorted(data)
        for result_row, data_row in zip(results, sorted_data):
            assert list(result_row) == list(data_row), \
                f"Data mismatch: {result_row} != {data_row}"
        
        print(f"✓ {table_name}: {rows} rows verified")
        
    finally:
        cursor.execute(f"DROP TABLE IF EXISTS {table}")
        cursor.close()


def test_int_type(connection):
    """Test INTEGER data type"""
    def generator(row, col):
        return row * row
    
    check_data_integrity(
        connection,
        ["col1 INT"],
        "int",
        generator
    )


def test_decimal_type(connection):
    """Test DECIMAL data type with precision and scale"""
    def generator(row, col):
        return decimal.Decimal(f"{row}.{col:02d}")
    
    check_data_integrity(
        connection,
        ["col1 DECIMAL(10,2)"],
        "decimal",
        generator
    )


def test_real_type(connection):
    """Test REAL (float32) data type"""
    def generator(row, col):
        return float(row * 3.14)
    
    check_data_integrity(
        connection,
        ["col1 REAL"],
        "real",
        generator
    )


def test_double_type(connection):
    """Test DOUBLE (float64) data type"""
    def generator(row, col):
        return float(row * 2.718281828)
    
    check_data_integrity(
        connection,
        ["col1 DOUBLE"],
        "double",
        generator
    )


def test_varchar_type(connection):
    """Test VARCHAR string type"""
    def generator(row, col):
        return f"test_string_{row}_{col}"
    
    check_data_integrity(
        connection,
        ["col1 VARCHAR"],
        "varchar",
        generator
    )


def test_boolean_type(connection):
    """Test BOOLEAN data type"""
    def generator(row, col):
        return row % 2 == 0
    
    check_data_integrity(
        connection,
        ["col1 BOOLEAN"],
        "boolean",
        generator
    )


def test_date_type(connection):
    """Test DATE data type"""
    def generator(row, col):
        return date(2024, 1 + (row % 12), 1 + (col % 28))
    
    check_data_integrity(
        connection,
        ["col1 DATE"],
        "date",
        generator
    )


def test_multiple_columns(connection):
    """Test table with multiple data types"""
    def generator(row, col):
        if col == 0:  # INT
            return row * 100
        elif col == 1:  # VARCHAR
            return f"row_{row}"
        elif col == 2:  # DOUBLE
            return float(row * 1.5)
        elif col == 3:  # BOOLEAN
            return row % 2 == 0
    
    check_data_integrity(
        connection,
        [
            "id INT",
            "name VARCHAR",
            "value DOUBLE",
            "active BOOLEAN"
        ],
        "multi",
        generator
    )


def test_null_values(connection):
    """Test NULL handling"""
    table = f"test_nulls_{random.randint(1000, 9999)}"
    cursor = connection.cursor()
    
    try:
        cursor.execute(f"""
            CREATE OR REPLACE TABLE {table} (
                id INT,
                nullable_string VARCHAR,
                nullable_int INT
            )
        """)
        
        # Insert rows with NULLs
        cursor.execute(f"INSERT INTO {table} VALUES (1, 'hello', 100)")
        cursor.execute(f"INSERT INTO {table} VALUES (2, NULL, 200)")
        cursor.execute(f"INSERT INTO {table} VALUES (3, 'world', NULL)")
        cursor.execute(f"INSERT INTO {table} VALUES (4, NULL, NULL)")
        
        cursor.execute(f"SELECT * FROM {table} ORDER BY id")
        results = cursor.fetchall()
        
        assert len(results) == 4
        assert results[0] == (1, 'hello', 100)
        assert results[1] == (2, None, 200)
        assert results[2] == (3, 'world', None)
        assert results[3] == (4, None, None)
        
        print("✓ NULL values: verified")
        
    finally:
        cursor.execute(f"DROP TABLE IF EXISTS {table}")
        cursor.close()


def test_large_dataset(connection):
    """Test with larger dataset (100 rows)"""
    def generator(row, col):
        if col == 0:
            return row
        elif col == 1:
            return f"data_{row:04d}"
        elif col == 2:
            return float(row * 1.23456)
    
    check_data_integrity(
        connection,
        ["id INT", "name VARCHAR", "value DOUBLE"],
        "large",
        generator,
        rows=100
    )


def test_edge_case_numbers(connection):
    """Test edge cases for numeric types"""
    table = f"test_edges_{random.randint(1000, 9999)}"
    cursor = connection.cursor()
    
    try:
        cursor.execute(f"""
            CREATE OR REPLACE TABLE {table} (
                id INT,
                big_int NUMBER(38,0),
                small_decimal DECIMAL(5,2),
                zero_value INT
            )
        """)
        
        # Test edge values
        cursor.execute(f"INSERT INTO {table} VALUES (1, 999999999999999999, 999.99, 0)")
        cursor.execute(f"INSERT INTO {table} VALUES (2, -999999999999999999, -999.99, 0)")
        cursor.execute(f"INSERT INTO {table} VALUES (3, 0, 0.01, 0)")
        
        cursor.execute(f"SELECT * FROM {table} ORDER BY id")
        results = cursor.fetchall()
        
        assert len(results) == 3
        assert results[0][1] == 999999999999999999
        assert results[1][1] == -999999999999999999
        assert results[2][2] == decimal.Decimal('0.01')
        
        print("✓ Edge case numbers: verified")
        
    finally:
        cursor.execute(f"DROP TABLE IF EXISTS {table}")
        cursor.close()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

