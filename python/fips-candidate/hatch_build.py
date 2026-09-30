"""Build the independently named FIPS-TLS candidate without reusing a native bridge."""

from __future__ import annotations

import os
import sysconfig

from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from typing import Any


_ordinary_hook_spec = spec_from_file_location(
    "_snowflake_ordinary_hatch_build", Path(__file__).with_name("hatch_ordinary.py")
)
if _ordinary_hook_spec is None or _ordinary_hook_spec.loader is None:
    raise RuntimeError("The ordinary build hook is missing from the candidate source package")
_ordinary_hook = module_from_spec(_ordinary_hook_spec)
_ordinary_hook_spec.loader.exec_module(_ordinary_hook)
OrdinaryBuildHook = _ordinary_hook.BuildHook
CYTHON_AVAILABLE = _ordinary_hook.CYTHON_AVAILABLE


class BuildHook(OrdinaryBuildHook):
    """Select the Rust feature and fail closed on incomplete candidate wheels."""

    CORE_BUILD_FEATURES = ("fips-tls",)
    CORE_OUTPUT_DIR = Path("candidate-native")
    CYTHON_BUILD_DIR = Path("candidate-native/cython")
    EXTENSION_OUTPUT_DIR = Path("candidate-native/arrow")

    def initialize(self, version: str, build_data: dict[str, Any]) -> None:
        if self.target_name == "editable":
            raise RuntimeError("The FIPS-TLS candidate is only available as a built wheel or sdist")

        if self.target_name == "wheel":
            if os.environ.get("SKIP_CORE_BUILD", "").lower() in self.POSITIVE_VALUES:
                raise RuntimeError("SKIP_CORE_BUILD is not permitted for the FIPS-TLS candidate")
            if os.environ.get(self.DISABLE_COMPILE_ENV_VAR, "").lower() in self.POSITIVE_VALUES:
                raise RuntimeError("The FIPS-TLS candidate requires its native extensions")
            if not CYTHON_AVAILABLE:
                raise RuntimeError(
                    "The FIPS-TLS candidate requires Cython and setuptools"
                ) from _ordinary_hook.CYTHON_IMPORT_ERROR
            if not (Path(self.root) / "Cargo.toml").is_file():
                raise RuntimeError("The FIPS-TLS candidate requires its own Rust workspace manifest")

        super().initialize(version, build_data)
        if self.target_name == "wheel":
            core_name = f"sf_core_python{sysconfig.get_config_var('EXT_SUFFIX')}"
            core_ext = Path(self.root) / self.CORE_OUTPUT_DIR / core_name
            core_destination = f"snowflake/connector/_core/{core_name}"
            if not core_ext.is_file() or build_data.get("force_include", {}).get(str(core_ext)) != core_destination:
                raise RuntimeError(
                    "The FIPS-TLS candidate native bridge for this interpreter was not built and selected for the wheel"
                )
            ext_name = f"arrow_stream_iterator{sysconfig.get_config_var('EXT_SUFFIX')}"
            arrow_ext = Path(self.root) / self.EXTENSION_OUTPUT_DIR / "snowflake/connector/_internal" / ext_name
            if not arrow_ext.is_file():
                raise RuntimeError("The candidate Arrow native extension was not built")
            build_data.setdefault("force_include", {})[str(arrow_ext)] = f"snowflake/connector/_internal/{ext_name}"


def get_build_hook() -> type[BuildHook]:
    """Return the candidate's Hatch build hook."""
    return BuildHook
