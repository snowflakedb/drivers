"""Helper functions for private key conversion."""

from __future__ import annotations

from typing import Any

from ..exceptions import ProgrammingError


def normalize_private_key(private_key: Any) -> bytes | str:
    """
    Normalize private_key to a format that can be sent to the Rust core.

    The Rust core handles:
    - bytes: DER format (sent via connection_set_option_bytes)
    - str: base64-encoded PEM or DER (sent via connection_set_option_string)

    This function only converts RSAPrivateKey objects to bytes.
    bytes and str are passed through as-is.

    Args:
        private_key: Private key in one of the supported formats:
                     - bytes (DER format)
                     - str (base64-encoded PEM or DER)
                     - RSAPrivateKey object

    Returns:
        bytes or str: Private key ready to be sent to Rust core

    Raises:
        ProgrammingError: If the private_key type is not supported or conversion fails
    """
    if isinstance(private_key, (bytes, str)):
        # Pass through - Rust handles DER bytes and base64-encoded strings (PEM or DER)
        return private_key

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
