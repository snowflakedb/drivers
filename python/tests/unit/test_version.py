"""
Unit tests for version module.

Verifies that the PEP 440 compliant ``__version__`` string and the legacy
``VERSION`` tuple (kept for backward compatibility with the old Python
connector) stay in sync.
"""

import re

import pytest

import snowflake.connector as pep249_dbapi

from snowflake.connector.version import VERSION, __version__, _release_components


# https://packaging.python.org/en/latest/specifications/version-specifiers/#public-version-identifiers
PEP440_RE = re.compile(
    r"^([1-9][0-9]*!)?"  # optional epoch
    r"(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))*"  # release segment
    r"((a|b|rc)(0|[1-9][0-9]*))?"  # pre-release
    r"(\.post(0|[1-9][0-9]*))?"  # post-release
    r"(\.dev(0|[1-9][0-9]*))?"  # dev release
    r"(\+[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*)?$"  # local version
)


class TestVersionString:
    """Tests for the PEP 440 compliant ``__version__`` string."""

    def test_version_is_string(self):
        assert isinstance(__version__, str)
        assert __version__

    def test_version_is_pep440_compliant(self):
        assert PEP440_RE.match(__version__), f"__version__={__version__!r} is not PEP 440 compliant"

    def test_version_exported_at_package_level(self):
        assert hasattr(pep249_dbapi, "__version__")
        assert pep249_dbapi.__version__ == __version__

    def test_version_in_all(self):
        assert "__version__" in pep249_dbapi.__all__


class TestVersionTuple:
    """Tests for the legacy ``VERSION`` tuple kept for backward compatibility."""

    def test_version_tuple_last_element_is_none(self):
        # Matches the legacy (major, minor, patch, build) shape where the last
        # slot was historically reserved for a build identifier. The new driver
        # always sets it to ``None``.
        assert VERSION[-1] is None

    def test_version_tuple_release_components_are_ints(self):
        for component in VERSION[:-1]:
            assert isinstance(component, int)
            assert component >= 0


class TestReleaseComponents:
    """Tests for the ``_release_components`` helper."""

    @pytest.mark.parametrize(
        "version_string,expected",
        [
            ("1.2.3", (1, 2, 3)),
            ("5.0.0", (5, 0, 0)),
            ("5.0.0b1", (5, 0, 0)),
            ("5.0.0rc2", (5, 0, 0)),
            ("5.0.0a10", (5, 0, 0)),
            ("2.7.9.post1", (2, 7, 9)),
            ("3.1.0.dev1", (3, 1, 0)),
            ("10", (10,)),
            ("1.0", (1, 0)),
            ("1.0.0.0", (1, 0, 0, 0)),
        ],
    )
    def test_release_components_parses_known_versions(self, version_string, expected):
        assert _release_components(version_string) == expected

    @pytest.mark.parametrize(
        "version_string",
        ["", "abc", "a.b.c"],
    )
    def test_release_components_handles_non_numeric_prefix(self, version_string):
        assert _release_components(version_string) == ()

    def test_release_components_stops_at_non_numeric_segment(self):
        # Once a segment contains trailing non-numeric characters (e.g. a
        # pre-release tag), parsing stops after that numeric prefix.
        assert _release_components("1.2.3b4.5") == (1, 2, 3)
