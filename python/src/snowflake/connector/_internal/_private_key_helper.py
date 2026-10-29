"""Helper functions for private key conversion."""

from __future__ import annotations

from typing import Any

from ..errors import ProgrammingError


def normalize_private_key(private_key: Any) -> bytes | str:
    """Return PKCS#8 DER for a cryptography key object; pass bytes and str through.

    The Rust core reads DER bytes and PEM or base64 strings itself.
    """
    if isinstance(private_key, (bytes, str)):
        return private_key

    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.ec import EllipticCurvePrivateKey
    from cryptography.hazmat.primitives.asymmetric.rsa import RSAPrivateKey

    if isinstance(private_key, (RSAPrivateKey, EllipticCurvePrivateKey)):
        return private_key.private_bytes(
            encoding=serialization.Encoding.DER,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )
    raise ProgrammingError(
        f"Unsupported private_key type: {type(private_key)}. "
        "Pass DER bytes, a PEM or base64 string, or an RSAPrivateKey or EllipticCurvePrivateKey object."
    )
