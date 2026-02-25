"""Decorators for internal use."""

from typing import Callable, TypeVar


F = TypeVar("F", bound=Callable)


def internal_api(func: F) -> F:
    """
    Mark a method or function as internal.
    This is an identity function that returns the function unchanged.
    It serves as a marker for internal APIs that should not be used by external consumers.
    Args:
        func: The function or method to mark as internal
    Returns:
        The unchanged function
    """
    return func


def backward_compatibility(func: F) -> F:
    """Mark a method as backward compatibility utility"""
    return func


def pep249(func: F) -> F:
    """Mark a method or property as defined by PEP 249 (required or optional)."""
    return func
