"""End-to-end checks for the separately installable FIPS-TLS candidate."""

from __future__ import annotations

import os
import platform
import subprocess
import sys
import sysconfig
import tarfile
import venv
import zipfile

from email.parser import BytesParser
from pathlib import Path, PurePosixPath
from tempfile import TemporaryDirectory

import pytest


CANDIDATE_DIR = Path(__file__).resolve().parents[2] / "fips-candidate"


def run_build(*args: str, cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    build_env = {
        **os.environ,
        "SKIP_CORE_BUILD": "",
        "SKIP_PROTO_GENERATION": "",
        "SNOWFLAKE_DISABLE_COMPILE_ARROW_EXTENSIONS": "",
    }
    if env:
        build_env.update(env)
    return subprocess.run(
        [sys.executable, "-m", "build", *args, "--installer", "uv"],
        cwd=cwd,
        env=build_env,
        capture_output=True,
        text=True,
        timeout=1800,
    )


def run_in_venv(python: Path, cwd: Path, *args: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env.pop("PYTHONPATH", None)
    env.pop("PYTHONHOME", None)
    return subprocess.run(
        [str(python), *args],
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
        timeout=300,
    )


def test_candidate_rejects_skipping_native_build() -> None:
    """SKIP_CORE_BUILD must not turn a default prebuilt bridge into a candidate wheel."""
    with TemporaryDirectory() as temp_dir:
        result = run_build("--wheel", "--outdir", temp_dir, cwd=CANDIDATE_DIR, env={"SKIP_CORE_BUILD": "true"})
    assert result.returncode != 0
    assert "SKIP_CORE_BUILD" in result.stderr + result.stdout


@pytest.mark.slow
def test_candidate_sdist_builds_a_distinct_native_wheel() -> None:
    """An extracted source archive can build and run without its checkout."""
    if sys.platform != "linux" or platform.machine() != "x86_64":
        pytest.skip("FIPS build-toolchain integration test is provisioned on Linux x86_64")

    with TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        source_dist = temp_path / "source-dist"
        wheel_dist = temp_path / "wheel-dist"
        source_dist.mkdir()
        wheel_dist.mkdir()

        source_build = run_build("--sdist", "--outdir", str(source_dist), cwd=CANDIDATE_DIR)
        assert source_build.returncode == 0, source_build.stderr + source_build.stdout
        source_archive = next(source_dist.glob("snowflake_connector_python_fips_candidate-*.tar.gz"))
        archive_root = source_archive.name.removesuffix(".tar.gz")

        with tarfile.open(source_archive, "r:gz") as archive:
            members = archive.getmembers()
            names = set()
            for member in members:
                path = PurePosixPath(member.name)
                assert (
                    not path.is_absolute()
                    and path.parts
                    and path.parts[0] == archive_root
                    and ".." not in path.parts
                    and "\\" not in member.name
                    and (member.isfile() or member.isdir())
                ), f"Unsafe candidate sdist member: {member.name}"
                names.add(path.relative_to(archive_root).as_posix())
            assert {
                "Cargo.toml",
                "Cargo.lock",
                "LICENSE",
                "build_common.rs",
                "hatch_build.py",
                "hatch_ordinary.py",
                "fips_profile.py",
                "python_bridge/Cargo.toml",
                "sf_core/Cargo.toml",
                "sf_core/tests/common/arrow_deserialize_macro/Cargo.toml",
                "protobuf/database_driver_v1.proto",
                "src/snowflake/connector/_core/sf_core_python.pyi",
                "src/snowflake/connector/_core/__init__.py",
                "src/snowflake/connector/_internal/_distribution_profile.py",
            } <= names
            # All entries are confined regular files/directories. Older 3.11
            # patch releases lack tarfile's extraction filter.
            if hasattr(tarfile, "data_filter"):
                archive.extractall(temp_path, members=members, filter="data")
            else:
                archive.extractall(temp_path, members=members)

        unpacked = temp_path / archive_root
        wheel_build = run_build("--wheel", "--outdir", str(wheel_dist), cwd=unpacked)
        assert wheel_build.returncode == 0, wheel_build.stderr + wheel_build.stdout
        wheel = next(wheel_dist.glob("snowflake_connector_python_fips_candidate-*.whl"))
        assert f"-cp{sys.version_info.major}{sys.version_info.minor}-" in wheel.name

        with zipfile.ZipFile(wheel) as archive:
            names = archive.namelist()
            assert "snowflake/__init__.py" not in names
            extension_name = f"snowflake/connector/_core/sf_core_python{sysconfig.get_config_var('EXT_SUFFIX')}"
            assert [
                name
                for name in names
                if name.startswith("snowflake/connector/_core/sf_core_python") and name.endswith((".so", ".pyd"))
            ] == [extension_name]
            arrow_name = f"snowflake/connector/_internal/arrow_stream_iterator{sysconfig.get_config_var('EXT_SUFFIX')}"
            assert [
                name
                for name in names
                if name.startswith("snowflake/connector/_internal/arrow_stream_iterator")
                and name.endswith((".so", ".pyd"))
            ] == [arrow_name]
            assert "snowflake/connector/_core/sf_core_python.pyi" in names
            assert "snowflake/connector/_internal/_distribution_profile.py" in names
            metadata_name = next(name for name in names if name.endswith(".dist-info/METADATA"))
            metadata = BytesParser().parsebytes(archive.read(metadata_name))
            assert metadata["Name"] == "snowflake-connector-python-fips-candidate"
            assert not any("cryptography" in dep.lower() for dep in metadata.get_all("Requires-Dist", []))

        venv_dir = temp_path / "venv"
        venv.create(venv_dir, with_pip=True)
        python = venv_dir / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
        install = run_in_venv(python, temp_path, "-m", "pip", "install", str(wheel))
        assert install.returncode == 0, install.stderr + install.stdout

        no_crypto_check = run_in_venv(
            python,
            temp_path,
            "-c",
            """
import builtins
from importlib.util import find_spec

assert find_spec('cryptography') is None, 'unexpected cryptography dependency'
original_import = builtins.__import__
def forbid_cryptography_import(name, *args, **kwargs):
    if name == 'cryptography' or name.startswith('cryptography.'):
        raise AssertionError('candidate imported cryptography')
    return original_import(name, *args, **kwargs)
builtins.__import__ = forbid_cryptography_import

import snowflake.connector
from snowflake.connector._core import sf_core_python
from snowflake.connector._internal._private_key_helper import normalize_private_key
assert sf_core_python.fips_tls_enabled() is True
raw = b'private-key-data'
assert normalize_private_key(raw) is raw
encoded = 'private-key-data'
assert normalize_private_key(encoded) is encoded
""",
        )
        assert no_crypto_check.returncode == 0, no_crypto_check.stderr + no_crypto_check.stdout

        install_crypto = run_in_venv(python, temp_path, "-m", "pip", "install", "cryptography>=46.0.5")
        assert install_crypto.returncode == 0, install_crypto.stderr + install_crypto.stdout
        key_check = run_in_venv(
            python,
            temp_path,
            "-c",
            """
import builtins
from cryptography.hazmat.primitives.asymmetric import rsa
from snowflake.connector._internal._private_key_helper import normalize_private_key
from snowflake.connector.errors import ProgrammingError

key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
original_import = builtins.__import__
def forbid_cryptography_import(name, *args, **kwargs):
    if name == 'cryptography' or name.startswith('cryptography.'):
        raise AssertionError('candidate imported cryptography during normalization')
    return original_import(name, *args, **kwargs)
builtins.__import__ = forbid_cryptography_import
try:
    normalize_private_key(key)
except ProgrammingError as error:
    assert 'not supported' in str(error)
else:
    raise AssertionError('candidate accepted a PyCA key object')
""",
        )
        assert key_check.returncode == 0, key_check.stderr + key_check.stdout

        # Build the actual ordinary wheel without compiling another native bridge:
        # the overlap test must observe the distribution's real installed files.
        ordinary_dist = temp_path / "ordinary-dist"
        ordinary_dist.mkdir()
        ordinary_build = run_build(
            "--wheel",
            "--outdir",
            str(ordinary_dist),
            cwd=CANDIDATE_DIR.parent,
            env={
                "SKIP_CORE_BUILD": "true",
                "SKIP_PROTO_GENERATION": "true",
                "SNOWFLAKE_DISABLE_COMPILE_ARROW_EXTENSIONS": "true",
            },
        )
        assert ordinary_build.returncode == 0, ordinary_build.stderr + ordinary_build.stdout
        ordinary_wheel = next(ordinary_dist.glob("snowflake_connector_python-*.whl"))

        profile_location = run_in_venv(
            python,
            temp_path,
            "-c",
            "from snowflake.connector._internal import _distribution_profile as profile; print(profile.__file__)",
        )
        assert profile_location.returncode == 0, profile_location.stderr + profile_location.stdout
        installed_profile = Path(profile_location.stdout.strip())
        candidate_profile = installed_profile.read_bytes()
        with zipfile.ZipFile(ordinary_wheel) as archive:
            ordinary_profile = archive.read("snowflake/connector/_internal/_distribution_profile.py")
        try:
            installed_profile.write_bytes(ordinary_profile)
            mismatch = run_in_venv(python, temp_path, "-c", "import snowflake.connector")
            assert mismatch.returncode != 0
            assert "native bridge" in mismatch.stderr
            assert "FIPS-TLS build profile" in mismatch.stderr
        finally:
            installed_profile.write_bytes(candidate_profile)

        # Installing either distribution last can overwrite shared package files;
        # both metadata names must reject import regardless of overwrite order.
        ordinary_install = run_in_venv(python, temp_path, "-m", "pip", "install", "--no-deps", str(ordinary_wheel))
        assert ordinary_install.returncode == 0, ordinary_install.stderr + ordinary_install.stdout
        ordinary_collision = run_in_venv(python, temp_path, "-c", "import snowflake.connector")
        assert ordinary_collision.returncode != 0
        assert "snowflake-connector-python-fips-candidate" in ordinary_collision.stderr
        assert "snowflake-connector-python" in ordinary_collision.stderr
        assert "uninstall" in ordinary_collision.stderr.lower()

        candidate_install = run_in_venv(
            python, temp_path, "-m", "pip", "install", "--force-reinstall", "--no-deps", str(wheel)
        )
        assert candidate_install.returncode == 0, candidate_install.stderr + candidate_install.stdout
        candidate_collision = run_in_venv(python, temp_path, "-c", "import snowflake.connector")
        assert candidate_collision.returncode != 0
        assert "snowflake-connector-python-fips-candidate" in candidate_collision.stderr
        assert "snowflake-connector-python" in candidate_collision.stderr
        assert "uninstall" in candidate_collision.stderr.lower()
