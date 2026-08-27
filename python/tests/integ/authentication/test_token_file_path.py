"""token_file_path must satisfy PAT / OAuth / OIDC token requirements.

These tests do not complete a real login: they only assert that a file-backed
token is accepted in place of an inline ``token`` parameter (the customer
connections.toml path).
"""

from __future__ import annotations

import re

import pytest

from ...compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")

_MISSING_TOKEN_RE = re.compile(
    r"Missing required parameter[: ]\s*['\"]?token['\"]?",
    re.IGNORECASE,
)


def _full_error_text(exception: BaseException) -> str:
    parts: list[str] = []
    current: BaseException | None = exception
    seen: set[int] = set()
    while current is not None and id(current) not in seen:
        seen.add(id(current))
        parts.append(repr(current))
        parts.append(str(current))
        current = current.__cause__ or current.__context__
    return "\n".join(parts)


def _attempt_connect(int_test_connection_factory, **kwargs):
    with pytest.raises(Exception) as exc_info:
        int_test_connection_factory(**kwargs)
    return exc_info.value


class TestTokenFilePath:
    def test_pat_accepts_token_file_path_instead_of_token(self, int_test_connection_factory, tmp_path):
        token_file = tmp_path / "pat.token"
        token_file.write_text("not-a-real-pat\n")
        token_file.chmod(0o600)

        exception = _attempt_connect(
            int_test_connection_factory,
            authenticator="PROGRAMMATIC_ACCESS_TOKEN",
            private_key_file=None,
            token_file_path=str(token_file),
        )
        text = _full_error_text(exception)
        assert not _MISSING_TOKEN_RE.search(text), f"PAT must accept token_file_path in place of token. Got: {text!r}"
        assert "Unknown parameter" not in text, f"token_file_path must be a known connection parameter. Got: {text!r}"
