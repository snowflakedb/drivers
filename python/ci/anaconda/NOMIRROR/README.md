# Conda testing scripts

Internal wrappers around `../recipe` for stored-procedure / stage-channel
testing. The directory name is what keeps this off the public mirror
(`NOMIRROR_PATHS` in `ci/mirroring/copy.bara.sky`); renaming it publishes
every script below.

```
NOMIRROR/
├── conda_build.sh                 # one arch, every PYTHON_VERSIONS entry
├── package_builder.sh             # both package formats + conda index
├── prepare_stage_channel.sh       # assemble a stage channel
├── build_connector_package.sh     # build the driver from a git ref
├── build_snowpark_package.sh      # noarch snowpark from a git ref
├── inject_sproc_deps.py           # add pyarrow/pandas next to the connector
└── validate_deps_sync.py          # recipe run deps == pyproject dependencies
```

Requires `conda-build` and `conda-index`. Do not install `conda-verify` (last
release needs `python <3.12`).

## Dual format and stage channel

XP reads `.tar.bz2` from `repodata.json` and `.conda` from
`repodata_merged.json`. Build both formats:

```bash
UNIVERSAL_DRIVER_DIR=/path/to/universal-driver \
CONDA_BLD_PATH=/path/to/conda-bld \
  bash python/ci/anaconda/NOMIRROR/package_builder.sh
```

`CONDA_BLD_PATH` must sit outside the source tree. conda-build copies
`source.path` (the repo root) into `$CONDA_BLD_PATH/work`.

Assemble a channel:

```bash
python/ci/anaconda/NOMIRROR/prepare_stage_channel.sh \
    --x86 /path/to/x86-conda-bld/linux-64 \
    --arm /path/to/arm-conda-bld/linux-aarch64 \
    --output ./temp-stage-conda
```

A channel needs both `linux-64/` and `linux-aarch64/` for x86 and ARM SUTs.
`noarch/repodata.json` is kept empty so conda accepts the channel; if snowpark
owns `noarch/` on the same stage, do not PUT this empty copy over it.

Released snowpark bounds the connector `<5.0.0`. A 5.x driver in a stored
procedure needs a snowpark build whose bound admits 5.x, staged into the same
channel (`build_snowpark_package.sh`). Check the solve with
`SELECT SYSTEM$GET_FROZEN_SOLVE_INFO('my_procedure()');`.

## CI

`.github/workflows/build-conda-packages.yml` calls
`_build-conda-packages.yml`. Both files are listed in `EXCLUDED_PATHS` —
GitHub only loads workflows from `.github/workflows/`, so they cannot live
on a Copybara-excluded directory. PRs that touch the recipe, `pyproject.toml`,
`hatch_build.py`, or `version.py` build Python 3.12 on both native
architectures. Widen the matrix with `workflow_dispatch`.

Install conda-build into base (`conda install -n base`): `conda build` and
`conda index` are subcommands of the base `conda`, not of the env
setup-miniconda activates.

## Recipe drift

```bash
cd python
uv run --no-project --python 3.13 --with pyyaml python ci/anaconda/NOMIRROR/validate_deps_sync.py
```

Exit 0 and no output means in sync.
