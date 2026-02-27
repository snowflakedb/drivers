"""BACKWARD COMPATIBILITY MODULE ONLY"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


DEFAULT_ENTRA_SNOWFLAKE_RESOURCE = "api://fd3f753b-eed3-462c-b6a7-a4b5bb650aad"


class AttestationProvider(Enum):
    AWS = "AWS"
    AZURE = "AZURE"
    GCP = "GCP"
    OIDC = "OIDC"

    @classmethod
    def from_string(cls, value: str) -> AttestationProvider:
        return cls(value.upper())


@dataclass
class WorkloadIdentityAttestation:
    provider: AttestationProvider
    credential: str
    user_identifier_components: dict
