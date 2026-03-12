"""
Internal proto StatusCode enum values (from database_driver_v1.proto).

These are implementation details of the proto-to-PEP-249 error conversion
layer and are NOT part of the public API.  External code should not depend
on them.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from snowflake.connector._internal.errorcode import ER_FAILED_TO_CONNECT_TO_DB, ER_INVALID_VALUE


if TYPE_CHECKING:
    from snowflake.connector.errors import Error


STATUS_CODE_GENERIC_ERROR = 1
STATUS_CODE_AUTHENTICATION_ERROR = 2
STATUS_CODE_INVALID_ARGUMENT = 3
STATUS_CODE_TIMEOUT = 4
STATUS_CODE_NOT_FOUND = 5
STATUS_CODE_ALREADY_EXISTS = 6
STATUS_CODE_NOT_IMPLEMENTED = 7
STATUS_CODE_UNAUTHORIZED = 8
STATUS_CODE_CANCELLED = 9
STATUS_CODE_INVALID_STATE = 10
STATUS_CODE_RESOURCE_EXHAUSTED = 11
STATUS_CODE_ADBC_INTERNAL = 12
STATUS_CODE_IO = 13
STATUS_CODE_INTERNAL_ERROR = 14
STATUS_CODE_MISSING_PARAMETER = 15
STATUS_CODE_INVALID_PARAMETER_VALUE = 16
STATUS_CODE_LOGIN_ERROR = 17
STATUS_CODE_INVALID_DATA = 18

# Human-readable label for each status code (used as fallback message)
STATUS_CODE_LABELS: dict[int, str] = {
    STATUS_CODE_GENERIC_ERROR: "Generic error",
    STATUS_CODE_AUTHENTICATION_ERROR: "Authentication error",
    STATUS_CODE_INVALID_ARGUMENT: "Invalid argument",
    STATUS_CODE_TIMEOUT: "Timeout",
    STATUS_CODE_NOT_FOUND: "Not found",
    STATUS_CODE_ALREADY_EXISTS: "Already exists",
    STATUS_CODE_NOT_IMPLEMENTED: "Not implemented",
    STATUS_CODE_UNAUTHORIZED: "Unauthorized",
    STATUS_CODE_CANCELLED: "Cancelled",
    STATUS_CODE_INVALID_STATE: "Invalid state",
    STATUS_CODE_RESOURCE_EXHAUSTED: "Resource exhausted",
    STATUS_CODE_ADBC_INTERNAL: "ADBC internal error",
    STATUS_CODE_IO: "I/O error",
    STATUS_CODE_INTERNAL_ERROR: "Internal error",
    STATUS_CODE_MISSING_PARAMETER: "Missing parameter",
    STATUS_CODE_INVALID_PARAMETER_VALUE: "Invalid parameter value",
    STATUS_CODE_LOGIN_ERROR: "Login error",
    STATUS_CODE_INVALID_DATA: "Invalid data",
}


def _build_status_to_exception() -> dict[int, type[Error]]:
    """Build the status-code-to-exception-class mapping.

    Constructed via a function to avoid a circular import: this module is
    imported by ``errors.py`` (for errorcode constants), and the mapping
    needs the exception classes defined there.
    """
    from snowflake.connector.errors import (
        DatabaseError,
        DataError,
        InternalError,
        NotSupportedError,
        OperationalError,
        ProgrammingError,
    )

    return {
        STATUS_CODE_GENERIC_ERROR: DatabaseError,
        STATUS_CODE_AUTHENTICATION_ERROR: DatabaseError,
        STATUS_CODE_INVALID_ARGUMENT: ProgrammingError,
        STATUS_CODE_TIMEOUT: OperationalError,
        STATUS_CODE_NOT_FOUND: ProgrammingError,
        STATUS_CODE_ALREADY_EXISTS: ProgrammingError,
        STATUS_CODE_NOT_IMPLEMENTED: NotSupportedError,
        STATUS_CODE_UNAUTHORIZED: OperationalError,
        STATUS_CODE_CANCELLED: OperationalError,
        STATUS_CODE_INVALID_STATE: InternalError,
        STATUS_CODE_RESOURCE_EXHAUSTED: OperationalError,
        STATUS_CODE_ADBC_INTERNAL: InternalError,
        STATUS_CODE_IO: OperationalError,
        # INTERNAL_ERROR maps to ProgrammingError (not InternalError) because
        # the Rust core uses this status code for Snowflake query failures
        # (syntax errors, invalid identifiers, etc.) which are SQL programming
        # mistakes.  The server's vendor_code is surfaced separately via the
        # proto so the errno seen by users is the server code, not this value.
        STATUS_CODE_INTERNAL_ERROR: ProgrammingError,
        STATUS_CODE_MISSING_PARAMETER: ProgrammingError,
        STATUS_CODE_INVALID_PARAMETER_VALUE: ProgrammingError,
        STATUS_CODE_LOGIN_ERROR: DatabaseError,
        STATUS_CODE_INVALID_DATA: DataError,
    }


def _build_status_to_errno() -> dict[int, int]:
    """Build the status-code-to-errno mapping.

    For query errors (INTERNAL_ERROR) the Snowflake server code is exposed via
    the proto ``vendor_code`` field and takes precedence in the conversion layer.
    """
    return {
        STATUS_CODE_AUTHENTICATION_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
        STATUS_CODE_LOGIN_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
        STATUS_CODE_INVALID_PARAMETER_VALUE: ER_INVALID_VALUE,
    }


STATUS_TO_EXCEPTION: dict[int, type[Error]] = _build_status_to_exception()
STATUS_TO_ERRNO: dict[int, int] = _build_status_to_errno()
