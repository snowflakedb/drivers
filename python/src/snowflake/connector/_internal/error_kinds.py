"""
Proto ErrorKind → PEP 249 exception class / errno mappings.

ERROR_KIND_* values are imported from the generated pb2 module so they
stay in sync with the Rust core automatically.
"""

from __future__ import annotations

from ..errors import (
    DatabaseError,
    Error,
    IntegrityError,
    NotSupportedError,
    OperationalError,
    ProgrammingError,
)
from .errorcode import (
    ER_COMPRESSION_NOT_SUPPORTED,
    ER_FAILED_TO_CONNECT_TO_DB,
    ER_FILE_NOT_EXISTS,
    ER_INVALID_VALUE,
)
from .protobuf_gen.database_driver_v1_pb2 import (
    ERROR_KIND_AUTHENTICATION_ERROR,
    ERROR_KIND_CANCELLED,
    ERROR_KIND_GENERIC_ERROR,
    ERROR_KIND_INTERNAL_ERROR,
    ERROR_KIND_INVALID_ARGUMENT,
    ERROR_KIND_INVALID_PARAMETER_VALUE,
    ERROR_KIND_IO,
    ERROR_KIND_LOCAL_FILE_NOT_FOUND,
    ERROR_KIND_LOGIN_ERROR,
    ERROR_KIND_MISSING_PARAMETER,
    ERROR_KIND_NOT_IMPLEMENTED,
    ERROR_KIND_REMOTE_FILE_NOT_FOUND,
    ERROR_KIND_UNSUPPORTED_COMPRESSION,
)


ERROR_KIND_LABELS: dict[int, str] = {
    ERROR_KIND_AUTHENTICATION_ERROR: "Authentication error",
    ERROR_KIND_NOT_IMPLEMENTED: "Not implemented",
    ERROR_KIND_INVALID_ARGUMENT: "Invalid argument",
    ERROR_KIND_IO: "I/O error",
    ERROR_KIND_CANCELLED: "Cancelled",
    ERROR_KIND_GENERIC_ERROR: "Generic error",
    ERROR_KIND_INTERNAL_ERROR: "Internal error",
    ERROR_KIND_MISSING_PARAMETER: "Missing parameter",
    ERROR_KIND_INVALID_PARAMETER_VALUE: "Invalid parameter value",
    ERROR_KIND_LOGIN_ERROR: "Login error",
    ERROR_KIND_LOCAL_FILE_NOT_FOUND: "Local file not found",
    ERROR_KIND_REMOTE_FILE_NOT_FOUND: "Remote file not found",
    ERROR_KIND_UNSUPPORTED_COMPRESSION: "Unsupported compression type",
}

KIND_TO_EXCEPTION: dict[int, type[Error]] = {
    ERROR_KIND_AUTHENTICATION_ERROR: DatabaseError,
    ERROR_KIND_NOT_IMPLEMENTED: NotSupportedError,
    ERROR_KIND_INVALID_ARGUMENT: ProgrammingError,
    ERROR_KIND_IO: OperationalError,
    ERROR_KIND_CANCELLED: OperationalError,
    ERROR_KIND_GENERIC_ERROR: DatabaseError,
    # INTERNAL_ERROR → ProgrammingError: the Rust core uses this for Snowflake
    # query failures (syntax errors, etc.), not internal driver bugs.
    ERROR_KIND_INTERNAL_ERROR: ProgrammingError,
    ERROR_KIND_MISSING_PARAMETER: ProgrammingError,
    ERROR_KIND_INVALID_PARAMETER_VALUE: ProgrammingError,
    ERROR_KIND_LOGIN_ERROR: DatabaseError,
    ERROR_KIND_LOCAL_FILE_NOT_FOUND: ProgrammingError,
    ERROR_KIND_REMOTE_FILE_NOT_FOUND: OperationalError,
    ERROR_KIND_UNSUPPORTED_COMPRESSION: ProgrammingError,
}

# Snowflake vendor_code → exception overrides.
#
# KIND_TO_EXCEPTION maps the proto ErrorKind (a broad category) to a default PEP 249 class.
# Some Snowflake server errors share the same ErrorKind (e.g. ERROR_KIND_INTERNAL_ERROR)
# but carry a vendor_code that warrants a more specific exception.
# Entries here take precedence over KIND_TO_EXCEPTION when a vendor_code is present.
VENDOR_CODE_TO_EXCEPTION: dict[int, type[Error]] = {
    100072: IntegrityError,  # NULL result in a non-nullable column
}

# Prefer the Snowflake server vendor_code when the core driver provides it, fallback to this mapping if not present.
KIND_TO_ERRNO: dict[int, int] = {
    ERROR_KIND_AUTHENTICATION_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
    ERROR_KIND_LOGIN_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
    ERROR_KIND_INVALID_PARAMETER_VALUE: ER_INVALID_VALUE,
    ERROR_KIND_LOCAL_FILE_NOT_FOUND: ER_FILE_NOT_EXISTS,
    ERROR_KIND_REMOTE_FILE_NOT_FOUND: ER_FILE_NOT_EXISTS,
    ERROR_KIND_UNSUPPORTED_COMPRESSION: ER_COMPRESSION_NOT_SUPPORTED,
}
