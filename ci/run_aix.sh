#!/usr/bin/env bash
set -euo pipefail

if ! bash ci/aix_should_run.sh; then
  buildkite-agent annotate "AIX skipped: put 'aix' in the branch name or '[aix]' in the commit message, or set AIX_FORCE=1." --style info --context aix
  exit 0
fi

exec bash ci/run_aix_toolchain_check.sh
