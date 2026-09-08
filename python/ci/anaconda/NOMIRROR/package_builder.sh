#!/bin/bash

set -euo pipefail

export UNIVERSAL_DRIVER_DIR="${UNIVERSAL_DRIVER_DIR:-/repo/universal-driver}"
export CONDA_BLD_PATH="${CONDA_BLD_PATH:-/repo/conda-bld}"

mkdir -p "$CONDA_BLD_PATH"
cd "$UNIVERSAL_DRIVER_DIR"

BUILD_SCRIPT="${UNIVERSAL_DRIVER_DIR}/python/ci/anaconda/NOMIRROR/conda_build.sh"

for fmt in 1 2; do
    CONDA_PACKAGE_FORMAT="$fmt" bash "$BUILD_SCRIPT"
done

conda build purge

cd "$CONDA_BLD_PATH"
conda index .
chmod -R o+w,g+w "$CONDA_BLD_PATH"

echo "Packages indexed under $CONDA_BLD_PATH"
