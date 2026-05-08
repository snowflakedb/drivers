"""Backward-compatible SnowflakeConverter stub.

.. deprecated::
    This module is a no-op compatibility shim.  The universal driver performs
    data conversion in its C++ Arrow layer and Rust core; the
    ``SnowflakeConverter`` class here has no effect on conversion behaviour and
    will be removed in a future version.

The module exists so that:

* ``connection.converter_class`` returns a recognisable type,
* ``from snowflake.connector.converter import SnowflakeConverter`` keeps
  working, and
* users can still pass custom ``converter_class`` subclasses as a connection
  parameter without raising an error.
"""

from __future__ import annotations

from ._internal.backward_compatibility import install_backward_compatibility_getattr
from ._internal.decorators import backward_compatibility


@backward_compatibility
class SnowflakeConverter:
    """No-op converter stub retained for backward compatibility.

    .. deprecated::
        This class has no effect in the universal driver and will be removed
        in a future version.
    """

    def __init__(self, **kwargs: object) -> None:
        pass


# Must be the last statement; see ``install_backward_compatibility_getattr``.
install_backward_compatibility_getattr(__name__)
