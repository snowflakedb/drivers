#!/usr/bin/env bash
# Shared Copybara runner for the mirror-outbound and mirror-inbound Buildkite
# pipelines. Copybara is invoked through a purpose-built Docker image
# (ci/mirroring/Dockerfile.copybara) so the runtime is reproducible and the same
# script works identically on CI agents and developer laptops.
#
# Subcommands:
#   main <last_rev>
#     Outbound mirror of internal main -> public mirror main. Applies
#     --last-rev when <last_rev> is non-empty.
#
#   release <branch> <last_rev> <single_ref>
#     Outbound mirror of one release/* branch. --last-rev applies only when
#     <single_ref> is non-empty (same semantics as the legacy workflow:
#     last_rev is a SHA on a specific branch's history and is ignored when
#     looping over every release/*).
#
#   import <pr_number>
#     Inbound import of a labeled mirror PR back to the internal repo.
#
# Required env:
#   COPYBARA_RELEASE        Copybara release tag (e.g. v20260504). Used both
#                           as the default build arg for the image and as
#                           part of the local image tag.
#   DRIVER_MIRROR_TOKEN     Push token for snowflake-eng/universal-driver
#                           (outbound: git fetch origin; inbound: inlined
#                           into --git-destination-url).
#   DRIVER_MIRROR_TOKEN_SNOWFLAKEDB
#                           Push token for snowflakedb/universal-driver
#                           (outbound: inlined into --git-destination-url;
#                           inbound: git fetch origin + PR metadata API).
#
# Optional env:
#   COPYBARA_IMAGE          Pre-built image reference to use instead of
#                           building locally. Set this on CI agents once a
#                           registry-hosted image is available.
#   IMPORT_AS_DRAFT         When "true", open the imported internal PR as a
#                           draft (passed to prepare_import_paths.py).

set -euo pipefail

: "${COPYBARA_RELEASE:?COPYBARA_RELEASE must be set}"
: "${DRIVER_MIRROR_TOKEN:?DRIVER_MIRROR_TOKEN must be set}"
: "${DRIVER_MIRROR_TOKEN_SNOWFLAKEDB:?DRIVER_MIRROR_TOKEN_SNOWFLAKEDB must be set}"

SUBCOMMAND="${1:-}"
if [ -z "${SUBCOMMAND}" ]; then
  echo "usage: $0 {main|release|import} ..." >&2
  exit 2
fi
shift

# Resolve repo root from this script's location so it works the same when
# invoked from CI (cwd == repo root) or from anywhere on a developer laptop.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

: "${COPYBARA_IMAGE:=universal-driver-copybara:${COPYBARA_RELEASE}}"

build_image_if_missing() {
  if docker image inspect "${COPYBARA_IMAGE}" >/dev/null 2>&1; then
    return 0
  fi
  echo "Building ${COPYBARA_IMAGE} from ci/mirroring/Dockerfile.copybara"
  docker build \
    --build-arg "COPYBARA_RELEASE=${COPYBARA_RELEASE}" \
    -t "${COPYBARA_IMAGE}" \
    -f "${REPO_ROOT}/ci/mirroring/Dockerfile.copybara" \
    "${REPO_ROOT}/ci/mirroring"
}

# Per-run scratch HOME keeps the credentials file outside the repo worktree
# and outside $HOME, and disappears when the script exits regardless of
# exit path.
TMP_HOME=""
cleanup() {
  if [ -n "${TMP_HOME}" ] && [ -d "${TMP_HOME}" ]; then
    rm -rf "${TMP_HOME}"
  fi
}
trap cleanup EXIT

prepare_home() {
  # $1 is the token to place in the credentials file. The *other* token is
  # inlined into --git-destination-url, which takes precedence over this
  # file for the destination push.
  local token="$1"
  TMP_HOME="$(mktemp -d -t copybara-home-XXXXXX)"
  umask 077
  printf 'https://x-access-token:%s@github.com\n' "${token}" > "${TMP_HOME}/.git-credentials"
  cat > "${TMP_HOME}/.gitconfig" <<'GITCONFIG'
[user]
  name = Mirror Bot
  email = mirror-bot@snowflake.com
[credential]
  helper = store
[safe]
  directory = /workdir
GITCONFIG
}

# Run the Copybara image. Arguments are forwarded verbatim to the
# `java -jar copybara_deploy.jar` ENTRYPOINT. The repo is mounted at
# /workdir (Copybara's working dir) and the scratch HOME at /root so the
# credentials file is picked up by git operations inside the container.
#
# Copybara exits 4 (NO_OP) when there are no new changes to migrate — the
# steady state whenever the source ref hasn't advanced since the last run.
# That is a success, not a failure, so it is normalized to exit 0 here;
# every other non-zero code propagates.
run_copybara() {
  local rc=0
  docker run --rm \
    -v "${REPO_ROOT}:/workdir" \
    -v "${TMP_HOME}:/root" \
    -w /workdir \
    "${COPYBARA_IMAGE}" \
    "$@" || rc=$?
  if [ "${rc}" -eq 4 ]; then
    echo "Copybara: no new changes (NO_OP, exit 4) — treating as success."
    return 0
  fi
  return "${rc}"
}

build_image_if_missing

case "${SUBCOMMAND}" in
  main)
    LAST_REV="${1:-}"
    prepare_home "${DRIVER_MIRROR_TOKEN}"
    extra_args=()
    if [ -n "${LAST_REV}" ]; then
      echo "Anchoring replay at --last-rev=${LAST_REV}"
      extra_args+=(--last-rev "${LAST_REV}")
    fi
    run_copybara migrate ci/mirroring/copy.bara.sky mirror \
      "${extra_args[@]}"
    ;;

  release)
    BRANCH="${1:-}"
    LAST_REV="${2:-}"
    SINGLE_REF="${3:-}"
    if [ -z "${BRANCH}" ]; then
      echo "usage: $0 release <branch> [last_rev] [single_ref]" >&2
      exit 2
    fi
    prepare_home "${DRIVER_MIRROR_TOKEN}"
    extra_args=()
    if [ -n "${LAST_REV}" ] && [ -n "${SINGLE_REF}" ]; then
      echo "Anchoring replay at --last-rev=${LAST_REV} for ${SINGLE_REF}"
      extra_args+=(--last-rev "${LAST_REV}")
    elif [ -n "${LAST_REV}" ]; then
      echo "::warning:: last_rev=${LAST_REV} ignored — only applied when a single branch is dispatched"
    fi
    # "${BRANCH}" is the positional source_ref (migrate <config> <workflow>
    # [source_ref]); it overrides the `ref = "main"` baked into copy.bara.sky
    # so the release branch — not main — is mirrored. Copybara has no `--ref`
    # flag, so the ref MUST be positional here.
    #
    # NOTE: a brand-new release branch must already exist on the mirror, or
    # Copybara aborts ("GitOrigin-RevId could not be found"). The GitHub
    # Actions workflow (.github/workflows/mirror.yml) seeds it automatically
    # from the mirrored fork-point commit; when running this script by hand
    # against a not-yet-mirrored branch, seed it first (or pass --init-history
    # via single_ref/last_rev for a full-history replay).
    run_copybara migrate ci/mirroring/copy.bara.sky mirror "${BRANCH}" \
      --git-destination-url="https://x-access-token:${DRIVER_MIRROR_TOKEN_SNOWFLAKEDB}@github.com/snowflakedb/universal-driver.git" \
      --git-destination-push="${BRANCH}" \
      --force \
      "${extra_args[@]}"
    ;;

  import)
    PR_NUMBER="${1:-}"
    if [ -z "${PR_NUMBER}" ]; then
      echo "usage: $0 import <pr_number>" >&2
      exit 2
    fi
    # Inbound: origin is the mirror, so the credentials file holds the
    # snowflakedb-side mirror token. Destination push into the internal repo
    # uses DRIVER_MIRROR_TOKEN (snowflake-eng access) inlined in the URL below.
    prepare_home "${DRIVER_MIRROR_TOKEN_SNOWFLAKEDB}"
    import_draft_args=()
    if [ "${IMPORT_AS_DRAFT:-false}" = "true" ]; then
      import_draft_args=(--draft)
    fi
    python3 "${REPO_ROOT}/scripts/mirror/prepare_import_paths.py" "${PR_NUMBER}" "${import_draft_args[@]}"
    # --nogit-destination-rebase: the mirror is a strict subset of the
    # internal repo (no copy.bara.sky, _internal/, adr/, .github/, …).
    # Rebasing the mirror PR branch onto internal main fails with
    # modify/delete conflicts on those internal-only files. Skipping the
    # rebase applies the PR diff as-is on top of internal main — safe
    # because the PR only touches files inside MIRRORED_PATHS.
    #
    # Copybara reads GITHUB_TOKEN to call the mirror's API for PR metadata;
    # forward DRIVER_MIRROR_TOKEN_SNOWFLAKEDB into the container under that name.
    #
    # This path uses its own `docker run` (it needs an extra -e GITHUB_TOKEN)
    # rather than run_copybara(), so it repeats the exit-4 (NO_OP) handling:
    # "nothing to import" is success, not failure.
    rc=0
    docker run --rm \
      -v "${REPO_ROOT}:/workdir" \
      -v "${TMP_HOME}:/root" \
      -w /workdir \
      -e "GITHUB_TOKEN=${DRIVER_MIRROR_TOKEN_SNOWFLAKEDB}" \
      "${COPYBARA_IMAGE}" \
      migrate ci/mirroring/copy.bara.sky import "${PR_NUMBER}" \
        --git-destination-url="https://x-access-token:${DRIVER_MIRROR_TOKEN}@github.com/snowflake-eng/universal-driver.git" \
        --nogit-destination-rebase \
        --force || rc=$?
    if [ "${rc}" -eq 4 ]; then
      echo "Copybara: nothing to import (NO_OP, exit 4) — treating as success."
      rc=0
    fi
    exit "${rc}"
    ;;

  *)
    echo "unknown subcommand: ${SUBCOMMAND}" >&2
    echo "usage: $0 {main|release|import} ..." >&2
    exit 2
    ;;
esac
