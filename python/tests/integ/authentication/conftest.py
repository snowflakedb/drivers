"""Shared helpers for authentication integration tests."""

from __future__ import annotations

import base64
import json

import pytest


def full_error_text(exception: BaseException) -> str:
    """Walk the full exception chain and return all repr/str text joined."""
    parts: list[str] = []
    current: BaseException | None = exception
    seen: set[int] = set()
    while current is not None and id(current) not in seen:
        seen.add(id(current))
        parts.append(repr(current))
        parts.append(str(current))
        current = current.__cause__ or current.__context__
    return "\n".join(parts)


def make_dummy_jwt(sub: str = "test-service", iss: str = "https://accounts.example.com") -> str:
    """Minimal structurally-valid JWT (signature is fake; passes format checks)."""

    def _b64url_encode(data: dict) -> str:
        return base64.urlsafe_b64encode(json.dumps(data).encode()).rstrip(b"=").decode()

    header = _b64url_encode({"alg": "RS256", "typ": "JWT"})
    payload = _b64url_encode({"sub": sub, "iss": iss, "aud": "snowflakecomputing.com"})
    return f"{header}.{payload}.fake-sig"


def connect_expecting_error(int_test_connection_factory, **kwargs) -> BaseException:
    """Call int_test_connection_factory, assert it raises, and return the exception."""
    with pytest.raises(Exception) as exc_info:
        int_test_connection_factory(**kwargs)
    return exc_info.value
