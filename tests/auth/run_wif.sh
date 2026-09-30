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
#   ./tests/auth/run_wif.sh <sf_core|nodejs>
#
# The first argument selects which artifact is shipped and run. There is no
# default: a missing or unknown lane is an error. Extra arguments are an error.
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
  echo "Usage: $0 <sf_core|nodejs>" >&2
  exit 1
fi

WIF_LANE="${1:-}"
case "$WIF_LANE" in
  sf_core|nodejs)
    shift
    ;;
  *)
    echo "Usage: $0 <sf_core|nodejs>" >&2
    exit 1
    ;;
esac

if [[ $# -ne 0 ]]; then
  echo "ERROR: extra arguments are not accepted: $*" >&2
  exit 1
fi

case "$WIF_LANE" in
  sf_core)
    LANE_ARTIFACT="$ARTIFACT_DIR/sf_core_e2e"
    LANE_BUILD_SCRIPT="tests/auth/build_wif_sf_core_artifacts.sh"
    LANE_IMAGE="$RUNTIME_IMAGE"
    LANE_ENTRYPOINT="wif_run_sf_core_in_container.sh"
    ;;
  nodejs)
    LANE_ARTIFACT="$ARTIFACT_DIR/nodejs_wif.tar.gz"
    LANE_BUILD_SCRIPT="tests/auth/build_wif_nodejs_artifacts.sh"
    LANE_IMAGE="$NODEJS_RUNTIME_IMAGE"
    LANE_ENTRYPOINT="wif_run_nodejs_in_container.sh"
    ;;
  *)
    echo "ERROR: lane must be sf_core or nodejs (got '$WIF_LANE')" >&2
    exit 1
    ;;
esac

TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

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
  local ssh_opts=(-i "$rsa_key_path" -o IdentitiesOnly=yes -o StrictHostKeyChecking=no -p 443)
  local scp_opts=(-P 443 -i "$rsa_key_path" -o IdentitiesOnly=yes -o StrictHostKeyChecking=no)

  local params_file
  params_file="$(mktemp)"
  write_parameters_json "$params_file" "$provider" "$snowflake_host" "$snowflake_user" "$impersonation_path"

  echo "==================================================================="
  echo "WIF tests: ${provider} lane=${WIF_LANE} (host=${host}, remote_dir=${remote_dir})"
  echo "==================================================================="

  ssh "${ssh_opts[@]}" "$host" "mkdir -p \"$remote_dir\"" || {
    echo "ERROR: failed to create remote dir '$remote_dir' on $host" >&2
    rm -f "$params_file"
    return 1
  }

  local transfers=(
    "$params_file|parameters.json"
    "$LANE_ARTIFACT|$(basename "$LANE_ARTIFACT")"
    "$THIS_DIR/$LANE_ENTRYPOINT|$LANE_ENTRYPOINT"
  )
  local src dst spec
  for spec in "${transfers[@]}"; do
    src="${spec%%|*}"
    dst="${spec##*|}"
    scp "${scp_opts[@]}" "$src" "$host:$remote_dir/$dst" || {
      echo "ERROR: failed to scp '$src' to $host:$remote_dir/$dst" >&2
      rm -f "$params_file"
      return 1
    }
  done
  rm -f "$params_file"

  ssh "${ssh_opts[@]}" "$host" \
    env REMOTE_DIR="$remote_dir" RUNTIME_IMAGE="$LANE_IMAGE" ENTRYPOINT="$LANE_ENTRYPOINT" bash <<'EOF'
    set -e
    set -o pipefail
    docker run \
      --rm \
      --cpus=2 \
      -m 2g \
      -v "$HOME/$REMOTE_DIR":/tests \
      -e SNOWFLAKE_RUNNING_INSIDE_WIF_VM=true \
      "$RUNTIME_IMAGE" \
      bash "/tests/$ENTRYPOINT"
EOF
}

run_tests_and_set_result() {
  local provider="$1" host="$2" snowflake_host="$3" rsa_key_path="$4"
  local snowflake_user="$5" impersonation_path="$6"

  run_wif_tests "$provider" "$host" "$snowflake_host" "$rsa_key_path" "$snowflake_user" "$impersonation_path"
  local status=$?

  if [[ $status -ne 0 ]]; then
    echo "$provider tests failed with exit status: $status"
    EXIT_STATUS=1
  else
    echo "$provider tests passed"
  fi

  ssh -i "$rsa_key_path" -o IdentitiesOnly=yes -o StrictHostKeyChecking=no -p 443 "$host" \
    "rm -rf \"wif_${provider}_${WIF_LANE}_${TIMESTAMP}\"" || true
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
