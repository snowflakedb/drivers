from __future__ import annotations

from typing import Any


_STAGE_PREFIXES = ("@", "snow://", "/")


def file_uri(path: str) -> str:
    """Add a ``file://`` prefix unless *path* already has one or is quoted."""
    stripped = path.strip()
    return stripped if stripped.startswith(("'", "file://")) else f"file://{stripped}"


def stage_ref(path: str) -> str:
    """Add an ``@`` prefix unless *path* already names a stage."""
    stripped = path.strip()
    return stripped if stripped.startswith(("'", *_STAGE_PREFIXES)) else f"@{stripped}"


def unquoted_stage_ref(path: str) -> str:
    """Add an ``@`` prefix and strip quoting; the streaming RPC takes the stage name as a field, not SQL."""
    stripped = path.strip()
    if len(stripped) > 1 and stripped.startswith("'") and stripped.endswith("'"):
        stripped = stripped[1:-1]
    return stripped if stripped.startswith(_STAGE_PREFIXES) else f"@{stripped}"


def put_get_options(options: dict[str, Any] | None) -> str:
    """Render an options mapping as trailing ``KEY=VALUE`` pairs, unquoted."""
    return " ".join(f"{name}={value}" for name, value in (options or {}).items())
