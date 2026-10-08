#!/bin/bash
#
# Builds the linux/amd64 WIF artifact for one lane in Docker, then runs the
# same tests/auth/run_wif.sh Jenkins runs, against the same three WIF VMs.
#
# Usage:
#   ./tests/auth/run_wif_local.sh <sf_core|nodejs> [--skip-build] [--reference] [wrapper args...]
#
# Arguments, in order:
#   * lane (required) selects which artifact is built and run.
#   * --skip-build (optional) reuses the lane's artifact under
#     tests/auth/wif/artifacts/ instead of building it.
#   * everything else goes to tests/auth/run_wif.sh unchanged.
#
# Examples:
#   ./tests/auth/run_wif_local.sh sf_core
#   ./tests/auth/run_wif_local.sh sf_core --skip-build
#   ./tests/auth/run_wif_local.sh nodejs                                      # universal, builds
#   ./tests/auth/run_wif_local.sh nodejs --skip-build --reference             # reference, same build
#   ./tests/auth/run_wif_local.sh nodejs -t "should authenticate"
#   ./tests/auth/run_wif_local.sh nodejs --skip-build -t "should authenticate"
#   ./tests/auth/run_wif_local.sh nodejs --provider AWS                         # that VM only
#
# Prerequisites:
#   * "./scripts/decode_secrets.sh wif" has decoded tests/auth/wif/parameters/
#   * Docker can run linux/amd64 containers
#   * Do not use parameters_preprod.json.
#
# Each lane has a builder image: tests/docker/wif-sf-core (rockylinux:8, the
# sf_core runtime base) and tests/docker/wif-nodejs. Named volumes keep cargo,
# npm, rustup, and one Cargo target dir per lane across runs.

set -euo pipefail

log() {
  printf '[wif-local][%s] %s\n' "$(date -u +'%H:%M:%S')" "$*"
}

THIS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$THIS_DIR/../.." && pwd)"
ARTIFACT_DIR="$THIS_DIR/wif/artifacts"
PLATFORM=linux/amd64

LANE="${1:-}"
case "$LANE" in
  sf_core|nodejs)
    shift
    ;;
  *)
    echo "Usage: $0 <sf_core|nodejs> [--skip-build] [--reference] [wrapper args...]" >&2
    exit 1
    ;;
esac

SKIP_BUILD=0
RUN_ARGS=()
for arg in "$@"; do
  if [[ "$arg" == --skip-build ]]; then
    SKIP_BUILD=1
  else
    RUN_ARGS+=("$arg")
  fi
done

for f in \
  "$THIS_DIR/wif/parameters/parameters_wif.json" \
  "$THIS_DIR/wif/parameters/rsa_wif_aws_azure" \
  "$THIS_DIR/wif/parameters/rsa_wif_gcp"
do
  if [[ ! -s "$f" ]]; then
    echo "ERROR: $f not found or empty" >&2
    echo "Run: ./scripts/decode_secrets.sh wif" >&2
    exit 1
  fi
done

mkdir -p "$ARTIFACT_DIR"

run_builder() {
  local image="$1" target_volume="$2" build_script="$3"
  docker run \
    --rm \
    --platform "$PLATFORM" \
    --user root \
    --volume "$REPO_ROOT:/source:ro" \
    --volume "$ARTIFACT_DIR:/artifacts" \
    --volume universal-driver-wif-cargo-registry:/root/.cargo/registry \
    --volume universal-driver-wif-cargo-git:/root/.cargo/git \
    --volume universal-driver-wif-npm-cache:/root/.npm \
    --volume universal-driver-wif-rustup:/root/.rustup \
    --volume "$target_volume:/cargo-target" \
    "$image" \
    bash "$build_script"
}

build_sf_core() {
  local image="${SF_CORE_BUILD_IMAGE:-ud-wif-sf-core-local:latest}"
  if [[ "$SKIP_BUILD" -eq 1 ]]; then
    log "Reusing $ARTIFACT_DIR/sf_core_e2e"
    return
  fi

  log "Building $image"
  docker build --platform="$PLATFORM" --tag "$image" "$REPO_ROOT/tests/docker/wif-sf-core"

  log "Building the linux/amd64 sf_core binary in $image"
  run_builder "$image" universal-driver-wif-sf-core-cargo-target \
    /source/tests/docker/wif-sf-core/build_in_container.sh
}

build_nodejs() {
  local image="${NODEJS_BUILD_IMAGE:-ud-wif-nodejs-local:latest}"
  ARTIFACT="$ARTIFACT_DIR/nodejs_wif.tar.gz"
  source "$THIS_DIR/wif_nodejs_artifact_validation.sh"

  IMAGE_ID=""
  if [[ "$SKIP_BUILD" -eq 1 ]]; then
    # An absent image leaves IMAGE_ID empty, which the sidecar check reports as
    # an image mismatch.
    IMAGE_ID="$(docker image inspect --format '{{.Id}}' "$image" 2>/dev/null || true)"
    skip_build_or_die
    return
  fi

  log "Building $image"
  PLATFORM="$PLATFORM" IMAGE_TAG="$image" \
    "$REPO_ROOT/tests/docker/wif-nodejs/build.sh"
  IMAGE_ID="$(docker image inspect --format '{{.Id}}' "$image")"

  log "Building the linux/amd64 artifact in $image"
  run_builder "$image" universal-driver-wif-cargo-target \
    /source/tests/docker/wif-nodejs/build_in_container.sh

  write_nodejs_sidecar "$IMAGE_ID" "$PLATFORM"
}

case "$LANE" in
  sf_core) build_sf_core ;;
  nodejs) build_nodejs ;;
esac

log "Handing over to run_wif.sh $LANE"
# The +"..." guard keeps the no-argument run working under bash 3.2, which
# treats an empty array expansion as an unset variable.
exec "$THIS_DIR/run_wif.sh" "$LANE" ${RUN_ARGS[@]+"${RUN_ARGS[@]}"}
