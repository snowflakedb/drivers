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
    IntegrityError,
    InternalError,
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
    STATUS_CODE_LOCAL_FILE_NOT_FOUND,
    STATUS_CODE_LOGIN_ERROR,
    STATUS_CODE_MISSING_PARAMETER,
    STATUS_CODE_NOT_FOUND,
    STATUS_CODE_NOT_IMPLEMENTED,
    STATUS_CODE_REMOTE_FILE_NOT_FOUND,
    STATUS_CODE_UNAUTHENTICATED,
    STATUS_CODE_UNAUTHORIZED,
    STATUS_CODE_UNSUPPORTED_COMPRESSION,
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
    STATUS_CODE_LOCAL_FILE_NOT_FOUND: "Local file not found",
    STATUS_CODE_REMOTE_FILE_NOT_FOUND: "Remote file not found",
    STATUS_CODE_UNSUPPORTED_COMPRESSION: "Unsupported compression type",
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
    STATUS_CODE_LOCAL_FILE_NOT_FOUND: ProgrammingError,
    STATUS_CODE_REMOTE_FILE_NOT_FOUND: OperationalError,
    STATUS_CODE_UNSUPPORTED_COMPRESSION: ProgrammingError,
}

def _build_vendor_code_to_exception() -> dict[int, type[Error]]:
    # Local import, deliberately not at module scope: ReauthenticationRequest
    # is a @backward_compatibility name (see ../errors.py), and
    # TestNoInternalImportsOfBackwardCompatNames forbids any internal
    # snowflake.connector module from rebinding one into its own globals. A
    # function-local import never becomes a module attribute, so this dict's
    # *values* can be ReauthenticationRequest instances without
    # `status_codes.ReauthenticationRequest` existing as a name anyone could
    # import — the same constraint the network.py shim works around with a
    # lazy __getattr__ instead of a static import.
    from ..errors import ReauthenticationRequest

    return {
        100072: IntegrityError,  # NULL result in a non-nullable column
        # Mid-session master-token-terminal codes: the master token was not
        # found, expired, or is invalid — the session can never be renewed.
        # Raised as ReauthenticationRequest (not the ReauthenticationRequiredError
        # base) so `except ReauthenticationRequest` — the legacy type name real
        # consumers like Snowpark catch — fires on driver-raised instances too.
        390113: ReauthenticationRequest,
        390114: ReauthenticationRequest,
        390115: ReauthenticationRequest,
        # Login-time cached-credential rejection, after the driver's own
        # evict-and-retry ladder gives up: cached ID token (390195) or cached
        # OAuth access token invalid/expired (390303/390318).
        390195: ReauthenticationRequest,
        390303: ReauthenticationRequest,
        390318: ReauthenticationRequest,
    }


# Snowflake vendor_code → exception overrides.
#
# STATUS_TO_EXCEPTION maps the proto StatusCode (a broad category) to a default PEP 249 class.
# Some Snowflake server errors share the same StatusCode (e.g. STATUS_CODE_INTERNAL_ERROR)
# but carry a vendor_code that warrants a more specific exception.
# Entries here take precedence over STATUS_TO_EXCEPTION when a vendor_code is present.
VENDOR_CODE_TO_EXCEPTION: dict[int, type[Error]] = _build_vendor_code_to_exception()

# Prefer the Snowflake server vendor_code when the core driver provides it, fallback to this mapping if not present.
STATUS_TO_ERRNO: dict[int, int] = {
    STATUS_CODE_AUTHENTICATION_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
    STATUS_CODE_LOGIN_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
    STATUS_CODE_INVALID_PARAMETER_VALUE: ER_INVALID_VALUE,
    STATUS_CODE_LOCAL_FILE_NOT_FOUND: ER_FILE_NOT_EXISTS,
    STATUS_CODE_REMOTE_FILE_NOT_FOUND: ER_FILE_NOT_EXISTS,
    STATUS_CODE_UNSUPPORTED_COMPRESSION: ER_COMPRESSION_NOT_SUPPORTED,
}
