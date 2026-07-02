#!/usr/bin/env bash
set -e

if ! command -v sf >/dev/null 2>&1; then
    # CI workers typically don't provision sf-cli. Skip gracefully.
    if [[ -n "${CI:-}" || -n "${GITHUB_ACTIONS:-}" || -n "${JENKINS_URL:-}" || -n "${BUILDKITE:-}" ]]; then
        echo "sf-cli not on PATH; skipping sf ai skills check in CI." >&2
        exit 0
    fi
    echo "sf-cli is required to validate .claude/skills/ but was not found on PATH." >&2
    echo "Install the sf CLI — ask your team lead for install instructions." >&2
    exit 1
fi

sf ai skills check --context=precommit --changed-only --severity=error
