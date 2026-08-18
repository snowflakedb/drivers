"""Integration tests for _common.wif_util.create_attestation against the real compiled core.

Unlike the other WIF integration tests in this directory, these do not go
through ``int_test_connection_factory`` — ``create_attestation`` talks
directly to cloud metadata / IdP endpoints, independent of any Snowflake
connection. Only the OIDC provider is exercised here: it is the one provider
that requires no outbound network call (the token is a pre-acquired,
structurally-validated passthrough), so it is the only path that can be
verified end-to-end without live cloud credentials.
"""

from __future__ import annotations

import pytest

from snowflake.connector.errors import ProgrammingError

from ...compatibility import IS_UNIVERSAL_DRIVER
from .conftest import make_dummy_jwt


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


class TestCreateAttestationOidc:
    def test_should_return_attestation_with_the_passthrough_token_as_credential(self):
        from snowflake.connector._common.wif_util import (
            AttestationProvider,
            WorkloadIdentityAttestation,
            create_attestation,
        )

        token = make_dummy_jwt(sub="test-service", iss="https://accounts.example.com")

        attestation = create_attestation(AttestationProvider.OIDC, token=token)

        assert isinstance(attestation, WorkloadIdentityAttestation)
        assert attestation.provider == AttestationProvider.OIDC
        assert attestation.credential == token

    def test_should_raise_when_token_is_missing(self):
        from snowflake.connector._common.wif_util import (
            AttestationProvider,
            create_attestation,
        )

        with pytest.raises(ProgrammingError, match="token"):
            create_attestation(AttestationProvider.OIDC)

    def test_should_raise_when_token_is_not_a_well_formed_jwt(self):
        from snowflake.connector._common.wif_util import (
            AttestationProvider,
            create_attestation,
        )

        with pytest.raises(ProgrammingError, match="JWT"):
            create_attestation(AttestationProvider.OIDC, token="not-a-jwt")
