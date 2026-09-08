#!/bin/bash

set -euo pipefail

X86_DIR=""
ARM_DIR=""
NOARCH_DIR=""
OUTPUT_DIR="$(pwd)/temp-stage-conda"
INJECT_SPROC_DEPS="true"

usage() {
    cat <<'EOF'
Usage: prepare_stage_channel.sh [--x86 DIR] [--arm DIR] [--noarch DIR] [--output DIR]

  --x86 DIR      conda-bld/linux-64 directory from an x86_64 build
  --arm DIR      conda-bld/linux-aarch64 directory from an aarch64 build
  --noarch DIR   noarch packages, i.e. snowpark built by build_snowpark_package.sh
  --output DIR   channel output directory (default: ./temp-stage-conda)
  --no-inject-sproc-deps
                 skip adding pyarrow / pandas next to the connector dependency

At least one of --x86 / --arm is required.

--noarch is required for stored-procedure testing. The Anaconda solver needs the
whole graph satisfiable, and a stored procedure pulls in snowpark; released snowpark
also bounds the connector <5.0.0, so a branch build is needed to admit a 5.x
universal driver. Without it the solve fails with errno 391525 "Packages not found"
when the released connector builds are unavailable. The stage channel combines
snowpark noarch and per-architecture packages with the connector before indexing.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --x86) X86_DIR="$2"; shift 2 ;;
        --arm) ARM_DIR="$2"; shift 2 ;;
        --noarch) NOARCH_DIR="$2"; shift 2 ;;
        --output) OUTPUT_DIR="$2"; shift 2 ;;
        --no-inject-sproc-deps) INJECT_SPROC_DEPS="false"; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

if [[ -z "$X86_DIR" && -z "$ARM_DIR" ]]; then
    echo "[FAILURE] at least one of --x86 / --arm must be provided" >&2
    usage
    exit 1
fi

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

collect_subdir() {
    local src="$1"
    local subdir="$2"

    if [[ -z "$src" ]]; then
        return 0
    fi
    if [[ ! -d "$src" ]]; then
        echo "[FAILURE] not a directory: $src" >&2
        exit 1
    fi

    local dest="$OUTPUT_DIR/$subdir"
    mkdir -p "$dest"

    shopt -s nullglob
    local packages=("$src"/snowflake-connector-python-*.conda "$src"/snowflake-connector-python-*.tar.bz2)
    shopt -u nullglob

    if [[ ${#packages[@]} -eq 0 ]]; then
        echo "[FAILURE] no snowflake-connector-python packages found in $src" >&2
        exit 1
    fi

    echo "Collecting ${#packages[@]} package(s) into $subdir/"
    cp "${packages[@]}" "$dest/"
}

collect_noarch() {
    local src="$1"
    if [[ -z "$src" ]]; then
        return 0
    fi
    if [[ ! -d "$src" ]]; then
        echo "[FAILURE] not a directory: $src" >&2
        exit 1
    fi

    local dest="$OUTPUT_DIR/noarch"
    mkdir -p "$dest"

    shopt -s nullglob
    local packages=("$src"/*.conda "$src"/*.tar.bz2)
    shopt -u nullglob

    if [[ ${#packages[@]} -eq 0 ]]; then
        echo "[FAILURE] no conda packages found in $src" >&2
        exit 1
    fi

    echo "Collecting ${#packages[@]} package(s) into noarch/"
    cp "${packages[@]}" "$dest/"
}

collect_subdir "$X86_DIR" "linux-64"
collect_subdir "$ARM_DIR" "linux-aarch64"
collect_noarch "$NOARCH_DIR"

echo "Indexing channel at $OUTPUT_DIR"
conda index "$OUTPUT_DIR"

find "$OUTPUT_DIR" -type d -name '.cache' -prune -exec rm -rf {} +
find "$OUTPUT_DIR" -type f \
    \( -name 'index.html' \
    -o -name 'repodata_from_packages.json' \
    -o -name 'current_repodata.json' \) -delete
rm -f "$OUTPUT_DIR/channeldata.json"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

for subdir in linux-64 linux-aarch64 noarch; do
    dir="$OUTPUT_DIR/$subdir"
    [[ -f "$dir/repodata.json" ]] || continue

    case "$subdir" in
        linux-64) subdir_src="$X86_DIR" ;;
        linux-aarch64) subdir_src="$ARM_DIR" ;;
        noarch) subdir_src="$NOARCH_DIR" ;;
    esac

    if [[ "$INJECT_SPROC_DEPS" == "true" ]]; then
        python3 "$SCRIPT_DIR/inject_sproc_deps.py" "$dir/repodata.json"
    fi

    cp "$dir/repodata.json" "$dir/repodata_merged.json"

    python3 - "$dir/repodata.json" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path) as fh:
    data = json.load(fh)

removed = data.pop("packages.conda", None)
print(
    f"{path}: {'removed packages.conda' if removed is not None else 'no packages.conda section'}"
)

with open(path, "w") as fh:
    json.dump(data, fh, indent=2)
PY

    if [[ -z "$subdir_src" ]]; then
        echo "$dir/repodata.json: no packages collected for $subdir/, skipping .tar.bz2 check"
        continue
    fi

    tarbz2_count="$(
        python3 -c "import json,sys; print(len(json.load(open(sys.argv[1])).get('packages', {})))" \
            "$dir/repodata.json"
    )"
    if [[ "$tarbz2_count" -eq 0 ]]; then
        echo "[FAILURE] $dir/repodata.json lists no .tar.bz2 packages." >&2
        echo "          XP resolves .tar.bz2 entries from repodata.json, so this channel" >&2
        echo "          would resolve nothing. Build both package formats:" >&2
        echo "            conda build --package-format 1 <recipe>" >&2
        echo "            conda build --package-format 2 <recipe>" >&2
        echo "          package_builder.sh already does both." >&2
        exit 1
    fi
    echo "$dir/repodata.json: $tarbz2_count .tar.bz2 package(s)"
done

echo
echo "Stage channel ready: $OUTPUT_DIR"
find "$OUTPUT_DIR" -type f | sort
echo
cat <<EOF
Upload with, e.g.:

  CREATE STAGE mystage;
  PUT file://${OUTPUT_DIR}/linux-64/* @mystage/linux-64 auto_compress=false;
  PUT file://${OUTPUT_DIR}/linux-aarch64/* @mystage/linux-aarch64 auto_compress=false;
  PUT file://${OUTPUT_DIR}/noarch/* @mystage/noarch auto_compress=false;
  ALTER SESSION SET conda_stage_channel_for_testing = 'MYDB.MYSCHEMA.MYSTAGE';

Note: noarch/repodata.json here is empty but required — conda rejects a channel
without one. If this stage also carries snowpark's noarch packages, skip the
noarch/ PUT above and let snowpark's populated repodata.json stand.
EOF
