from __future__ import annotations

import os
import shutil
import subprocess
import sys
import sysconfig
from contextlib import nullcontext
from pathlib import Path
from tempfile import TemporaryDirectory

_PYTHON_DIR = Path(__file__).resolve().parent
_CARGO_MANIFEST = (_PYTHON_DIR / "Cargo.toml").resolve()
_SUFFIXES = {".dylib", ".so", ".dll", ".pyd"}


def _libpython_dir() -> Path:
    names = [
        n
        for n in (
            sysconfig.get_config_var("INSTSONAME"),
            sysconfig.get_config_var("LDLIBRARY"),
        )
        if n
    ]
    dirs = [
        d
        for d in (
            sysconfig.get_config_var("LIBDIR"),
            str(Path(sys.base_prefix) / "lib"),
        )
        if d
    ]
    for directory in dirs:
        if any(Path(directory, name).exists() for name in names):
            return Path(directory)
    raise SystemExit(f"could not locate libpython for {sys.executable}")


def _target_dir_ctx():
    stable = os.environ.get("CORE_CARGO_TARGET_DIR")
    if stable:
        Path(stable).mkdir(parents=True, exist_ok=True)
        return nullcontext(stable)
    return TemporaryDirectory()


def _run_cargo(args: list[str], *, env: dict[str, str]) -> None:
    print("+", " ".join(args), flush=True)
    subprocess.run(args, check=True, env=env)


def _cargo_test(target_dir: str) -> None:
    # --no-default-features drops pyo3/extension-module so the test binary can
    # link libpython (dev-dep auto-initialize). Do not set
    # PYO3_BUILD_EXTENSION_MODULE here. Linking libpython leaves only its
    # SONAME and no rpath, so the loader needs its directory. LIBDIR can be
    # stale for relocated interpreters, hence the base_prefix/lib fallback.
    libdir = _libpython_dir()
    print(f"PYO3_PYTHON={sys.executable}", flush=True)
    print(f"PY_LIBDIR={libdir}", flush=True)
    env = {
        **os.environ,
        "PYO3_PYTHON": sys.executable,
    }
    env.pop("PYO3_BUILD_EXTENSION_MODULE", None)
    for key in ("LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"):
        prev = env.get(key, "")
        env[key] = str(libdir) if not prev else f"{libdir}{os.pathsep}{prev}"
    _run_cargo(
        [
            "cargo",
            "test",
            "--locked",
            "--package",
            "python_bridge",
            "--manifest-path",
            str(_CARGO_MANIFEST),
            "--target-dir",
            target_dir,
            "--no-default-features",
            "--features",
            "native-arrow",
        ],
        env=env,
    )


def _install_built_core(release_dir: Path, dest_dir: Path) -> Path:
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest_name = f"sf_core_python{sysconfig.get_config_var('EXT_SUFFIX')}"
    for file in release_dir.iterdir():
        if not file.is_file() or "sf_core_python" not in file.name:
            continue
        if file.suffix not in _SUFFIXES:
            continue
        for old in dest_dir.glob("sf_core_python.*"):
            if old.suffix in _SUFFIXES and old.name != dest_name:
                old.unlink()
        dest = dest_dir / dest_name
        tmp = dest_dir / (dest_name + ".tmp")
        shutil.copy2(file, tmp)
        os.replace(tmp, dest)
        return dest
    raise SystemExit(f"native-arrow sf_core_python .so not found in {release_dir}")


def _cargo_overlay(target_dir: str) -> None:
    env = {
        **os.environ,
        "PYO3_PYTHON": sys.executable,
        "PYO3_BUILD_EXTENSION_MODULE": "1",
    }
    _run_cargo(
        [
            "cargo",
            "build",
            "--locked",
            "--release",
            "--package",
            "python_bridge",
            "--manifest-path",
            str(_CARGO_MANIFEST),
            "--target-dir",
            target_dir,
            "--features",
            "vendored-openssl,native-arrow",
        ],
        env=env,
    )
    release_dir = Path(target_dir) / "release"
    dest_dir = Path(sysconfig.get_path("purelib")) / "snowflake" / "connector" / "_core"
    dest = _install_built_core(release_dir, dest_dir)
    print(f"Installed {dest}", flush=True)
    from snowflake.connector._core import sf_core_python

    if not sf_core_python.native_arrow_enabled():
        raise SystemExit("native_arrow_enabled() is False after rebuild")


def main() -> None:
    if not _CARGO_MANIFEST.exists():
        raise SystemExit(f"Cargo.toml not found: {_CARGO_MANIFEST}")
    with _target_dir_ctx() as target_dir:
        _cargo_test(target_dir)
        _cargo_overlay(target_dir)


if __name__ == "__main__":
    main()
