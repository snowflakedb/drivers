"""Unit tests for snowflake.connector._common.wif_util.create_attestation."""

from unittest.mock import MagicMock

import pytest

from snowflake.connector._common.wif_util import (
    DEFAULT_ENTRA_SNOWFLAKE_RESOURCE,
    AttestationProvider,
    WorkloadIdentityAttestation,
    create_attestation,
)
from snowflake.connector.errors import ProgrammingError


class TestAttestationProviderFromString:
    def test_parses_each_known_provider(self):
        for value in ("AWS", "AZURE", "GCP", "OIDC"):
            assert AttestationProvider.from_string(value).value == value

    def test_is_case_insensitive(self):
        assert AttestationProvider.from_string("aws") == AttestationProvider.AWS

    def test_unknown_provider_raises_programming_error(self):
        with pytest.raises(ProgrammingError, match="Unknown workload_identity_provider"):
            AttestationProvider.from_string("NOT_A_PROVIDER")


class TestCreateAttestation:
    def test_returns_attestation_matching_legacy_shape(self, mock_db_api):
        mock_db_api.wif_create_attestation.return_value = MagicMock(provider="AWS", credential="fake-credential")

        attestation = create_attestation(AttestationProvider.AWS)

        assert isinstance(attestation, WorkloadIdentityAttestation)
        assert attestation.provider == AttestationProvider.AWS
        assert attestation.credential == "fake-credential"
        assert attestation.user_identifier_components == {}

    def test_defaults_entra_resource_when_unset(self, mock_db_api):
        mock_db_api.wif_create_attestation.return_value = MagicMock(provider="AZURE", credential="fake-credential")

        create_attestation(AttestationProvider.AZURE)

        request = mock_db_api.wif_create_attestation.call_args[0][0]
        assert request.provider == "AZURE"
        assert not request.HasField("entra_resource")

    def test_passes_explicit_entra_resource_and_token(self, mock_db_api):
        mock_db_api.wif_create_attestation.return_value = MagicMock(provider="OIDC", credential="fake-credential")

        create_attestation(
            AttestationProvider.OIDC,
            entra_resource="api://custom-resource",
            token="fake-oidc-token",
        )

        request = mock_db_api.wif_create_attestation.call_args[0][0]
        assert request.provider == "OIDC"
        assert request.entra_resource == "api://custom-resource"
        assert request.token == "fake-oidc-token"

    def test_forwards_impersonation_path_to_request(self, mock_db_api):
        mock_db_api.wif_create_attestation.return_value = MagicMock(provider="AWS", credential="fake-credential")

        create_attestation(
            AttestationProvider.AWS,
            impersonation_path=["arn:aws:iam::123456789012:role/MyRole"],
        )

        request = mock_db_api.wif_create_attestation.call_args[0][0]
        assert list(request.impersonation_path) == ["arn:aws:iam::123456789012:role/MyRole"]

    def test_impersonation_path_accepted_positionally_like_legacy(self, mock_db_api):
        """The legacy connector's WIF plugin calls create_attestation() with
        impersonation_path as the 4th positional argument; that slot must
        still mean impersonation_path, not session_manager."""
        mock_db_api.wif_create_attestation.return_value = MagicMock(provider="AWS", credential="fake-credential")

        create_attestation(
            AttestationProvider.AWS,
            None,
            None,
            ["arn:aws:iam::123456789012:role/MyRole"],
        )

        request = mock_db_api.wif_create_attestation.call_args[0][0]
        assert list(request.impersonation_path) == ["arn:aws:iam::123456789012:role/MyRole"]

    def test_rejects_fifth_positional_argument(self, mock_db_api):
        """Everything past impersonation_path is keyword-only, so a 5th
        positional argument must fail loudly instead of being silently
        reinterpreted."""
        with pytest.raises(TypeError):
            create_attestation(AttestationProvider.AWS, None, None, [], object())

    def test_session_manager_is_no_longer_accepted(self, mock_db_api):
        """session_manager was removed entirely rather than accepted-and-ignored."""
        with pytest.raises(TypeError, match="session_manager"):
            create_attestation(AttestationProvider.GCP, session_manager=object())

    def test_default_entra_resource_constant_has_expected_value(self):
        # Pinned so an accidental change (e.g. drift from the Rust-side default) is caught.
        assert DEFAULT_ENTRA_SNOWFLAKE_RESOURCE == "api://fd3f753b-eed3-462c-b6a7-a4b5bb650aad"


class TestCreateAttestationKwargsCompat:
    def test_aws_use_outbound_token_is_no_longer_accepted(self, mock_db_api):
        """aws_use_outbound_token was removed entirely rather than accepted-and-ignored."""
        with pytest.raises(TypeError, match="aws_use_outbound_token"):
            create_attestation(AttestationProvider.AWS, aws_use_outbound_token=True)

    def test_unknown_kwarg_raises_type_error(self, mock_db_api):
        with pytest.raises(TypeError, match="unknown_kwarg"):
            create_attestation(AttestationProvider.AWS, unknown_kwarg=1)
