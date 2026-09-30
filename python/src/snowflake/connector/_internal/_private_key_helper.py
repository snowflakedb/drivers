"""Helper functions for private key conversion."""

from __future__ import annotations

from typing import Any

from ..errors import ProgrammingError
from ._distribution_profile import ALLOW_CRYPTOGRAPHY_PRIVATE_KEY_OBJECTS


def normalize_private_key(private_key: Any) -> bytes | str:
    """
    Normalize private_key to a format that can be sent to the Rust core.

    The Rust core handles:
    - bytes: DER format (sent via connection_set_option_bytes)
    - str: base64-encoded PEM or DER (sent via connection_set_option_string)

    bytes and str are passed through as-is. The ordinary distribution also
    serializes RSAPrivateKey objects; the FIPS-TLS candidate only accepts raw
    bytes and strings and never loads cryptography for key normalization.

    Args:
        private_key: Private key as DER bytes, a base64-encoded PEM/DER string,
                     or (in the ordinary distribution) an RSAPrivateKey object

    Returns:
        bytes or str: Private key ready to be sent to Rust core

    Raises:
        ProgrammingError: If the private_key type is not supported or conversion fails
    """
    if isinstance(private_key, (bytes, str)):
        # Pass through - Rust handles DER bytes and base64-encoded strings (PEM or DER)
        return private_key

    if not ALLOW_CRYPTOGRAPHY_PRIVATE_KEY_OBJECTS:
        raise ProgrammingError(
            f"Unsupported private_key type: {type(private_key)}. "
            "Cryptography key objects are not supported in the FIPS-TLS candidate; "
            "provide DER bytes or a base64-encoded string."
        )

    # Handle RSAPrivateKey object from cryptography library
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.rsa import RSAPrivateKey

    if isinstance(private_key, RSAPrivateKey):
        return private_key.private_bytes(
            encoding=serialization.Encoding.DER,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )
    raise ProgrammingError(
        f"Unsupported private_key type: {type(private_key)}. "
        "Expected bytes, str (base64-encoded), or RSAPrivateKey object."
    )
