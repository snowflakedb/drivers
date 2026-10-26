"""Import-time GIL contract for free-threaded CPython.

Importing the connector on a free-threaded interpreter must not re-enable
the GIL. The checks run in a subprocess: pytest has already imported the
connector, so an in-process import hides the first-import GIL state.
"""

from __future__ import annotations

import subprocess
import sys
import sysconfig
import textwrap

import pytest


def _is_freethreaded_build() -> bool:
    """True for a free-threaded build, whether or not the GIL is on right now."""
    return bool(sysconfig.get_config_var("Py_GIL_DISABLED"))


@pytest.fixture
def require_gil_enabled():
    """Skip unless this interpreter is a default GIL build that reports GIL state.

    ``sys._is_gil_enabled`` exists on Python 3.13 and newer. Earlier
    interpreters always hold the GIL, and the probe this fixture guards
    cannot run there.
    """
    if _is_freethreaded_build():
        pytest.skip("requires a GIL-enabled build (for example python3.15, not python3.15t)")
    if not hasattr(sys, "_is_gil_enabled"):
        pytest.skip("sys._is_gil_enabled exists on Python 3.13 and newer")


@pytest.fixture
def require_gil_disabled():
    """Skip unless this interpreter is a free-threaded build."""
    if not _is_freethreaded_build():
        pytest.skip("requires a free-threaded build (for example python3.15t)")


def _run_in_subprocess(script: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, "-W", "always", "-c", textwrap.dedent(script)],
        capture_output=True,
        text=True,
        check=False,
    )


def test_connector_import_does_not_emit_gil_reenable_warning(require_gil_disabled) -> None:
    """A free-threaded import must not warn that the GIL was turned back on."""
    proc = _run_in_subprocess(
        """
        import warnings
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter('always')
            import snowflake.connector  # noqa: F401
            import snowflake.connector._internal.arrow_stream_iterator  # noqa: F401
            from snowflake.connector._core import sf_core_python  # noqa: F401
        gil_warnings = [
            (w.category.__name__, str(w.message))
            for w in caught
            if issubclass(w.category, RuntimeWarning)
            and 'global interpreter lock' in str(w.message).lower()
        ]
        if gil_warnings:
            import sys
            for cat, msg in gil_warnings:
                print(f'GIL_WARNING: {cat}: {msg}', file=sys.stderr)
            sys.exit(2)
        """
    )
    assert proc.returncode == 0, (
        f"Connector import emitted a GIL re-enable warning (subprocess exit {proc.returncode}). "
        "A compiled extension is missing Cython's freethreading_compatible directive "
        "or PyO3's gil_used = false.\n"
        f"--- subprocess stderr ---\n{proc.stderr}\n"
        f"--- subprocess stdout ---\n{proc.stdout}"
    )


def test_connector_import_leaves_gil_disabled_on_freethreaded_build(require_gil_disabled) -> None:
    """On a free-threaded build, the GIL must still be disabled after import."""
    proc = _run_in_subprocess(
        """
        import sys
        import snowflake.connector  # noqa: F401
        import snowflake.connector._internal.arrow_stream_iterator  # noqa: F401
        from snowflake.connector._core import sf_core_python  # noqa: F401
        sys.exit(0 if sys._is_gil_enabled() is False else 1)
        """
    )
    assert proc.returncode == 0, (
        f"GIL was enabled after importing the connector on a free-threaded interpreter "
        f"(subprocess exit {proc.returncode}).\n"
        f"--- subprocess stderr ---\n{proc.stderr}\n"
        f"--- subprocess stdout ---\n{proc.stdout}"
    )


def test_connector_import_on_gil_build_leaves_gil_enabled(require_gil_enabled) -> None:
    """On a GIL-enabled build, importing the connector must not disable the GIL."""
    proc = _run_in_subprocess(
        """
        import sys
        import snowflake.connector  # noqa: F401
        import snowflake.connector._internal.arrow_stream_iterator  # noqa: F401
        from snowflake.connector._core import sf_core_python  # noqa: F401
        sys.exit(0 if sys._is_gil_enabled() is True else 1)
        """
    )
    assert proc.returncode == 0, (
        f"Unexpected GIL state after importing the connector on a GIL-enabled interpreter "
        f"(subprocess exit {proc.returncode}).\n"
        f"--- subprocess stderr ---\n{proc.stderr}\n"
        f"--- subprocess stdout ---\n{proc.stdout}"
    )
