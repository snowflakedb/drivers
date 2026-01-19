"""Utility functions for type tests."""

from collections.abc import Iterable


def assert_types(values: Iterable, expected_type: type, can_be_none: bool = False) -> None:
    """Assert all values in an iterable are of the expected type.

    Args:
        values: Iterable of values to check.
        expected_type: The expected type for all values.
        can_be_none: If True, None values are allowed.
    """
    for i, value in enumerate(values):
        if can_be_none and value is None:
            continue
        assert isinstance(value, expected_type), (
            f"Value at index {i} should be {expected_type.__name__}, got {type(value).__name__}"
        )
