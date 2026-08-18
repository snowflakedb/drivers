from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, unique

from .._internal.api_client.client_api import core_driver
from .._internal.decorators import snowpark_compat
from .._internal.errorcode import ER_INVALID_WIF_SETTINGS
from ..errors import ProgrammingError


# Kept for legacy API compatibility; the Rust core uses its own copy of this constant.
DEFAULT_ENTRA_SNOWFLAKE_RESOURCE = "api://fd3f753b-eed3-462c-b6a7-a4b5bb650aad"


@unique
class AttestationProvider(Enum):
    """A WIF provider implementation that can produce an attestation."""

    AWS = "AWS"
    AZURE = "AZURE"
    GCP = "GCP"
    OIDC = "OIDC"

    @staticmethod
    def from_string(provider: str) -> AttestationProvider:
        """Convert a string to a strongly-typed enum value of AttestationProvider."""
        try:
            return AttestationProvider[provider.upper()]
        except KeyError:
            allowed = ", ".join(AttestationProvider.all_string_values())
            raise ProgrammingError(
                msg=f"Unknown workload_identity_provider: '{provider}'. Expected one of: {allowed}",
                errno=ER_INVALID_WIF_SETTINGS,
            ) from None

    @staticmethod
    def all_string_values() -> list[str]:
        """Return a list of all string values of the AttestationProvider enum."""
        return [provider.value for provider in AttestationProvider]


@dataclass
class WorkloadIdentityAttestation:
    provider: AttestationProvider
    # repr=False: this is a bearer credential and must not leak into
    # tracebacks, logs, or debugger output via the default dataclass repr.
    credential: str = field(repr=False)
    # Always empty; sf_core doesn't yet surface per-provider identifier metadata
    # (AWS region/partition, GCP/Azure/OIDC iss/sub claims).
    user_identifier_components: dict = field(default_factory=dict)


@snowpark_compat
def create_attestation(
    provider: AttestationProvider,
    entra_resource: str | None = None,
    token: str | None = None,
    impersonation_path: list[str] | None = None,
) -> WorkloadIdentityAttestation:
    """Acquires a Workload Identity Federation attestation for the given provider.

    Delegates to the Rust core's attestation logic (the same code path used by
    the driver's own WIF login flow), independent of any active connection.

    If an explicit entra_resource is provided it is forwarded to the Rust core;
    otherwise the Rust core applies its own default for the Azure Entra resource.

    `impersonation_path` is forwarded to the Rust core, which chains
    `sts:AssumeRole` calls (AWS), delegate impersonation (GCP), or a
    single-hop service-principal token exchange (Azure) exactly as the
    driver's own WIF login flow does.

    Any other keyword argument raises `TypeError`.
    """
    response = core_driver.wif_create_attestation(
        provider=provider.value,
        entra_resource=entra_resource,
        token=token,
        impersonation_path=impersonation_path or [],
    )
    return WorkloadIdentityAttestation(
        provider=AttestationProvider.from_string(response.provider),
        credential=response.credential,
    )
