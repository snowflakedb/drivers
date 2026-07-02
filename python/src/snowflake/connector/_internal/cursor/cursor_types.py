"""Shared type aliases and type variables for sync and async cursors."""

from __future__ import annotations

from collections.abc import Callable, Sequence
from typing import Any, TypeVar


Row = tuple[Any, ...]
DictRow = dict[str, Any]

F = TypeVar("F", bound=Callable[..., Any])
Args = TypeVar("Args", bound=Sequence[Any])
