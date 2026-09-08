# Conda recipe

Builds `snowflake-connector-python` from this repository. The package is not
`noarch`: it ships two ABI-tagged native extensions (the PyO3 core bridge and
the Cython/C++ Arrow reader), so it is built per Python version and per
architecture.

`source.path` is the repository root. The Python wrapper reaches the Rust
workspace through relative symlinks (`python/Cargo.toml`, `python/sf_core`,
`python/python_bridge`) that only resolve when the whole workspace is present.
Build from a clean checkout, or remove `target/` first — conda-build copies the
source tree.

## Build

From the repository root, with conda-build installed (`conda install -c
conda-forge conda-build`):

```bash
conda build python/ci/anaconda/recipe --python 3.12
```

The C/C++ and Rust toolchains come from `requirements.build`. Version is read
from `python/src/snowflake/connector/version.py`; override with
`SNOWFLAKE_CONNECTOR_PYTHON_VERSION`. Build number defaults to `0`; override
with `UNIVERSAL_DRIVER_BUILD_NUMBER`.

`conda_build_config.yaml` pins the sysroot (glibc floor) to 2.17. That is the
oldest conda-forge publishes for both `linux-64` and `linux-aarch64`. Without
it, current compilers select sysroot 2.34.

`libprotobuf` is capped to match the runtime available in Snowflake's
in-platform channel.

Optional `script_env` passthroughs for iteration: `SKIP_CORE_BUILD`,
`SNOWFLAKE_DISABLE_COMPILE_ARROW_EXTENSIONS`, `SKIP_PROTO_GENERATION`,
`CORE_CARGO_TARGET_DIR`, `PROTO_CARGO_TARGET_DIR`, `CARGO_NET_OFFLINE`. A
`_core/` binary is tagged with `sysconfig` `EXT_SUFFIX`, so a 3.13 build is not
reusable for 3.12. `test.imports` fails the package if an extension is missing.
