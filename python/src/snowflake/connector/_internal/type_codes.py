"""
Type code mappings for Snowflake data types to PEP 249 DB API 2.0.

Maps Snowflake type names to integer type codes for cursor.description.
"""

# Type code constants for PEP 249 compliance
# These match the indices from snowflake-connector-python for compatibility
FIXED = 0
REAL = 1
TEXT = 2
DATE = 3
TIMESTAMP = 4
VARIANT = 5
TIMESTAMP_LTZ = 6
TIMESTAMP_TZ = 7
TIMESTAMP_NTZ = 8
OBJECT = 9
ARRAY = 10
BINARY = 11
TIME = 12
BOOLEAN = 13
GEOGRAPHY = 14
GEOMETRY = 15
VECTOR = 16

# Mapping from Snowflake type name to type code
SNOWFLAKE_TYPE_TO_CODE = {
    "FIXED": FIXED,
    "NUMBER": FIXED,
    "DECIMAL": FIXED,
    "NUMERIC": FIXED,
    "INT": FIXED,
    "INTEGER": FIXED,
    "BIGINT": FIXED,
    "SMALLINT": FIXED,
    "TINYINT": FIXED,
    "BYTEINT": FIXED,
    "REAL": REAL,
    "FLOAT": REAL,
    "FLOAT4": REAL,
    "FLOAT8": REAL,
    "DOUBLE": REAL,
    "DOUBLE PRECISION": REAL,
    "TEXT": TEXT,
    "VARCHAR": TEXT,
    "CHAR": TEXT,
    "CHARACTER": TEXT,
    "STRING": TEXT,
    "DATE": DATE,
    "DATETIME": TIMESTAMP,
    "TIMESTAMP": TIMESTAMP,
    "TIMESTAMP_LTZ": TIMESTAMP_LTZ,
    "TIMESTAMP_NTZ": TIMESTAMP_NTZ,
    "TIMESTAMP_TZ": TIMESTAMP_TZ,
    "TIME": TIME,
    "VARIANT": VARIANT,
    "OBJECT": OBJECT,
    "ARRAY": ARRAY,
    "BINARY": BINARY,
    "VARBINARY": BINARY,
    "BOOLEAN": BOOLEAN,
    "GEOGRAPHY": GEOGRAPHY,
    "GEOMETRY": GEOMETRY,
    "VECTOR": VECTOR,
}

# Python type name to Snowflake type name mapping.
# Used by JsonBindingConverter to infer the Snowflake type from a Python value.
# Mirrors PYTHON_TO_SNOWFLAKE_TYPE from the reference connector's converter.py.
PYTHON_TO_SNOWFLAKE_TYPE = {
    "int": "FIXED",
    "long": "FIXED",
    "float": "REAL",
    "str": "TEXT",
    "unicode": "TEXT",
    "bool": "BOOLEAN",
    "bytes": "BINARY",
    "bytearray": "BINARY",
    "datetime": "TIMESTAMP_NTZ",
    "date": "DATE",
    "time": "TIME",
    "decimal": "FIXED",
    "struct_time": "TIMESTAMP_NTZ",
    "timedelta": "TIME",
    "nonetype": "ANY",
}


def get_type_code(snowflake_type: str) -> int:
    """
    Get the type code for a Snowflake type name.

    Args:
        snowflake_type: Snowflake type name (e.g., "FIXED", "TEXT", "TIMESTAMP_NTZ")

    Returns:
        int: Type code for PEP 249 compliance
    """
    # Normalize type name: uppercase and strip leading/trailing whitespace (preserve internal spaces)
    normalized_type = snowflake_type.upper().strip()
    return SNOWFLAKE_TYPE_TO_CODE.get(normalized_type, TEXT)  # Default to TEXT if unknown
