"""Mypy check that DictCursor fetch results are dictionaries."""

from __future__ import annotations

import os
import subprocess
import sys

from pathlib import Path


def test_dict_cursor_fetch_rows_are_dicts() -> None:
    python_root = Path(__file__).resolve().parents[2]
    snippet = python_root / "tests" / "typing" / "dict_cursor_rows.py"
    env = os.environ.copy()
    env["MYPYPATH"] = str(python_root / "src")
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy",
            "--follow-imports=silent",
            "--explicit-package-bases",
            "--config-file",
            str(python_root / "pyproject.toml"),
            str(snippet),
        ],
        cwd=python_root,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
