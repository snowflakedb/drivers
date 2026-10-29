"""Compile checks for the vendored Arrow IPC ENODATA fallback."""

from __future__ import annotations

import shutil
import subprocess

from pathlib import Path

import pytest


_ARROW_DIR = (
    Path(__file__).resolve().parents[2]
    / "src"
    / "snowflake"
    / "connector"
    / "_internal"
    / "nanoarrow_cpp"
    / "ArrowIterator"
)
_FALLBACK_C = Path(__file__).resolve().parent / "enodata_fallback.c"
_CFLAGS = [
    "-std=c99",
    "-DFLATBUFFERS_LITTLEENDIAN=1",
    "-DFLATBUFFERS_PROTOCOL_IS_LE=1",
    "-Wno-unused-variable",
    "-Wno-unused-const-variable",
    "-Wno-unreachable-code",
]


def _compiler() -> str:
    cc = shutil.which("gcc") or shutil.which("cc")
    if cc is None:
        pytest.fail("gcc or cc is required to compile nanoarrow_ipc.c")
    return cc


def _run(args: list[str], *, stdin: str | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, input=stdin, capture_output=True, text=True, check=False)


def _enodata_macro(cc: str, source: Path | None = None, stdin: str | None = None) -> int:
    command = [cc, *_CFLAGS, "-dM", "-E"]
    if source is None:
        command.append("-")
    else:
        command.extend(["-I", str(_ARROW_DIR), str(source)])
    result = _run(command, stdin=stdin)
    assert result.returncode == 0, result.stderr
    values = [int(line.split()[-1]) for line in result.stdout.splitlines() if line.startswith("#define ENODATA ")]
    assert len(values) == 1
    return values[0]


def test_nanoarrow_ipc_compiles_with_system_enodata(tmp_path: Path) -> None:
    """Platforms that define ENODATA keep that value when the decoder is compiled."""
    cc = _compiler()
    system_enodata = _enodata_macro(cc, stdin="#include <errno.h>\n")
    ipc = _ARROW_DIR / "nanoarrow_ipc.c"
    obj = tmp_path / "nanoarrow_ipc.o"
    compiled = _run([cc, *_CFLAGS, "-I", str(_ARROW_DIR), "-c", str(ipc), "-o", str(obj)])
    assert compiled.returncode == 0, compiled.stderr
    assert obj.is_file()
    assert obj.stat().st_size > 0
    assert _enodata_macro(cc, source=ipc) == system_enodata


def test_nanoarrow_ipc_fallback_when_enodata_undefined(tmp_path: Path) -> None:
    """Undefining ENODATA compiles the decoder with the local fallback value 120."""
    cc = _compiler()
    assert _enodata_macro(cc, source=_FALLBACK_C) == 120
    binary = tmp_path / "enodata_fallback"
    compiled = _run(
        [
            cc,
            *_CFLAGS,
            "-I",
            str(_ARROW_DIR),
            str(_FALLBACK_C),
            str(_ARROW_DIR / "flatcc.c"),
            str(_ARROW_DIR / "nanoarrow.c"),
            "-o",
            str(binary),
        ]
    )
    assert compiled.returncode == 0, compiled.stderr
    ran = subprocess.run([str(binary)], check=False)
    assert ran.returncode == 0
