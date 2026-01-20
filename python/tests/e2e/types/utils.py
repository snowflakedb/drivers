"""Utility functions for type tests."""

from __future__ import annotations

from collections.abc import Iterable
from math import isinf, isnan


# Minimum normalized positive value (smallest normal number)
# Used for tolerance selection in float comparisons
FLOAT_MIN_NORMAL = 2.2250738585072014e-308


def assert_type(values: Iterable, expected_type: type, can_be_none: bool = False) -> None:
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


def assert_float_equal(actual: float, expected: float | None, msg: str = "") -> None:
    """Assert two float values are equal within appropriate tolerance.

    Selects comparison strategy based on IEEE 754 value magnitude:
    - Subnormal (|x| < 2.2e-308): absolute tolerance 1e-325
    - Large (|x| > 1e10): relative tolerance 1e-14
    - Regular: absolute tolerance 1e-10
    """
    error_msg = msg or f"Expected {expected}, got {actual}"

    # None
    if expected is None:
        assert actual is None, error_msg
        return
    # NaN
    if isnan(expected):
        assert isnan(actual), error_msg
        return
    # inf, -inf
    if isinf(expected):
        assert actual == expected, error_msg
        return

    abs_expected = abs(expected)
    diff = abs(actual - expected)

    # Subnormal range (very small numbers near minimum representable)
    if abs_expected < FLOAT_MIN_NORMAL:
        assert diff <= 1e-325, error_msg
    # Large values - use relative tolerance for ~15 digit precision
    elif abs_expected > 1e10:
        assert diff <= abs_expected * 1e-14, error_msg
    # Regular values - absolute tolerance
    else:
        assert diff < 1e-10, error_msg


def assert_floats_equal(actual: Iterable[float], expected: Iterable[float]) -> None:
    """Assert two iterables of float values are equal element-wise.

    Calls assert_float_equal on consecutive elements.
    """
    actual_list = list(actual)
    expected_list = list(expected)
    assert len(actual_list) == len(expected_list), f"Length mismatch: {len(actual_list)} != {len(expected_list)}"
    for i, (a, e) in enumerate(zip(actual_list, expected_list)):
        assert_float_equal(a, e, f"Mismatch at index {i}: expected {e}, got {a}")
