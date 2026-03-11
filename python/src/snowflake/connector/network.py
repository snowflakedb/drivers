"""BACKWARD COMPATIBILITY MODULE ONLY — network layer stubs."""

from __future__ import annotations


class ReauthenticationRequest(Exception):
    """Signal that the connection must reauthenticate.

    Matches snowflake-connector-python network.ReauthenticationRequest.
    """

    def __init__(self, cause: Exception) -> None:
        self.cause = cause
        super().__init__(str(cause))
