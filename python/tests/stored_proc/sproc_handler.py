"""Stored-procedure handler that runs a pytest selection inside Snowflake.

``run_stored_proc.py`` stages this file with the connector wheel, an optional
snowpark wheel and a zip of ``tests/``. By the time the handler runs, the
sandbox has already imported the connector and snowpark from its own packages,
so the handler evicts both from ``sys.modules`` and puts the staged wheels first
on ``sys.path`` before pytest imports anything. Wheels are extracted rather than
zip-imported because the connector ships a native extension.

Nothing here imports ``snowflake.*`` at module level.
"""

from __future__ import annotations

import contextlib
import os
import platform
import shutil
import sys
import traceback
import zipfile

from pathlib import Path


WORK_DIR = Path("/tmp/driver_sproc")
TESTS_ZIP = "driver_tests.zip"
# The VARIANT result is capped at 16 MB; keep the tail of the log, where the
# pytest summary is.
MAX_LOG_CHARS = 8_000_000
# `snowflake` is a namespace package whose cached __path__ predates our
# sys.path entries, so it is evicted with the packages being replaced.
_REPLACED_MODULES = ("snowflake", "snowflake.connector", "snowflake.snowpark", "tests")
_DEFAULT_PYTEST_ARGS = ["-p", "no:cacheprovider", "-rfE", "-m", "not flaky and not slow and not requires_no_mfa"]


def run(session, stage: str, wheels: list, pytest_args: list, env: dict) -> dict:
    """Run pytest inside the stored procedure.

    ``wheels`` are file names on ``stage``; the connector wheel is chosen by this
    sandbox's architecture, every other wheel is pure Python. ``env`` is merged
    into ``os.environ`` before the tests import anything.
    """
    shutil.rmtree(WORK_DIR, ignore_errors=True)
    WORK_DIR.mkdir(parents=True)
    log_path = WORK_DIR / "pytest.log"
    junit_path = WORK_DIR / "junit.xml"
    result: dict = {"exit_code": -1, "machine": platform.machine(), "python": platform.python_version()}

    with open(log_path, "w") as log, contextlib.redirect_stdout(log), contextlib.redirect_stderr(log):
        try:
            site_dir, src_dir, chosen = _stage_payload(session, stage, wheels)
            result["wheels"] = chosen
            os.environ.update(env)
            with _replaced_modules([str(site_dir), str(src_dir)]), contextlib.chdir(src_dir):
                import pytest

                import snowflake.connector

                result["connector_version"] = snowflake.connector.__version__
                result["connector_file"] = snowflake.connector.__file__
                if not Path(snowflake.connector.__file__).is_relative_to(site_dir):
                    raise RuntimeError(f"Staged connector was not imported: got {snowflake.connector.__file__}")
                result["exit_code"] = int(
                    pytest.main([*_DEFAULT_PYTEST_ARGS, *pytest_args, f"--junitxml={junit_path}"])
                )
        except Exception:
            traceback.print_exc()

    log_text = log_path.read_text(errors="replace")
    result["log"] = log_text[-MAX_LOG_CHARS:]
    result["junit"] = junit_path.read_text(errors="replace") if junit_path.exists() else ""
    return result


def _stage_payload(session, stage: str, wheels: list) -> tuple[Path, Path, list]:
    download_dir = WORK_DIR / "download"
    site_dir = WORK_DIR / "site"
    src_dir = WORK_DIR / "src"

    chosen = _select_wheels(wheels)
    for name in [*chosen, TESTS_ZIP]:
        session.file.get(f"@{stage}/{name}", str(download_dir))
    for name in chosen:
        with zipfile.ZipFile(download_dir / name) as wheel:
            wheel.extractall(site_dir)
    with zipfile.ZipFile(download_dir / TESTS_ZIP) as tests_zip:
        tests_zip.extractall(src_dir)
    print(f"Staged {chosen} for {platform.machine()}, Python {platform.python_version()}")
    return site_dir, src_dir, chosen


def _select_wheels(wheels: list) -> list:
    """Return the pure-Python wheels plus the one native wheel for this machine."""
    machine = platform.machine()
    python_tag = f"cp{sys.version_info.major}{sys.version_info.minor}"
    pure = [w for w in wheels if w.endswith("-none-any.whl")]
    native = [w for w in wheels if w not in pure and f"-{python_tag}-" in w and w.endswith(f"_{machine}.whl")]
    if len(native) != 1:
        raise RuntimeError(f"Expected one {python_tag} {machine} connector wheel among {wheels}, found {native}")
    return [*native, *pure]


@contextlib.contextmanager
def _replaced_modules(paths: list):
    """Evict the replaced packages and put ``paths`` first on ``sys.path``.

    Restores the sandbox's modules afterwards, so the session that called the
    procedure keeps the connector it was created with.
    """

    def matches(name: str) -> bool:
        return any(name == prefix or name.startswith(prefix + ".") for prefix in _REPLACED_MODULES)

    saved = {name: sys.modules.pop(name) for name in list(sys.modules) if matches(name)}
    saved_path = list(sys.path)
    sys.path[:0] = paths
    try:
        yield
    finally:
        for name in [name for name in sys.modules if matches(name)]:
            del sys.modules[name]
        sys.modules.update(saved)
        sys.path[:] = saved_path
