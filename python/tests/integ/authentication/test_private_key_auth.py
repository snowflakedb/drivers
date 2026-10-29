import base64
import hashlib
import json
import os
import tempfile

import pytest

from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import encode_dss_signature

from snowflake.connector import ProgrammingError

from ...compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY


_EC_CURVES = (
    (ec.SECP256R1(), "ES256", hashes.SHA256()),
    (ec.SECP384R1(), "ES384", hashes.SHA384()),
    (ec.SECP521R1(), "ES512", hashes.SHA512()),
)


class TestPrivateKeyAuthentication:
    def test_should_fail_jwt_authentication_when_no_private_file_provided(self, int_test_connection_factory):
        # Given Authentication is set to JWT
        authenticator = "SNOWFLAKE_JWT"

        # When Trying to Connect with no private file provided
        with pytest.raises(Exception) as exception_info:
            int_test_connection_factory(authenticator=authenticator, private_key_file=None)

        exception = exception_info.value

        # Then There is error returned
        assert exception is not None

        if NEW_DRIVER_ONLY("BD#4"):
            assert isinstance(exception, ProgrammingError)
            assert "Missing required parameter: 'private_key' or 'private_key_file'" in str(exception)
        elif OLD_DRIVER_ONLY("BD#4"):
            assert isinstance(exception, TypeError)
            error_msg = str(exception)
            assert "Expected bytes" in error_msg and "RSAPrivateKey" in error_msg

    def test_should_sign_jwt_with_the_algorithm_matching_the_ecdsa_key_curve(
        self, int_test_connection_factory, wiremock
    ):
        wiremock.add_mapping("auth/login_success_jwt.json")
        for curve, alg, digest in _EC_CURVES:
            private_key = ec.generate_private_key(curve)
            pem = private_key.private_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PrivateFormat.PKCS8,
                encryption_algorithm=serialization.NoEncryption(),
            )
            # Given Authentication is set to JWT and an ECDSA private key file is provided
            with tempfile.NamedTemporaryFile(suffix=".p8") as key_file:
                os.chmod(key_file.name, 0o600)
                key_file.write(pem)
                key_file.flush()
                # When Trying to Connect
                connection = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    authenticator="SNOWFLAKE_JWT",
                    private_key_file=key_file.name,
                )
            # Then The login request carries a JWT signed with the curve's algorithm that verifies under the public key
            try:
                _assert_login_jwt_verifies(wiremock, private_key.public_key(), alg, digest)
            finally:
                connection.close()

    def test_should_authenticate_using_private_key_as_ellipticcurveprivatekey_object(
        self, int_test_connection_factory, wiremock
    ):
        wiremock.add_mapping("auth/login_success_jwt.json")
        for curve, alg, digest in _EC_CURVES:
            # Given Authentication is set to JWT and private key is provided as EllipticCurvePrivateKey object
            private_key = ec.generate_private_key(curve)
            # When Trying to Connect
            connection = int_test_connection_factory(
                server_url=wiremock.http_url(),
                authenticator="SNOWFLAKE_JWT",
                private_key=private_key,
                private_key_file=None,
            )
            # Then The login request carries a JWT signed with the curve's algorithm that verifies under the public key
            try:
                _assert_login_jwt_verifies(wiremock, private_key.public_key(), alg, digest)
            finally:
                connection.close()


def _assert_login_jwt_verifies(wiremock, public_key, alg, digest) -> None:
    login_requests = wiremock.get_requests("/session/v1/login-request.*")
    assert login_requests, f"{alg}: expected a login request"
    token = json.loads(login_requests[-1]["body"])["data"]["TOKEN"]
    header_segment, claims_segment, signature_segment = token.split(".")
    header = json.loads(_b64url_decode(header_segment))
    claims = json.loads(_b64url_decode(claims_segment))
    assert header["alg"] == alg

    spki = public_key.public_bytes(
        serialization.Encoding.DER,
        serialization.PublicFormat.SubjectPublicKeyInfo,
    )
    fingerprint = base64.b64encode(hashlib.sha256(spki).digest()).decode()
    assert claims["sub"] == "TEST_ACCOUNT.TEST_USER"
    assert claims["iss"] == f"TEST_ACCOUNT.TEST_USER.SHA256:{fingerprint}"

    raw = _b64url_decode(signature_segment)
    half = len(raw) // 2
    der_signature = encode_dss_signature(
        int.from_bytes(raw[:half], "big"),
        int.from_bytes(raw[half:], "big"),
    )
    public_key.verify(der_signature, f"{header_segment}.{claims_segment}".encode(), ec.ECDSA(digest))


def _b64url_decode(segment: str) -> bytes:
    return base64.urlsafe_b64decode(segment + "=" * (-len(segment) % 4))
