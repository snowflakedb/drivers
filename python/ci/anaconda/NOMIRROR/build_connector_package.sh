#!/bin/bash

set -euo pipefail

BRANCH=""
PYTHON_VERSION="3.12"
OUTPUT_DIR="$(pwd)/connector-pkgs"

usage() {
    cat <<'EOF'
Usage: build_connector_package.sh --branch REF [options]

  --branch REF   git ref to build the driver from (required); fetched from origin
                 if not present locally
  --python VER   Python version to build for (default: 3.12)
  --output DIR   where to place the built packages (default: ./connector-pkgs)

Produces both package formats into <output>/<subdir>/, ready for
prepare_stage_channel.sh. Only the host architecture is built -- the frozen solve is
computed per architecture, so a channel needs a run of this on each.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --branch) BRANCH="$2"; shift 2 ;;
        --python) PYTHON_VERSION="$2"; shift 2 ;;
        --output) OUTPUT_DIR="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

if [[ -z "$BRANCH" ]]; then
    echo "[FAILURE] --branch is required" >&2
    usage
    exit 1
fi

command -v conda >/dev/null || { echo "[FAILURE] conda not on PATH" >&2; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANACONDA_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

WORK_DIR="$(mktemp -d)"
WORKTREE="$WORK_DIR/driver"
cleanup() {
    git -C "$REPO_ROOT" worktree remove --force "$WORKTREE" 2>/dev/null || true
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

if ! git -C "$REPO_ROOT" rev-parse --verify --quiet "$BRANCH^{commit}" >/dev/null; then
    echo "Fetching $BRANCH from origin"
    git -C "$REPO_ROOT" fetch origin "$BRANCH"
    REF="FETCH_HEAD"
else
    REF="$BRANCH"
fi

echo "Creating throwaway worktree of $BRANCH"
git -C "$REPO_ROOT" worktree add --detach "$WORKTREE" "$REF"

echo "Building driver from $(git -C "$WORKTREE" rev-parse --short HEAD) ($BRANCH)"

mkdir -p "$WORKTREE/python/ci"
cp -r "$ANACONDA_DIR" "$WORKTREE/python/ci/anaconda"

echo "Snowpark-compat surface in this build:"
for module in options.py telemetry.py; do
    if [[ -f "$WORKTREE/python/src/snowflake/connector/$module" ]]; then
        echo "    present: $module"
    else
        echo "    MISSING: $module  <-- snowpark will fail to import against this build"
    fi
done

VERSION="$(
    grep -Eo '__version__[[:space:]]*=[[:space:]]*"[^"]+"' \
        "$WORKTREE/python/src/snowflake/connector/version.py" \
        | head -n1 | sed -E 's/.*"([^"]+)".*/\1/'
)"
echo "    version: $VERSION"

export CONDA_BLD_PATH="$WORK_DIR/conda-bld"
export SNOWFLAKE_CONNECTOR_PYTHON_VERSION="$VERSION"
mkdir -p "$CONDA_BLD_PATH"

cd "$WORKTREE"
for fmt in 1 2; do
    echo "===== conda build (pkg_format $fmt), python $PYTHON_VERSION ====="
    conda build python/ci/anaconda/recipe --python "$PYTHON_VERSION" \
        --package-format "$fmt" -c conda-forge --override-channels --no-anaconda-upload
done

copied=0
for subdir in linux-64 linux-aarch64; do
    shopt -s nullglob
    built=("$CONDA_BLD_PATH/$subdir"/snowflake-connector-python-*.conda "$CONDA_BLD_PATH/$subdir"/snowflake-connector-python-*.tar.bz2)
    shopt -u nullglob
    if [[ ${#built[@]} -gt 0 ]]; then
        mkdir -p "$OUTPUT_DIR/$subdir"
        cp "${built[@]}" "$OUTPUT_DIR/$subdir/"
        echo "Copied ${#built[@]} package(s) into $OUTPUT_DIR/$subdir/"
        copied=$((copied + ${#built[@]}))
    fi
done

if [[ "$copied" -eq 0 ]]; then
    echo "[FAILURE] no connector packages found under $CONDA_BLD_PATH" >&2
    ls -la "$CONDA_BLD_PATH" >&2 || true
    exit 1
fi

echo
echo "Driver $VERSION built from $BRANCH into $OUTPUT_DIR"
echo "Next: prepare_stage_channel.sh --arm $OUTPUT_DIR/<subdir> --noarch <snowpark-pkgs>"
