"""Connection type aliases shared by sync and async connection implementations."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any, TypeVar


SessionParameters = dict[str, Any]
ConnectionParamValue = int | str | float | bytes | bool | SessionParameters
ConnectionParameters = dict[str, ConnectionParamValue]

F = TypeVar("F", bound=Callable[..., Any])
