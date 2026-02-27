"""BACKWARD COMPATIBILITY MODULE ONLY"""

from . import by_plugin  # noqa: F401 - needed for attribute access


class AuthByPlugin:
    pass


class AuthNoAuth(AuthByPlugin):
    pass


class AuthByWorkloadIdentity(AuthByPlugin):
    pass
