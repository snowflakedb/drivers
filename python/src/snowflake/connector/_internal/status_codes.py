"""
Proto StatusCode → PEP 249 exception class / errno mappings.

STATUS_CODE_* values are imported from the generated pb2 module so they
stay in sync with the Rust core automatically.
"""

from __future__ import annotations

from ..errors import (
    DatabaseError,
    DataError,
    Error,
    InternalError,
    NotSupportedError,
    OperationalError,
    ProgrammingError,
)
from .errorcode import ER_FAILED_TO_CONNECT_TO_DB, ER_INVALID_VALUE
from .protobuf_gen.database_driver_v1_pb2 import (
    STATUS_CODE_ALREADY_EXISTS,
    STATUS_CODE_AUTHENTICATION_ERROR,
    STATUS_CODE_CANCELLED,
    STATUS_CODE_GENERIC_ERROR,
    STATUS_CODE_INTERNAL_ERROR,
    STATUS_CODE_INVALID_ARGUMENT,
    STATUS_CODE_INVALID_DATA,
    STATUS_CODE_INVALID_PARAMETER_VALUE,
    STATUS_CODE_INVALID_STATE,
    STATUS_CODE_IO,
    STATUS_CODE_LOGIN_ERROR,
    STATUS_CODE_MISSING_PARAMETER,
    STATUS_CODE_NOT_FOUND,
    STATUS_CODE_NOT_IMPLEMENTED,
    STATUS_CODE_UNAUTHENTICATED,
    STATUS_CODE_UNAUTHORIZED,
)


STATUS_CODE_LABELS: dict[int, str] = {
    STATUS_CODE_AUTHENTICATION_ERROR: "Authentication error",
    STATUS_CODE_NOT_IMPLEMENTED: "Not implemented",
    STATUS_CODE_NOT_FOUND: "Not found",
    STATUS_CODE_ALREADY_EXISTS: "Already exists",
    STATUS_CODE_INVALID_ARGUMENT: "Invalid argument",
    STATUS_CODE_INVALID_STATE: "Invalid state",
    STATUS_CODE_INVALID_DATA: "Invalid data",
    STATUS_CODE_IO: "I/O error",
    STATUS_CODE_CANCELLED: "Cancelled",
    STATUS_CODE_UNAUTHENTICATED: "Unauthenticated",
    STATUS_CODE_UNAUTHORIZED: "Unauthorized",
    STATUS_CODE_GENERIC_ERROR: "Generic error",
    STATUS_CODE_INTERNAL_ERROR: "Internal error",
    STATUS_CODE_MISSING_PARAMETER: "Missing parameter",
    STATUS_CODE_INVALID_PARAMETER_VALUE: "Invalid parameter value",
    STATUS_CODE_LOGIN_ERROR: "Login error",
}

STATUS_TO_EXCEPTION: dict[int, type[Error]] = {
    STATUS_CODE_AUTHENTICATION_ERROR: DatabaseError,
    STATUS_CODE_NOT_IMPLEMENTED: NotSupportedError,
    STATUS_CODE_NOT_FOUND: ProgrammingError,
    STATUS_CODE_ALREADY_EXISTS: ProgrammingError,
    STATUS_CODE_INVALID_ARGUMENT: ProgrammingError,
    STATUS_CODE_INVALID_STATE: InternalError,
    STATUS_CODE_INVALID_DATA: DataError,
    STATUS_CODE_IO: OperationalError,
    STATUS_CODE_CANCELLED: OperationalError,
    STATUS_CODE_UNAUTHENTICATED: OperationalError,
    STATUS_CODE_UNAUTHORIZED: OperationalError,
    STATUS_CODE_GENERIC_ERROR: DatabaseError,
    # INTERNAL_ERROR → ProgrammingError: the Rust core uses this for Snowflake
    # query failures (syntax errors, etc.), not internal driver bugs.
    STATUS_CODE_INTERNAL_ERROR: ProgrammingError,
    STATUS_CODE_MISSING_PARAMETER: ProgrammingError,
    STATUS_CODE_INVALID_PARAMETER_VALUE: ProgrammingError,
    STATUS_CODE_LOGIN_ERROR: DatabaseError,
}

STATUS_TO_ERRNO: dict[int, int] = {
    STATUS_CODE_AUTHENTICATION_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
    STATUS_CODE_LOGIN_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
    STATUS_CODE_INVALID_PARAMETER_VALUE: ER_INVALID_VALUE,
}
