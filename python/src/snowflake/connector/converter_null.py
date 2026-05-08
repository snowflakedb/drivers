"""Backward-compatible SnowflakeNoConverterToPython stub.

.. deprecated::
    This module is a no-op compatibility shim.  In the old connector, passing
    ``converter_class=SnowflakeNoConverterToPython`` bypassed Python-side type
    conversion.  In the universal driver this class has no effect on conversion
    behaviour and will be removed in a future version.
"""

from __future__ import annotations

from ._internal.backward_compatibility import install_backward_compatibility_getattr
from ._internal.decorators import backward_compatibility
from .converter import SnowflakeConverter as _SnowflakeConverter


@backward_compatibility
class SnowflakeNoConverterToPython(_SnowflakeConverter):
    """No-op converter stub that mirrors the old connector's null converter.

    .. deprecated::
        This class has no effect in the universal driver and will be removed
        in a future version.
    """

    def __init__(self, **kwargs: object) -> None:
        super().__init__(**kwargs)


# Must be the last statement; see ``install_backward_compatibility_getattr``.
install_backward_compatibility_getattr(__name__)
