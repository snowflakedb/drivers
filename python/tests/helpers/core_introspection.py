"""Shared Core introspection utilities for test fixtures.

Provides CoreIntrospector — a wrapper around a MagicMock-spied db_api that
extracts what was sent to Core via connection_set_options RPCs. Used by both
integ fixtures (core_mock) and e2e fixtures (core_proxy).
"""

from __future__ import annotations

from typing import Any
from unittest.mock import MagicMock


class CoreIntrospector:
    """Inspect what was sent to Core via set-option RPCs at init time.

    Reads from the batch connection_set_options RPC (the only option-setting
    path used by the connector). Does NOT capture close-time overrides
    passed via ConnectionCloseRequest proto fields — those are tested via
    WireMock request counts (behavioral outcome).
    """

    def __init__(self, spy: MagicMock) -> None:
        self.db_api = spy

    def get_options_sent(self) -> dict[str, Any]:
        """Extract key->value pairs from batch connection_set_options RPC."""
        options: dict[str, Any] = {}
        for call in self.db_api.connection_set_options.call_args_list:
            req = call.args[0]
            for key, setting in req.options.items():
                field = setting.WhichOneof("value")
                if field:
                    options[key] = getattr(setting, field)
        return options
