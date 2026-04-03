from __future__ import annotations

import ctypes

from ctypes import c_char_p
from typing import TYPE_CHECKING, Any


if TYPE_CHECKING:
    from snowflake.connector.errors import Error

from ..protobuf_gen.database_driver_v1_pb2 import (
    AuthenticationError as ProtoAuthenticationError,
)
from ..protobuf_gen.database_driver_v1_pb2 import (
    InvalidParameterValue as ProtoInvalidParameterValue,
)
from ..protobuf_gen.database_driver_v1_pb2 import (
    LoginError as ProtoLoginError,
)
from ..protobuf_gen.database_driver_v1_pb2 import (
    MissingParameter as ProtoMissingParameter,
)
from ..protobuf_gen.database_driver_v1_services import DatabaseDriverClient
from ..protobuf_gen.proto_exception import (
    ProtoApplicationException,
    ProtoTransportException,
)
from .c_api import sf_core_api_call_proto, sf_core_free_buffer


# ---------------------------------------------------------------------------
# Proto-to-PEP-249 error conversion (kept here, at the transport boundary)
# ---------------------------------------------------------------------------


def _extract_error_detail(driver_exception: Any) -> str | None:
    error = getattr(driver_exception, "error", None)
    if error is None:
        return None

    error_type = error.WhichOneof("error_type")
    if error_type is None:
        return None

    inner = getattr(error, error_type, None)
    if inner is None:
        return None

    if isinstance(inner, ProtoAuthenticationError):
        return inner.detail or None
    if isinstance(inner, ProtoLoginError):
        if inner.message and inner.code:
            return f"{inner.message} (code={inner.code})"
        return inner.message or None
    if isinstance(inner, ProtoMissingParameter):
        return f"Missing required parameter: {inner.parameter}" if inner.parameter else None
    if isinstance(inner, ProtoInvalidParameterValue):
        parts = [f"Invalid value {inner.value!r} for parameter {inner.parameter!r}"]
        if inner.explanation:
            parts.append(inner.explanation)
        return ". ".join(parts)
    # GenericError, InternalError have no extra fields
    return None


def _append_detail(base: str, detail: str) -> str:
    """Append *detail* to *base* with `. ` separator, avoiding double punctuation."""
    if not base:
        return detail
    base = base.rstrip(".")
    return f"{base}. {detail}"


def _proto_to_public_error(proto_exc: Exception) -> Error:
    """Convert a proto-layer exception into a PEP 249 ``Error`` subclass.

    This function **returns** the converted exception; it does not raise it.
    The caller (``_raise_error`` in the generated client) is responsible for
    raising the returned value.
    """
    from snowflake.connector.errors import DatabaseError, OperationalError

    if isinstance(proto_exc, ProtoApplicationException):
        return _convert_application_error(proto_exc)
    if isinstance(proto_exc, ProtoTransportException):
        return OperationalError(f"Driver communication error: {proto_exc}")
    return DatabaseError(str(proto_exc))


def _refine_exception_class(
    exc_class: type,
    status_code: int,
    message: str,
) -> type:
    """Refine the PEP 249 exception class using message content.

    A single proto StatusCode can map to several specific Python exception
    classes depending on the Rust error variant that produced it.  This
    function performs a secondary dispatch based on well-known substrings in
    the combined message + root_cause string (the root_cause has already been
    appended to *message* by the caller before this function is invoked).

    Keywords are coupled to the Rust error Display strings in:
      - sf_core/src/crl/error.rs  (CrlError variants → RevocationCheckError)
      - sf_core/src/apis/database_driver_v1/error.rs
          ApiError::MasterTokenExpired  → TokenExpiredError
          ApiError::InvalidRefreshState → RefreshTokenError
    """
    from snowflake.connector._internal.status_codes import (
        STATUS_CODE_AUTHENTICATION_ERROR,
        STATUS_CODE_INTERNAL_ERROR,
    )
    from snowflake.connector.errors import (
        RefreshTokenError,
        RevocationCheckError,
        TokenExpiredError,
    )

    combined = message.lower()

    if status_code == STATUS_CODE_AUTHENTICATION_ERROR:
        if "master token expired" in combined:
            return TokenExpiredError
        # CrlError display strings all contain "revoked" or "crl"
        if "revoked" in combined or "crl" in combined:
            return RevocationCheckError

    if status_code == STATUS_CODE_INTERNAL_ERROR:
        if "invalid refresh state" in combined:
            return RefreshTokenError

    return exc_class


def _http_error_from_status(
    http_status: int,
    message: str,
    sqlstate: str | None,
    sfqid: str | None,
) -> Error:
    """Map a raw HTTP status code to the matching HTTP exception class."""
    from snowflake.connector.errors import (
        BadGatewayError,
        BadRequest,
        ForbiddenError,
        GatewayTimeoutError,
        InternalServerError,
        MethodNotAllowed,
        OtherHTTPRetryableError,
        RequestTimeoutError,
        ServiceUnavailableError,
        TooManyRequests,
    )

    _HTTP_STATUS_TO_EXC: dict[int, type] = {
        400: BadRequest,
        403: ForbiddenError,
        405: MethodNotAllowed,
        408: RequestTimeoutError,
        429: TooManyRequests,
        500: InternalServerError,
        502: BadGatewayError,
        503: ServiceUnavailableError,
        504: GatewayTimeoutError,
    }

    exc_class = _HTTP_STATUS_TO_EXC.get(http_status)
    if exc_class is not None:
        return exc_class(msg=message, sqlstate=sqlstate, sfqid=sfqid)
    return OtherHTTPRetryableError(msg=message, code=http_status, sqlstate=sqlstate, sfqid=sfqid)


def _convert_application_error(proto_exc: ProtoApplicationException) -> Error:
    from snowflake.connector._internal.status_codes import (
        STATUS_CODE_LABELS,
        STATUS_TO_ERRNO,
        STATUS_TO_EXCEPTION,
    )
    from snowflake.connector.errors import DatabaseError

    driver_exc = getattr(proto_exc, "api_error_pb", None)
    if driver_exc is None:
        return DatabaseError(str(proto_exc))

    status_code = getattr(driver_exc, "status_code", 0)
    message = getattr(driver_exc, "message", "") or ""

    # The root_cause field carries the deepest error in the chain from the
    # Rust core, which is typically the most informative for end users.
    root_cause = _get_optional_str(driver_exc, "root_cause")
    if root_cause and root_cause not in message:
        message = _append_detail(message, root_cause)

    detail = _extract_error_detail(driver_exc)
    if detail and detail not in message:
        message = _append_detail(message, detail)

    if not message:
        message = STATUS_CODE_LABELS.get(status_code, "Unknown error")

    # HTTP status override: when the core provides a concrete HTTP status code,
    # raise the matching HTTP exception class (e.g. ServiceUnavailableError for
    # 503, TooManyRequests for 429) rather than the coarser StatusCode-derived
    # class.
    http_status_code = _get_optional_int(driver_exc, "http_status_code")
    if http_status_code is not None:
        sqlstate = _get_optional_str(driver_exc, "sql_state") or _derive_sqlstate(driver_exc)
        sfqid = _get_optional_str(driver_exc, "query_id")
        return _http_error_from_status(http_status_code, message, sqlstate, sfqid)

    exc_class = STATUS_TO_EXCEPTION.get(status_code, DatabaseError)
    exc_class = _refine_exception_class(exc_class, status_code, message)

    # Prefer the Snowflake server vendor_code when the core driver provides it
    # (e.g. 1003 for syntax error, 904 for invalid identifier).
    # Fall back to the old-driver-compatible errno mapping, then to the raw
    # proto status code.
    vendor_code = _get_optional_int(driver_exc, "vendor_code")
    errno = vendor_code if vendor_code is not None else STATUS_TO_ERRNO.get(status_code, status_code)

    # Prefer the server-provided sql_state; fall back to a type-derived value.
    sqlstate = _get_optional_str(driver_exc, "sql_state") or _derive_sqlstate(driver_exc)

    sfqid = _get_optional_str(driver_exc, "query_id")

    return exc_class(msg=message, errno=errno, sqlstate=sqlstate, sfqid=sfqid)


def _get_optional_int(msg: Any, field: str) -> int | None:
    """Read an optional int32 proto field, returning None if not set."""
    try:
        if msg.HasField(field):
            return int(getattr(msg, field))
    except (ValueError, TypeError, AttributeError):
        # Field missing from the proto schema or cannot be coerced to int; treat as unset.
        pass
    return None


def _get_optional_str(msg: Any, field: str) -> str | None:
    """Read an optional string proto field, returning None if not set."""
    try:
        if msg.HasField(field):
            return str(getattr(msg, field)) or None
    except (ValueError, TypeError, AttributeError):
        # Field missing from the proto schema or cannot be coerced to string; treat as unset.
        pass
    return None


def _derive_sqlstate(driver_exception: Any) -> str | None:
    """Derive sqlstate from the error type when the proto does not carry it.

    Only login/auth errors have an obvious ANSI SQL state mapping today.
    Other error types (missing_parameter, invalid_parameter_value, etc.)
    will return ``None``; extend this function as mappings become clear.
    """
    error = getattr(driver_exception, "error", None)
    if error is None:
        return None
    error_type = error.WhichOneof("error_type")
    if error_type in ("login_error", "auth_error"):
        return "08001"  # SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED
    return None


# ---------------------------------------------------------------------------
# Transport + singleton
# ---------------------------------------------------------------------------


class ProtoTransport:
    def handle_message(self, api: str, method: str, message: bytes) -> tuple[int, bytes]:
        response = ctypes.POINTER(ctypes.c_ubyte)()
        response_len = ctypes.c_size_t()
        api_bytes: c_char_p = ctypes.c_char_p(api.encode("utf-8"))
        method_bytes: c_char_p = ctypes.c_char_p(method.encode("utf-8"))
        message_buf = (ctypes.c_ubyte * len(message))()
        message_buf[:] = message  # type: ignore
        code = sf_core_api_call_proto(
            api_bytes,
            method_bytes,
            ctypes.cast(message_buf, ctypes.POINTER(ctypes.c_ubyte)),
            len(message),
            ctypes.byref(response),
            ctypes.byref(response_len),
        )
        if code == 0 or code == 1 or code == 2:
            result = bytes(response[: response_len.value])
            sf_core_free_buffer(response, response_len.value)
            return (code, result)

        raise ProtoTransportException(f"Unknown error code: {code}")


_DATABASE_DRIVER_CLIENT: DatabaseDriverClient | None = None


def database_driver_client() -> DatabaseDriverClient:
    global _DATABASE_DRIVER_CLIENT
    if _DATABASE_DRIVER_CLIENT is None:
        _DATABASE_DRIVER_CLIENT = DatabaseDriverClient(ProtoTransport(), error_handler=_proto_to_public_error)
    return _DATABASE_DRIVER_CLIENT
