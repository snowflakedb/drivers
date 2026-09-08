#!/bin/bash

set -euo pipefail

BRANCH=""
REPO=""
PYTHON_VERSION="3.12"
OUTPUT_DIR="$(pwd)/snowpark-pkgs"
BUMP_MINOR="true"

usage() {
    cat <<'EOF'
Usage: build_snowpark_package.sh --branch REF [options]

  --branch REF     git ref to build snowpark from (required)
  --repo PATH      snowpark-python checkout or clone URL (default: /home/repo/snowpark-python)
  --python VER     Python version for the noarch build (default: 3.12)
  --output DIR     where to place the built packages (default: ./snowpark-pkgs)
  --no-bump        do not bump the minor version

By default the package version is bumped by minor+1, matching snowpark's own
prepare_no_arch_packages.sh. That is what makes the staged build outrank the
released one when the solver has both to choose from.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --branch) BRANCH="$2"; shift 2 ;;
        --repo) REPO="$2"; shift 2 ;;
        --python) PYTHON_VERSION="$2"; shift 2 ;;
        --output) OUTPUT_DIR="$2"; shift 2 ;;
        --no-bump) BUMP_MINOR="false"; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

if [[ -z "$BRANCH" ]]; then
    echo "[FAILURE] --branch is required" >&2
    usage
    exit 1
fi
REPO="${REPO:-/home/repo/snowpark-python}"

command -v conda >/dev/null || { echo "[FAILURE] conda not on PATH" >&2; exit 1; }

WORK_DIR="$(mktemp -d)"
cleanup() {
    if [[ -n "${WORKTREE:-}" && -d "$REPO/.git" ]]; then
        git -C "$REPO" worktree remove --force "$WORKTREE" 2>/dev/null || true
    fi
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

if [[ -d "$REPO/.git" ]]; then
    echo "Creating throwaway worktree of $BRANCH from $REPO"
    WORKTREE="$WORK_DIR/snowpark"
    git -C "$REPO" worktree add --detach "$WORKTREE" "$BRANCH"
    SRC="$WORKTREE"
else
    echo "Cloning $REPO at $BRANCH"
    git clone --depth 1 --branch "$BRANCH" "$REPO" "$WORK_DIR/snowpark"
    SRC="$WORK_DIR/snowpark"
fi

cd "$SRC"
echo "Building snowpark from $(git rev-parse --short HEAD) ($BRANCH)"
echo "Connector bound in this build:"
grep -n "snowflake-connector-python" recipe/meta.yaml | sed 's/^/    /'

if [[ "$BUMP_MINOR" == "true" ]]; then
    python3 - <<'PY'
import re
from pathlib import Path

version_path = Path("src/snowflake/snowpark/version.py")
meta_path = Path("recipe/meta.yaml")

text = version_path.read_text(encoding="utf-8")
match = re.search(r"VERSION\s*=\s*\((\d+),\s*(\d+),\s*(\d+)\)", text)
if match is None:
    raise SystemExit(f"Could not parse VERSION tuple in {version_path}")

major, minor, _ = map(int, match.groups())
new_version = f"{major}.{minor + 1}.0"
version_path.write_text(
    re.sub(r"VERSION\s*=\s*\(\d+,\s*\d+,\s*\d+\)", f"VERSION = ({major}, {minor + 1}, 0)", text, count=1),
    encoding="utf-8",
)

meta_text = meta_path.read_text(encoding="utf-8")
meta_path.write_text(
    re.sub(
        r'(\{\%\s*set\s+version\s*=\s*")[0-9]+\.[0-9]+\.[0-9]+("\s*\%\})',
        rf"\g<1>{new_version}\g<2>",
        meta_text,
        count=1,
    ),
    encoding="utf-8",
)
print(f"Bumped snowpark version to {new_version} so it outranks the released build")
PY
fi

export SNOWFLAKE_SNOWPARK_PYTHON_NOARCH_BUILD=true
export CONDA_BLD_PATH="$WORK_DIR/conda-bld"
mkdir -p "$CONDA_BLD_PATH"

for fmt in 1 2; do
    echo "===== conda build (pkg_format $fmt), python $PYTHON_VERSION ====="
    conda build ./recipe/ --python "$PYTHON_VERSION" --package-format "$fmt" \
        -c conda-forge --override-channels --no-anaconda-upload
done

mkdir -p "$OUTPUT_DIR"
shopt -s nullglob
built=("$CONDA_BLD_PATH"/noarch/snowflake-snowpark-python-*.conda "$CONDA_BLD_PATH"/noarch/snowflake-snowpark-python-*.tar.bz2)
shopt -u nullglob
if [[ ${#built[@]} -eq 0 ]]; then
    echo "[FAILURE] no snowpark packages found under $CONDA_BLD_PATH/noarch" >&2
    ls -la "$CONDA_BLD_PATH" >&2 || true
    exit 1
fi

cp "${built[@]}" "$OUTPUT_DIR/"
echo
echo "Built ${#built[@]} snowpark package(s) into $OUTPUT_DIR:"
for f in "${built[@]}"; do echo "    $(basename "$f")"; done
echo
echo "Next: pass this directory to prepare_stage_channel.sh --noarch"
