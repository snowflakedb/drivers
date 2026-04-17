"""
Snowflake error codes.

These codes are intentionally aligned with the reference snowflake-connector-python
so that users can rely on stable, documented error numbers across driver implementations.
"""

from __future__ import annotations


# network
ER_FAILED_TO_CONNECT_TO_DB = 250001
ER_CONNECTION_IS_CLOSED = 250002

# connection
ER_NO_ACCOUNT_NAME = 251001
ER_NO_USER = 251005
ER_NO_PASSWORD = 251006
ER_INVALID_VALUE = 251007

# cursor
ER_FAILED_TO_CONVERT_ROW_TO_PYTHON_TYPE = 252005
ER_CURSOR_IS_CLOSED = 252006

# file transfer
ER_FILE_NOT_EXISTS = 253006
ER_COMPRESSION_NOT_SUPPORTED = 253007

# converter
ER_NO_PYARROW = 255002
ER_NO_NUMPY = 255006
