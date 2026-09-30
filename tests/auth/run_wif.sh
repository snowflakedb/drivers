#!/bin/bash -e
#
# WIF e2e test orchestrator. Jenkins invokes this script directly; local runs
# use tests/auth/run_wif_local.sh.
#
# Strategy (mirrors snowflake-odbc): the bare WIF cloud VMs have Docker + scp
# but no Rust/cmake/napi toolchain and no access to our private Artifactory. So
# the test artifacts are prebuilt off the VM — on the Jenkins node in CI, or on
# the developer's machine under tests/auth/run_wif_local.sh — and this script ships them
# to each VM and runs them there inside a public runtime container. The
# container inherits the VM's cloud identity via IMDS, which is what the WIF
# flow attests against.
#
# Usage:
#   ./tests/auth/run_wif.sh <sf_core|nodejs> [--reference]
#
# The first argument selects which artifact is shipped and run. There is no
# default: a missing or unknown lane is an error. --reference runs that
# lane's reference suite against the same artifact instead of its own: for
# `nodejs` that is Vitest project e2e-old-driver, which exercises the old
# snowflake-sdk. `sf_core` has no reference suite and rejects the flag.
#
# Prerequisites:
#   * ./scripts/decode_secrets.sh wif has decoded tests/auth/wif/parameters/
#   * tests/auth/wif/artifacts/ holds the artifact for the selected lane
#   * Do not use parameters_preprod.json.

set -o pipefail

export THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
export RSA_KEY_PATH_AWS_AZURE="$THIS_DIR/wif/parameters/rsa_wif_aws_azure"
export RSA_KEY_PATH_GCP="$THIS_DIR/wif/parameters/rsa_wif_gcp"
export PARAMETERS_FILE_PATH="$THIS_DIR/wif/parameters/parameters_wif.json"
export ARTIFACT_DIR="$THIS_DIR/wif/artifacts"

# Public runtime image. Shares the rockylinux:8 base used to build the binary,
# so the prebuilt binary's glibc/libstdc++ deps line up.
RUNTIME_IMAGE="${WIF_RUNTIME_IMAGE:-rockylinux:8}"
NODEJS_RUNTIME_IMAGE="${WIF_NODEJS_RUNTIME_IMAGE:-node:22-slim}"

if [[ -n "${WIF_LANE:-}" ]]; then
  echo "ERROR: WIF_LANE is unused; pass the lane as the first argument" >&2
  echo "Usage: $0 <sf_core|nodejs> [--reference]" >&2
  exit 1
fi

WIF_LANE="${1:-}"
case "$WIF_LANE" in
  sf_core|nodejs)
    shift
    ;;
  *)
    echo "Usage: $0 <sf_core|nodejs> [--reference]" >&2
    exit 1
    ;;
esac

REFERENCE=0
for arg in "$@"; do
  case "$arg" in
    --reference)
      REFERENCE=1
      ;;
    *)
      echo "ERROR: unknown argument '$arg'" >&2
      exit 1
      ;;
  esac
done

case "$WIF_LANE" in
  sf_core)
    if [[ "$REFERENCE" -eq 1 ]]; then
      echo "ERROR: the sf_core lane has no reference suite to run with --reference" >&2
      exit 1
    fi
    LANE_ARTIFACT="$ARTIFACT_DIR/sf_core_e2e"
    LANE_BUILD_SCRIPT="tests/auth/build_wif_sf_core_artifacts.sh"
    LANE_IMAGE="$RUNTIME_IMAGE"
    LANE_ENTRYPOINT="wif_run_sf_core_in_container.sh"
    SUITE=""
    ;;
  nodejs)
    LANE_ARTIFACT="$ARTIFACT_DIR/nodejs_wif.tar.gz"
    LANE_BUILD_SCRIPT="tests/auth/build_wif_nodejs_artifacts.sh"
    LANE_IMAGE="$NODEJS_RUNTIME_IMAGE"
    LANE_ENTRYPOINT="wif_run_nodejs_in_container.sh"
    if [[ "$REFERENCE" -eq 1 ]]; then
      SUITE="e2e-old-driver"
    else
      SUITE="e2e"
    fi
    ;;
  *)
    echo "ERROR: lane must be sf_core or nodejs (got '$WIF_LANE')" >&2
    exit 1
    ;;
esac

TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

log() {
  printf '[wif][%s] %s\n' "$(date -u +'%H:%M:%S')" "$*"
}

# ServerAlive* bounds a stalled transfer: without it a dropped connection
# hangs with no output until the Jenkins stage timeout.
set_wif_remote_opts() {
  local rsa_key_path="$1"
  local ssh_common=(
    -i "$rsa_key_path"
    -o IdentitiesOnly=yes
    -o StrictHostKeyChecking=no
    -o ConnectTimeout=30
    -o ServerAliveInterval=30
    -o ServerAliveCountMax=10
  )
  ssh_opts=("${ssh_common[@]}" -p 443)
  scp_opts=("${ssh_common[@]}" -P 443)
}

ACTIVE_REMOTE_HOST=""
ACTIVE_REMOTE_DIR=""
ACTIVE_REMOTE_RSA_KEY_PATH=""

cleanup_active_remote() {
  if [[ -z "$ACTIVE_REMOTE_DIR" ]]; then
    return 0
  fi

  local host="$ACTIVE_REMOTE_HOST"
  local remote_dir="$ACTIVE_REMOTE_DIR"
  local rsa_key_path="$ACTIVE_REMOTE_RSA_KEY_PATH"
  ACTIVE_REMOTE_HOST=""
  ACTIVE_REMOTE_DIR=""
  ACTIVE_REMOTE_RSA_KEY_PATH=""

  local ssh_opts scp_opts
  set_wif_remote_opts "$rsa_key_path"
  ssh "${ssh_opts[@]}" "$host" "rm -rf \"$remote_dir\"" || true
}

# Jenkins aborts terminate the script before provider cleanup can run.
trap cleanup_active_remote EXIT

# Generate the parameters.json the tests expect (PARAMETER_PATH).
# Uses jq -n so values are JSON-encoded safely.
write_parameters_json() {
  local out="$1" provider="$2" snowflake_host="$3" snowflake_user="$4" impersonation_path="$5"
  jq -n \
    --arg account "$SNOWFLAKE_TEST_WIF_ACCOUNT" \
    --arg user "$snowflake_user" \
    --arg host "$snowflake_host" \
    --arg provider "$provider" \
    --arg impersonation_path "$impersonation_path" \
    '{
      testconnection: {
        SNOWFLAKE_TEST_ACCOUNT: $account,
        SNOWFLAKE_TEST_USER: $user,
        SNOWFLAKE_TEST_HOST: $host,
        SNOWFLAKE_TEST_WIF_PROVIDER: $provider,
        SNOWFLAKE_TEST_WIF_ACCOUNT: $account,
        SNOWFLAKE_TEST_WIF_USER: $user,
        SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH: $impersonation_path
      }
    }' > "$out"
}

run_wif_tests() {
  local provider="$1" host="$2" snowflake_host="$3" rsa_key_path="$4"
  local snowflake_user="$5" impersonation_path="$6"

  local remote_dir="wif_${provider}_${WIF_LANE}_${TIMESTAMP}"
  local ssh_opts scp_opts
  set_wif_remote_opts "$rsa_key_path"

  local params_file
  params_file="$(mktemp)"
  write_parameters_json "$params_file" "$provider" "$snowflake_host" "$snowflake_user" "$impersonation_path"

  echo "==================================================================="
  echo "WIF tests: ${provider} lane=${WIF_LANE}${SUITE:+ suite=$SUITE} (host=${host}, remote_dir=${remote_dir})"
  echo "==================================================================="

  log "${provider}: creating ${remote_dir}"
  ssh "${ssh_opts[@]}" "$host" "mkdir -p \"$remote_dir\"" || {
    echo "ERROR: failed to create remote dir '$remote_dir' on $host" >&2
    rm -f "$params_file"
    return 1
  }
  ACTIVE_REMOTE_HOST="$host"
  ACTIVE_REMOTE_DIR="$remote_dir"
  ACTIVE_REMOTE_RSA_KEY_PATH="$rsa_key_path"

  local transfers=(
    "$params_file|parameters.json"
    "$LANE_ARTIFACT|$(basename "$LANE_ARTIFACT")"
    "$THIS_DIR/$LANE_ENTRYPOINT|$LANE_ENTRYPOINT"
  )
  local src dst spec scp_started
  for spec in "${transfers[@]}"; do
    src="${spec%%|*}"
    dst="${spec##*|}"
    # The artifact is hundreds of megabytes over a 443 tunnel and scp prints no
    # progress when stdout is not a terminal, so the size and duration here are
    # the only signal that a long pause is a transfer rather than a hang.
    log "${provider}: sending ${dst} ($(du -h "$src" | cut -f1))"
    scp_started=$SECONDS
    scp "${scp_opts[@]}" "$src" "$host:$remote_dir/$dst" || {
      echo "ERROR: failed to scp '$src' to $host:$remote_dir/$dst" >&2
      rm -f "$params_file"
      return 1
    }
    log "${provider}: sent ${dst} in $((SECONDS - scp_started))s"
  done
  rm -f "$params_file"

  # A cold VM pulls the runtime image here, which is minutes before the
  # entrypoint prints anything of its own.
  log "${provider}: running ${LANE_ENTRYPOINT} in ${LANE_IMAGE}"
  local run_started=$SECONDS
  ssh "${ssh_opts[@]}" "$host" \
    env REMOTE_DIR="$remote_dir" RUNTIME_IMAGE="$LANE_IMAGE" ENTRYPOINT="$LANE_ENTRYPOINT" VITEST_PROJECT="$SUITE" bash <<'EOF'
    set -e
    set -o pipefail
    docker run \
      --rm \
      --cpus=2 \
      -m 2g \
      -v "$HOME/$REMOTE_DIR":/tests \
      -e SNOWFLAKE_RUNNING_INSIDE_WIF_VM=true \
      -e VITEST_PROJECT="$VITEST_PROJECT" \
      "$RUNTIME_IMAGE" \
      bash "/tests/$ENTRYPOINT"
EOF
  local run_status=$?
  log "${provider}: container exited $run_status after $((SECONDS - run_started))s"
  return $run_status
}

run_tests_and_set_result() {
  local provider="$1" host="$2" snowflake_host="$3" rsa_key_path="$4"
  local snowflake_user="$5" impersonation_path="$6"

  local provider_started=$SECONDS
  run_wif_tests "$provider" "$host" "$snowflake_host" "$rsa_key_path" "$snowflake_user" "$impersonation_path"
  local status=$?

  if [[ $status -ne 0 ]]; then
    echo "$provider tests failed with exit status: $status"
    EXIT_STATUS=1
  else
    echo "$provider tests passed"
  fi
  log "${provider}: lane=${WIF_LANE}${SUITE:+ suite=$SUITE} finished in $((SECONDS - provider_started))s"

  log "${provider}: removing the remote working directory"
  cleanup_active_remote
}

get_branch() {
  local branch
  if [[ -n "${GIT_BRANCH}" ]]; then
    branch="${GIT_BRANCH}"
  else
    branch=$(git rev-parse --abbrev-ref HEAD)
  fi
  echo "${branch}"
}

setup_parameters() {
  local f
  for f in "$RSA_KEY_PATH_AWS_AZURE" "$RSA_KEY_PATH_GCP" "$PARAMETERS_FILE_PATH"; do
    if [[ ! -s "$f" ]]; then
      echo "ERROR: $f not found or empty" >&2
      echo "Run: ./scripts/decode_secrets.sh wif" >&2
      exit 1
    fi
  done
  eval $(jq -r '.wif | to_entries | map("export \(.key)=\(.value|tostring)")|.[]' "$PARAMETERS_FILE_PATH")
}

if [[ ! -f "$LANE_ARTIFACT" ]]; then
  echo "ERROR: $LANE_ARTIFACT not found. Run $LANE_BUILD_SCRIPT first." >&2
  exit 1
fi

BRANCH=$(get_branch)
export BRANCH
log "Lane ${WIF_LANE}${SUITE:+ suite=$SUITE} on branch ${BRANCH}"
log "Decrypting the WIF keys and parameters"
setup_parameters

# Run tests for all cloud providers
EXIT_STATUS=0
set +e  # Don't exit on first failure
run_tests_and_set_result "AZURE" "$HOST_AZURE" "$SNOWFLAKE_TEST_WIF_HOST_AZURE" "$RSA_KEY_PATH_AWS_AZURE" "$SNOWFLAKE_TEST_WIF_USERNAME_AZURE" "$SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH_AZURE"
run_tests_and_set_result "AWS"   "$HOST_AWS"   "$SNOWFLAKE_TEST_WIF_HOST_AWS"   "$RSA_KEY_PATH_AWS_AZURE" "$SNOWFLAKE_TEST_WIF_USERNAME_AWS"   "$SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH_AWS"
run_tests_and_set_result "GCP"   "$HOST_GCP"   "$SNOWFLAKE_TEST_WIF_HOST_GCP"   "$RSA_KEY_PATH_GCP"       "$SNOWFLAKE_TEST_WIF_USERNAME_GCP"   "$SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH_GCP"
set -e  # Re-enable exit on error
echo "Exit status: $EXIT_STATUS"
exit $EXIT_STATUS
