from __future__ import annotations

from typing import Any


_STAGE_PREFIXES = ("@", "snow://", "/")


def unquoted_stage_ref(path: str) -> str:
    """Add an ``@`` prefix and strip quoting; the streaming RPC takes the stage name as a field, not SQL."""
    stripped = path.strip()
    if len(stripped) > 1 and stripped.startswith("'") and stripped.endswith("'"):
        stripped = stripped[1:-1]
    return stripped if stripped.startswith(_STAGE_PREFIXES) else f"@{stripped}"


def stage_transfer_options(options: dict[str, Any] | None) -> dict[str, str]:
    """Stringify an options mapping the way the 4.x connector rendered it into PUT/GET SQL."""
    return {str(name): str(value) for name, value in (options or {}).items()}
