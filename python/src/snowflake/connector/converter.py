"""Converters that customise how fetched values are turned into Python objects.

Pass a subclass of :class:`ArrowConverterContext` as ``converter_class`` to ``connect``.
"""

from __future__ import annotations

from ._internal.arrow_context import ArrowConverterContext


__all__ = ["ArrowConverterContext"]
