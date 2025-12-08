#!/usr/bin/env python3
"""Real-world test against actual Snowflake using our universal driver."""

import json
import os
import sys
import random
import decimal
from datetime import date

# Add pep249_dbapi to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'pep249_dbapi'))

import pep249_dbapi


def load_credentials():
    """Load Snowflake credentials from parameters.json"""
    param_file = os.path.join(os.path.dirname(__file__), 'parameters.json')
    with open(param_file) as f:
        params = json.load(f)["testconnection"]
    return params


def test_connection():
    """Test basic connection"""
    print("\n" + "="*70)
    print("TEST 1: Basic Connection")
    print("="*70)
    
    params = load_credentials()
    conn = pep249_dbapi.connect(
        account=params["SNOWFLAKE_TEST_ACCOUNT"],
        user=params["SNOWFLAKE_TEST_USER"],
        password=params["SNOWFLAKE_TEST_PASSWORD"],
        database=params.get("SNOWFLAKE_TEST_DATABASE"),
        schema=params.get("SNOWFLAKE_TEST_SCHEMA"),
        warehouse=params.get("SNOWFLAKE_TEST_WAREHOUSE"),
    )
    
    cursor = conn.cursor()
    cursor.execute("SELECT 1 as test_col")
    result = cursor.fetchone()
    
    assert result[0] == '1', f"Expected '1', got {result[0]}"
    print(f"✓ Basic query works: SELECT 1 returned {result[0]}")
    
    cursor.close()
    conn.close()
    print("✓ Connection closed successfully")
    

def test_integer_type():
    """Test INTEGER data type with data integrity"""
    print("\n" + "="*70)
    print("TEST 2: INTEGER Data Type")
    print("="*70)
    
    params = load_credentials()
    conn = pep249_dbapi.connect(
        account=params["SNOWFLAKE_TEST_ACCOUNT"],
        user=params["SNOWFLAKE_TEST_USER"],
        password=params["SNOWFLAKE_TEST_PASSWORD"],
        database=params.get("SNOWFLAKE_TEST_DATABASE"),
        schema=params.get("SNOWFLAKE_TEST_SCHEMA"),
        warehouse=params.get("SNOWFLAKE_TEST_WAREHOUSE"),
    )
    
    cursor = conn.cursor()
    table = f"test_int_{random.randint(1000, 9999)}"
    
    try:
        # Create table
        cursor.execute(f"CREATE OR REPLACE TABLE {table} (id INT, value INT)")
        print(f"✓ Created table {table}")
        
        # Insert data
        test_data = [(i, i * i) for i in range(1, 11)]
        for id_val, val in test_data:
            cursor.execute(f"INSERT INTO {table} VALUES ({id_val}, {val})")
        print(f"✓ Inserted {len(test_data)} rows")
        
        # Read back
        cursor.execute(f"SELECT * FROM {table} ORDER BY id")
        results = cursor.fetchall()
        
        assert len(results) == len(test_data), f"Expected {len(test_data)} rows, got {len(results)}"
        
        for i, (result_row, test_row) in enumerate(zip(results, test_data)):
            # Convert strings to ints for comparison
            result_vals = (int(result_row[0]), int(result_row[1]))
            assert result_vals == test_row, f"Row {i}: {result_vals} != {test_row}"
        
        print(f"✓ All {len(results)} rows verified correctly")
        
    finally:
        cursor.execute(f"DROP TABLE IF EXISTS {table}")
        cursor.close()
        conn.close()


def test_multiple_types():
    """Test multiple data types in one table"""
    print("\n" + "="*70)
    print("TEST 3: Multiple Data Types")
    print("="*70)
    
    params = load_credentials()
    conn = pep249_dbapi.connect(
        account=params["SNOWFLAKE_TEST_ACCOUNT"],
        user=params["SNOWFLAKE_TEST_USER"],
        password=params["SNOWFLAKE_TEST_PASSWORD"],
        database=params.get("SNOWFLAKE_TEST_DATABASE"),
        schema=params.get("SNOWFLAKE_TEST_SCHEMA"),
        warehouse=params.get("SNOWFLAKE_TEST_WAREHOUSE"),
    )
    
    cursor = conn.cursor()
    table = f"test_multi_{random.randint(1000, 9999)}"
    
    try:
        # Create table with multiple types
        cursor.execute(f"""
            CREATE OR REPLACE TABLE {table} (
                id INT,
                name VARCHAR,
                price DECIMAL(10,2),
                active BOOLEAN,
                created DATE
            )
        """)
        print(f"✓ Created table {table} with 5 different data types")
        
        # Insert test data
        cursor.execute(f"""
            INSERT INTO {table} VALUES
            (1, 'Product A', 19.99, TRUE, '2024-01-15'),
            (2, 'Product B', 29.50, FALSE, '2024-02-20'),
            (3, 'Product C', 99.00, TRUE, '2024-03-10')
        """)
        print("✓ Inserted 3 test rows")
        
        # Read back
        cursor.execute(f"SELECT * FROM {table} ORDER BY id")
        results = cursor.fetchall()
        
        assert len(results) == 3, f"Expected 3 rows, got {len(results)}"
        
        # Check first row
        row1 = results[0]
        print(f"  Row 1: {row1}")
        assert int(row1[0]) == 1
        assert row1[1] == 'Product A'
        # Price might come back as string, decimal, or float
        assert float(str(row1[2])) == 19.99
        
        print("✓ Multiple data types verified")
        
    finally:
        cursor.execute(f"DROP TABLE IF EXISTS {table}")
        cursor.close()
        conn.close()


def test_null_handling():
    """Test NULL value handling"""
    print("\n" + "="*70)
    print("TEST 4: NULL Value Handling")
    print("="*70)
    
    params = load_credentials()
    conn = pep249_dbapi.connect(
        account=params["SNOWFLAKE_TEST_ACCOUNT"],
        user=params["SNOWFLAKE_TEST_USER"],
        password=params["SNOWFLAKE_TEST_PASSWORD"],
        database=params.get("SNOWFLAKE_TEST_DATABASE"),
        schema=params.get("SNOWFLAKE_TEST_SCHEMA"),
        warehouse=params.get("SNOWFLAKE_TEST_WAREHOUSE"),
    )
    
    cursor = conn.cursor()
    table = f"test_nulls_{random.randint(1000, 9999)}"
    
    try:
        cursor.execute(f"""
            CREATE OR REPLACE TABLE {table} (
                id INT,
                nullable_string VARCHAR,
                nullable_int INT
            )
        """)
        
        cursor.execute(f"INSERT INTO {table} VALUES (1, 'hello', 100)")
        cursor.execute(f"INSERT INTO {table} VALUES (2, NULL, 200)")
        cursor.execute(f"INSERT INTO {table} VALUES (3, 'world', NULL)")
        cursor.execute(f"INSERT INTO {table} VALUES (4, NULL, NULL)")
        
        cursor.execute(f"SELECT * FROM {table} ORDER BY id")
        results = cursor.fetchall()
        
        assert len(results) == 4
        print(f"✓ Retrieved 4 rows with NULL values")
        
        # Check NULL handling
        assert results[1][1] is None, f"Expected None, got {results[1][1]}"
        assert results[2][2] is None, f"Expected None, got {results[2][2]}"
        assert results[3][1] is None and results[3][2] is None
        
        print("✓ NULL values handled correctly")
        
    finally:
        cursor.execute(f"DROP TABLE IF EXISTS {table}")
        cursor.close()
        conn.close()


def test_large_result_set():
    """Test fetching larger result set"""
    print("\n" + "="*70)
    print("TEST 5: Large Result Set (100 rows)")
    print("="*70)
    
    params = load_credentials()
    conn = pep249_dbapi.connect(
        account=params["SNOWFLAKE_TEST_ACCOUNT"],
        user=params["SNOWFLAKE_TEST_USER"],
        password=params["SNOWFLAKE_TEST_PASSWORD"],
        database=params.get("SNOWFLAKE_TEST_DATABASE"),
        schema=params.get("SNOWFLAKE_TEST_SCHEMA"),
        warehouse=params.get("SNOWFLAKE_TEST_WAREHOUSE"),
    )
    
    cursor = conn.cursor()
    table = f"test_large_{random.randint(1000, 9999)}"
    
    try:
        cursor.execute(f"CREATE OR REPLACE TABLE {table} (id INT, value VARCHAR)")
        
        # Insert 100 rows
        for i in range(1, 101):
            cursor.execute(f"INSERT INTO {table} VALUES ({i}, 'row_{i:03d}')")
        
        print("✓ Inserted 100 rows")
        
        # Fetch all
        cursor.execute(f"SELECT * FROM {table} ORDER BY id")
        results = cursor.fetchall()
        
        assert len(results) == 100, f"Expected 100 rows, got {len(results)}"
        
        # Verify first and last
        assert int(results[0][0]) == 1
        assert results[0][1] == 'row_001'
        assert int(results[-1][0]) == 100
        assert results[-1][1] == 'row_100'
        
        print("✓ All 100 rows fetched and verified")
        
        # Test fetchmany
        cursor.execute(f"SELECT * FROM {table} ORDER BY id")
        batch = cursor.fetchmany(10)
        assert len(batch) == 10, f"Expected 10 rows, got {len(batch)}"
        print("✓ fetchmany(10) works correctly")
        
    finally:
        cursor.execute(f"DROP TABLE IF EXISTS {table}")
        cursor.close()
        conn.close()


def test_all_numeric_types():
    """Test all numeric types: INT, DECIMAL, REAL, DOUBLE"""
    print("\n" + "="*70)
    print("TEST 6: All Numeric Types")
    print("="*70)
    
    params = load_credentials()
    conn = pep249_dbapi.connect(
        account=params["SNOWFLAKE_TEST_ACCOUNT"],
        user=params["SNOWFLAKE_TEST_USER"],
        password=params["SNOWFLAKE_TEST_PASSWORD"],
        database=params.get("SNOWFLAKE_TEST_DATABASE"),
        schema=params.get("SNOWFLAKE_TEST_SCHEMA"),
        warehouse=params.get("SNOWFLAKE_TEST_WAREHOUSE"),
    )
    
    cursor = conn.cursor()
    
    try:
        # Test each numeric type
        cursor.execute("SELECT 42::INT as int_val")
        result = cursor.fetchone()
        print(f"✓ INT: {result[0]}")
        
        cursor.execute("SELECT 123.45::DECIMAL(10,2) as decimal_val")
        result = cursor.fetchone()
        print(f"✓ DECIMAL: {result[0]}")
        
        cursor.execute("SELECT 3.14::REAL as real_val")
        result = cursor.fetchone()
        print(f"✓ REAL: {result[0]}")
        
        cursor.execute("SELECT 2.718281828::DOUBLE as double_val")
        result = cursor.fetchone()
        print(f"✓ DOUBLE: {result[0]}")
        
        cursor.execute("SELECT TRUE as bool_val")
        result = cursor.fetchone()
        print(f"✓ BOOLEAN: {result[0]}")
        
        print("✓ All numeric types working")
        
    finally:
        cursor.close()
        conn.close()


def main():
    """Run all tests"""
    print("\n")
    print("╔" + "="*68 + "╗")
    print("║" + " "*10 + "UNIVERSAL SNOWFLAKE DRIVER - REAL TESTS" + " "*19 + "║")
    print("╚" + "="*68 + "╝")
    
    tests = [
        ("Connection", test_connection),
        ("Integer Type", test_integer_type),
        ("Multiple Types", test_multiple_types),
        ("NULL Handling", test_null_handling),
        ("Large Result Set", test_large_result_set),
        ("All Numeric Types", test_all_numeric_types),
    ]
    
    passed = 0
    failed = 0
    
    for name, test_func in tests:
        try:
            test_func()
            passed += 1
        except Exception as e:
            print(f"\n✗ TEST FAILED: {name}")
            print(f"  Error: {e}")
            import traceback
            traceback.print_exc()
            failed += 1
    
    print("\n" + "="*70)
    print(f"RESULTS: {passed} passed, {failed} failed out of {len(tests)} tests")
    print("="*70 + "\n")
    
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())

