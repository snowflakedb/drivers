"""Unit tests for normalize_private_key function."""

import base64

import pytest

from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa

from snowflake.connector._internal._private_key_helper import normalize_private_key
from snowflake.connector.exceptions import ProgrammingError


class TestPrivateKeyConversion:
    """Test suite for normalize_private_key function."""

    def test_normalize_private_key_from_bytes(self):
        """Test that bytes are returned as-is."""
        # Given a private key as bytes (DER format)
        private_key = rsa.generate_private_key(public_exponent=65537, key_size=2048, backend=default_backend())
        private_key_der = private_key.private_bytes(
            encoding=serialization.Encoding.DER,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )

        # When normalizing
        result = normalize_private_key(private_key_der)

        # Then the same bytes are returned
        assert result == private_key_der
        assert isinstance(result, bytes)

    def test_normalize_private_key_from_string(self):
        """Test that string is returned as-is (Rust handles base64 decoding)."""
        # Given a private key as base64-encoded string
        private_key = rsa.generate_private_key(public_exponent=65537, key_size=2048, backend=default_backend())
        private_key_der = private_key.private_bytes(
            encoding=serialization.Encoding.DER,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )
        private_key_base64 = base64.b64encode(private_key_der).decode()

        # When normalizing
        result = normalize_private_key(private_key_base64)

        # Then the string is returned as-is (Rust handles decoding)
        assert result == private_key_base64
        assert isinstance(result, str)

    def test_normalize_private_key_from_rsa_object(self):
        """Test that RSAPrivateKey object is serialized to bytes."""
        # Given a private key as RSAPrivateKey object
        private_key = rsa.generate_private_key(public_exponent=65537, key_size=2048, backend=default_backend())
        expected_der = private_key.private_bytes(
            encoding=serialization.Encoding.DER,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )

        # When normalizing
        result = normalize_private_key(private_key)

        # Then the serialized DER bytes are returned
        assert result == expected_der
        assert isinstance(result, bytes)

    def test_normalize_private_key_unsupported_type(self):
        """Test that unsupported type raises ProgrammingError."""
        # Given an unsupported type (int)
        invalid_key = 12345

        # When normalizing
        # Then ProgrammingError is raised
        with pytest.raises(ProgrammingError) as exc_info:
            normalize_private_key(invalid_key)

        assert "Unsupported private_key type" in str(exc_info.value)
        assert "int" in str(exc_info.value)

    def test_normalize_private_key_none(self):
        """Test that None raises ProgrammingError."""
        # Given None
        # When normalizing
        # Then ProgrammingError is raised
        with pytest.raises(ProgrammingError) as exc_info:
            normalize_private_key(None)

        assert "Unsupported private_key type" in str(exc_info.value)
