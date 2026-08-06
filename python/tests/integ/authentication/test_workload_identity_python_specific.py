"""Python-wrapper-specific WIF integration tests.

Covers Python-level normalisation and validation that has no corresponding
Gherkin scenario in the shared ``workload_identity.feature``:

* ``workload_identity_impersonation_path`` accepted as ``list[str]`` (legacy API).
* ``authenticator="workload_identity"`` (lowercase) is not rejected.
* ``user=`` is optional for WORKLOAD_IDENTITY.
* WIF-specific params with a non-WIF authenticator raise an error.
* ``workload_identity_impersonation_path`` with provider=OIDC raises an error.
* All four provider strings (AWS/AZURE/GCP/OIDC) are forwarded to sf_core
  without being rejected as unknown at the wrapper layer.

These tests exercise the Python wrapper's ``from_kwargs()`` normalisation path
without requiring cloud credentials — the connection always fails at the
attestation / network layer, and the assertions verify which layer rejected it.
"""

from __future__ import annotations

import pytest

from snowflake.connector.errors import ProgrammingError

from ...compatibility import IS_UNIVERSAL_DRIVER
from .conftest import connect_expecting_error, full_error_text, make_dummy_jwt


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


class TestWorkloadIdentityApiNormalisation:
    """Python-wrapper-level behaviour: list→str coercion, case-insensitivity."""

    def test_should_accept_impersonation_path_as_list(self, int_test_connection_factory):
        # Given workload_identity_impersonation_path is supplied as list[str] (legacy API)
        kwargs = {
            "authenticator": "WORKLOAD_IDENTITY",
            "workload_identity_provider": "AWS",
            "workload_identity_impersonation_path": ["arn:aws:iam::123456789012:role/A"],
        }

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then The wrapper does not raise a ProgrammingError for the list type
        # (the connection fails at the network / attestation layer, not the param layer)
        text = full_error_text(exception)
        assert "unsupported connection option type" not in text.lower()
        assert "list" not in text.lower()

    def test_should_accept_lowercase_workload_identity_authenticator(self, int_test_connection_factory):
        # Given Authentication is set to workload_identity (lowercase) and a valid provider is configured
        kwargs = {
            "authenticator": "workload_identity",
            "workload_identity_provider": "AWS",
        }

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then The wrapper does not reject the authenticator value
        text = full_error_text(exception)
        assert "invalid authenticator" not in text.lower()
        assert "unknown authenticator" not in text.lower()

    def test_should_accept_workload_identity_without_user(self, int_test_connection_factory):
        # Given Authentication is set to WORKLOAD_IDENTITY without a user
        kwargs = {
            "authenticator": "WORKLOAD_IDENTITY",
            "workload_identity_provider": "AWS",
        }

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then The wrapper does not raise a missing-parameter error for user
        text = full_error_text(exception)
        assert not ("missing" in text.lower() and "user" in text.lower()), (
            f"Should not require user for WORKLOAD_IDENTITY: {text}"
        )


# ---------------------------------------------------------------------------
# Dependent-parameter guards
# ---------------------------------------------------------------------------


class TestWorkloadIdentityDependentParamGuards:
    """WIF-specific params must not silently accept a non-WIF authenticator."""

    @pytest.mark.parametrize(
        "extra_kwarg,value",
        [
            ("workload_identity_provider", "AWS"),
            ("workload_identity_entra_resource", "api://00000000-0000-0000-0000-000000000001"),
            ("workload_identity_impersonation_path", "arn:aws:iam::123456789012:role/A"),
        ],
    )
    def test_should_fail_when_wif_param_set_without_wif_authenticator(
        self, extra_kwarg, value, int_test_connection_factory
    ):
        # Local import: pytestmark's skipif shields this from the reference driver, which lacks `_internal`.
        from snowflake.connector._internal.errorcode import ER_INVALID_WIF_SETTINGS

        # Given a WIF-specific param is set but authenticator is snowflake (not WORKLOAD_IDENTITY)
        kwargs = {
            "authenticator": "snowflake",
            "user": "test_user",
            "password": "test_password",
            # int_test_connection_factory defaults to private_key_file + SNOWFLAKE_JWT;
            # clearing the key prevents a 251007 conflict that would mask the WIF validation error.
            "private_key_file": None,
            extra_kwarg: value,
        }

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then Connection fails with an error that mentions the offending WIF param
        text = full_error_text(exception)
        assert extra_kwarg.lower() in text.lower(), (
            f"Expected error to mention {extra_kwarg!r} when set without WORKLOAD_IDENTITY "
            f"authenticator, but got: {text}"
        )

        # The wrapper re-maps sf_core's generic InvalidParameterValue (raised by
        # connection_init, not connection_set_options) to the legacy WIF errno, matching
        # snowflake-connector-python's ProgrammingError (errno 251017).
        assert isinstance(exception, ProgrammingError), f"Expected ProgrammingError, got {exception!r}"
        assert exception.errno == ER_INVALID_WIF_SETTINGS, (
            f"Expected errno ER_INVALID_WIF_SETTINGS ({ER_INVALID_WIF_SETTINGS}), got {exception.errno}: {text}"
        )
        # The re-raise preserves the original core error via `from e` (not `from None`),
        # so callers can still inspect the underlying cause.
        assert exception.__cause__ is not None, "WIF errno re-map should chain the original core error"


# ---------------------------------------------------------------------------
# Provider-specific restrictions
# ---------------------------------------------------------------------------


class TestWorkloadIdentityProviderRestrictions:
    """Cross-field validation: impersonation_path is unsupported for OIDC."""

    def test_should_fail_oidc_wif_when_impersonation_path_is_set(self, int_test_connection_factory):
        # Local import: pytestmark's skipif shields this from the reference driver, which lacks `_internal`.
        from snowflake.connector._internal.errorcode import ER_INVALID_WIF_SETTINGS

        # Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC
        kwargs = {
            "authenticator": "WORKLOAD_IDENTITY",
            "workload_identity_provider": "OIDC",
            "token": make_dummy_jwt(),
        }
        # And workload_identity_impersonation_path is also set (unsupported for OIDC)
        kwargs["workload_identity_impersonation_path"] = "sa@project.iam.gserviceaccount.com"

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then Connection fails with the impersonation-path restriction error (cites the param name)
        text = full_error_text(exception)
        assert "workload_identity_impersonation_path" in text.lower(), (
            f"Expected error citing 'workload_identity_impersonation_path', got: {text}"
        )

        # This check now lives in sf_core alongside the other WIF cross-param guards
        # (see TestWorkloadIdentityDependentParamGuards), so it gets the same legacy
        # errno and cause-chaining treatment.
        assert isinstance(exception, ProgrammingError), f"Expected ProgrammingError, got {exception!r}"
        assert exception.errno == ER_INVALID_WIF_SETTINGS, (
            f"Expected errno ER_INVALID_WIF_SETTINGS ({ER_INVALID_WIF_SETTINGS}), got {exception.errno}: {text}"
        )
        assert exception.__cause__ is not None, "WIF errno re-map should chain the original core error"


# ---------------------------------------------------------------------------
# Param forwarding
# ---------------------------------------------------------------------------


class TestWorkloadIdentityParamForwarding:
    """All four provider strings and ancillary WIF params reach sf_core."""

    @pytest.mark.parametrize(
        "provider,extra",
        [
            ("AWS", {}),
            ("AZURE", {}),
            ("GCP", {}),
            ("OIDC", {"token": make_dummy_jwt()}),
        ],
    )
    def test_should_accept_all_four_wif_provider_strings(self, provider, extra, int_test_connection_factory):
        # Given WORKLOAD_IDENTITY_PROVIDER is set to a valid string value
        kwargs = {
            "authenticator": "WORKLOAD_IDENTITY",
            "workload_identity_provider": provider,
            **extra,
        }

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then The provider value is accepted (failure is at attestation/network, not param layer)
        text = full_error_text(exception)
        assert "unknown workload_identity_provider" not in text.lower(), (
            f"Provider {provider!r} was rejected at the param layer: {text}"
        )

    def test_should_accept_workload_identity_entra_resource(self, int_test_connection_factory):
        # Given workload_identity_entra_resource is set alongside the AZURE provider
        kwargs = {
            "authenticator": "WORKLOAD_IDENTITY",
            "workload_identity_provider": "AZURE",
            "workload_identity_entra_resource": "api://00000000-0000-0000-0000-000000000001",
        }

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then The entra_resource is accepted (failure is at attestation/network, not param layer)
        text = full_error_text(exception)
        # entra_resource is valid for AZURE, so it must not be rejected at the param layer. A
        # param-layer rejection is a ProgrammingError that names the offending param; attestation
        # /network failures (the harness has no real IdP) do not cite it — and may themselves be
        # ProgrammingError (sf_core maps INTERNAL_ERROR / INVALID_ARGUMENT to ProgrammingError),
        # so a bare isinstance check would misfire on a genuine attestation error.
        assert not (isinstance(exception, ProgrammingError) and "workload_identity_entra_resource" in text.lower()), (
            f"entra_resource was rejected at the param layer for AZURE: {text}"
        )

    def test_should_forward_valid_oidc_token_past_param_validation(self, int_test_connection_factory):
        # Given a structurally-valid JWT is provided for the OIDC provider
        kwargs = {
            "authenticator": "WORKLOAD_IDENTITY",
            "workload_identity_provider": "OIDC",
            "token": make_dummy_jwt(sub="svc", iss="https://accounts.example.com"),
        }

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then The token passes format checks and reaches sf_core (failure is at attestation, not format)
        text = full_error_text(exception)
        assert "token must be provided" not in text.lower(), f"Token was not forwarded past param validation: {text}"
        assert "invalid jwt" not in text.lower(), f"Structurally-valid JWT was rejected at format-check layer: {text}"
