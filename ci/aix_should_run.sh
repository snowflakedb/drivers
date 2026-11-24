#!/usr/bin/env bash
# Exit 0 to run the AIX pipeline. Exit 1 to skip.
#
# Runs on:
#   - builds with AIX_FORCE=1
#   - branch builds of main
#   - pull requests whose branch name contains `aix` as a separate word
#     (for example `jdoe/aix-m1-byteorder`) or whose head commit message
#     contains `[aix]`
set -euo pipefail

lower() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

if [[ "${AIX_FORCE:-}" == "1" ]]; then
  echo "AIX: running (AIX_FORCE=1)"
  exit 0
fi

if [[ "${BUILDKITE_PULL_REQUEST:-false}" == "false" ]]; then
  if [[ "${BUILDKITE_BRANCH:-}" == "main" ]]; then
    echo "AIX: running on main"
    exit 0
  fi
  echo "AIX: skip (branch ${BUILDKITE_BRANCH:-unknown} is not main and is not a pull request; set AIX_FORCE=1 to run)"
  exit 1
fi

branch="$(lower "${BUILDKITE_BRANCH:-}")"
message="$(lower "${BUILDKITE_MESSAGE:-}")"

if [[ "$branch" =~ (^|[^a-z0-9])aix([^a-z0-9]|$) ]]; then
  echo "AIX: running on PR #${BUILDKITE_PULL_REQUEST} (branch name ${BUILDKITE_BRANCH})"
  exit 0
fi

if [[ "$message" == *"[aix]"* ]]; then
  echo "AIX: running on PR #${BUILDKITE_PULL_REQUEST} (commit message contains [aix])"
  exit 0
fi

echo "AIX: skip PR #${BUILDKITE_PULL_REQUEST} (branch ${BUILDKITE_BRANCH:-unknown} has no 'aix' word and the commit message has no [aix])"
exit 1
